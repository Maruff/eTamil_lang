// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Which functions a database may be given to call.
//!
//! A function that SQLite (and, in time, PostgreSQL) calls once per row runs
//! inside the database's own process, on the database's behalf. So it may only
//! compute: it may not read a file, open a socket or another database, read the
//! environment, or run a program. A function that can is refused when it is
//! registered, before anything is installed, not when a row happens to reach it.
//!
//! The check is on compiled code, because that is all there is at run time. It
//! walks the instructions of the function and of every function it can reach,
//! and refuses any that reaches outside the database. A builtin is allowed only
//! if it is on the list `build.rs` writes (`PURE_BUILTIN_NAMES`); the default for
//! a builtin nobody has read is "no".
//!
//! What it relies on:
//!
//!   * A function's body is emitted inline and jumped over, so it is the
//!     instructions from `FunctionInfo::start` to the target of the `Jump` just
//!     before it. A function defined inside another lies inside that range, and
//!     is scanned with it.
//!   * `Call(name, n)` runs a user function of that name if there is one, and
//!     otherwise a builtin or a shape's constructor. So a name that is a user
//!     function is followed, and any other name must be a pure builtin.
//!   * A function *value* is made by `MakeFunction`, and each one made inside a
//!     scanned body is checked where it is made, so `CallValue` is safe. The
//!     values a caller hands over are plain data, with one exception, which
//!     `check_value` refuses: a function value that has captured another function
//!     value, which could be a builtin that reaches outside.
//!   * Methods are refused for now. `CallMethod` finds its target by the
//!     receiver's shape at run time, which this walk cannot see.

use std::collections::HashSet;

use crate::vm::value::FunctionValue;
use crate::vm::{Bytecode, Instruction, Value};

mod pure_names {
    include!(concat!(env!("OUT_DIR"), "/pure_builtin_names.rs"));
}

/// Is this builtin, under any of its spellings, one a database function may call?
pub fn is_pure_builtin(name: &str) -> bool {
    pure_names::PURE_BUILTIN_NAMES.binary_search(&name).is_ok()
}

fn reaches_outside(what: &str) -> String {
    format!("a database function may not use {what}: it reaches outside the database")
}

/// What an instruction reaches, if it reaches outside the database.
fn outside(instruction: &Instruction) -> Option<&'static str> {
    Some(match instruction {
        Instruction::FileOpen(_)
        | Instruction::FileClose
        | Instruction::FileWrite
        | Instruction::FileRead
        | Instruction::ReadCSV
        | Instruction::WriteCSV => "a file",
        Instruction::DBConnect(..)
        | Instruction::DBDisconnect(_)
        | Instruction::DBQuery(_)
        | Instruction::DBExecute(_) => "another database",
        Instruction::DefineRoute(..)
        | Instruction::SendResponse
        | Instruction::SendJSON
        | Instruction::StartServer(..) => "the web",
        Instruction::Input => "keyboard input",
        _ => return None,
    })
}

/// Check that `function`, a function of `program`, and everything it can reach,
/// only computes.
pub fn check_function(program: &Bytecode, function: &str) -> Result<(), String> {
    let mut pending = vec![function.to_string()];
    let mut seen: HashSet<String> = HashSet::new();

    while let Some(name) = pending.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        let info = program
            .functions
            .get(&name)
            .ok_or_else(|| format!("there is no function called {name}"))?;
        let (start, end) = body(program, &name, info.start)?;

        for instruction in &program.instructions[start..end] {
            if let Some(what) = outside(instruction) {
                return Err(reaches_outside(what));
            }
            match instruction {
                Instruction::Call(callee, _) | Instruction::MakeFunction(callee, _) => {
                    follow(program, callee, &mut pending)?;
                }
                Instruction::CallMethod(method, _) => {
                    return Err(format!(
                        "a database function may not call a method ({method}): which function it runs is only known at run time"
                    ));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// A callee is a user function (followed), a shape's constructor (harmless), or a
/// builtin, which must be on the pure list.
fn follow(program: &Bytecode, callee: &str, pending: &mut Vec<String>) -> Result<(), String> {
    if program.functions.contains_key(callee) {
        pending.push(callee.to_string());
        Ok(())
    } else if program.shapes.contains_key(callee) || is_pure_builtin(callee) {
        Ok(())
    } else if crate::vm::is_builtin(callee) {
        Err(reaches_outside(&format!("`{callee}`")))
    } else {
        Err(format!("there is no function called {callee}"))
    }
}

/// Check a function value that is being handed over to be called by a database:
/// the function it names, and what it carries with it.
pub fn check_value(program: &Bytecode, function: &FunctionValue) -> Result<(), String> {
    if function.captured.iter().any(holds_function) {
        return Err(
            "a database function may not carry another function with it: it could be one that reaches outside the database".to_string(),
        );
    }
    let mut pending = Vec::new();
    follow(program, &function.name, &mut pending)?;
    for name in pending {
        check_function(program, &name)?;
    }
    Ok(())
}

/// Does this value contain a function value, at any depth?
fn holds_function(value: &Value) -> bool {
    match value {
        Value::Function(_) => true,
        Value::Array(items) => items.iter().any(holds_function),
        Value::Map(fields) => fields.values().any(holds_function),
        Value::Ok(inner) | Value::Err(inner) => holds_function(inner),
        _ => false,
    }
}

/// The instructions of a function's body: from `start` to where the jump over it lands.
fn body(program: &Bytecode, name: &str, start: usize) -> Result<(usize, usize), String> {
    let end = match start
        .checked_sub(1)
        .and_then(|i| program.instructions.get(i))
    {
        Some(Instruction::Jump(end)) if *end >= start && *end <= program.instructions.len() => *end,
        _ => {
            return Err(format!(
                "the body of {name} could not be found in the compiled program"
            ));
        }
    };
    Ok((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A program with one function, `f`, whose body is exactly `body`.
    fn with_body(body: Vec<Instruction>) -> Bytecode {
        let mut program = Bytecode::new();
        program.push(Instruction::Jump(0));
        let start = program.len();
        for instruction in body {
            program.push(instruction);
        }
        program.push(Instruction::Push(Value::Null));
        program.push(Instruction::Return);
        let end = program.len();
        program.instructions[0] = Instruction::Jump(end);
        program.functions.insert(
            "f".to_string(),
            crate::vm::bytecode::FunctionInfo {
                start,
                params: Vec::new(),
                captures: Vec::new(),
            },
        );
        program
    }

    #[test]
    fn a_body_that_only_computes_is_accepted() {
        let program = with_body(vec![
            Instruction::Push(Value::Null),
            Instruction::Pop,
            Instruction::Add,
            Instruction::Call("_length".to_string(), 1),
        ]);
        assert_eq!(check_function(&program, "f"), Ok(()));
    }

    #[test]
    fn every_instruction_that_reaches_outside_is_refused() {
        let cases: Vec<(Instruction, &str)> = vec![
            (Instruction::FileOpen("read".into()), "a file"),
            (Instruction::FileClose, "a file"),
            (Instruction::FileWrite, "a file"),
            (Instruction::FileRead, "a file"),
            (Instruction::ReadCSV, "a file"),
            (Instruction::WriteCSV, "a file"),
            (
                Instruction::DBConnect("SQLite".into(), "x".into()),
                "another database",
            ),
            (
                Instruction::DBDisconnect("SQLite".into()),
                "another database",
            ),
            (Instruction::DBQuery(None), "another database"),
            (Instruction::DBExecute(None), "another database"),
            (
                Instruction::DefineRoute("GET".into(), "/".into()),
                "the web",
            ),
            (Instruction::SendResponse, "the web"),
            (Instruction::SendJSON, "the web"),
            (Instruction::StartServer("0.0.0.0".into(), 80), "the web"),
            (Instruction::Input, "keyboard input"),
        ];
        for (instruction, what) in cases {
            let program = with_body(vec![instruction.clone()]);
            assert_eq!(
                check_function(&program, "f"),
                Err(reaches_outside(what)),
                "{instruction:?}"
            );
        }
    }

    #[test]
    fn a_builtin_that_reaches_outside_is_refused_under_every_spelling() {
        // Each group of the list in build.rs, with all three spellings of one member.
        let spellings = [
            ["கோப்பு_உள்ளதா", "kOppu_uLLaqA", "_fileExists"],
            ["வலை_பெறு", "valY_peRu", "_httpGet"],
            ["சூழல்", "cUzal", "_env"],
            ["கட்டளை_ஓட்டு", "kattaLY_Ottu", "_run"],
            ["வெளியேறு", "veLiyERu", "_exit"],
            ["வன்_காத்திரு", "vaZ_kAqqiru", "_sleepMs"],
        ];
        for group in spellings {
            for spelling in group {
                assert!(
                    crate::vm::is_builtin(spelling),
                    "{spelling} should be a builtin; has it been renamed?"
                );
                let program = with_body(vec![Instruction::Call(spelling.to_string(), 0)]);
                assert_eq!(
                    check_function(&program, "f"),
                    Err(reaches_outside(&format!("`{spelling}`"))),
                    "{spelling}"
                );
            }
        }
    }

    #[test]
    fn a_builtin_that_computes_is_accepted_under_every_spelling() {
        for spelling in [
            "நீளம்",
            "nILam",
            "_length",
            "_round",
            "_toString",
            "_upper",
            "_jsonParse",
        ] {
            assert!(is_pure_builtin(spelling), "{spelling}");
        }
        assert!(!is_pure_builtin("_env"));
        assert!(!is_pure_builtin("_httpGet"));
        assert!(!is_pure_builtin("not a builtin"));
    }

    #[test]
    fn a_function_value_of_a_forbidden_builtin_is_refused_where_it_is_made() {
        let program = with_body(vec![Instruction::MakeFunction("_env".to_string(), vec![])]);
        assert!(check_function(&program, "f").is_err());
    }

    #[test]
    fn an_unknown_name_is_refused() {
        let program = with_body(vec![Instruction::Call("nothing_here".to_string(), 0)]);
        assert_eq!(
            check_function(&program, "f"),
            Err("there is no function called nothing_here".to_string())
        );
    }

    #[test]
    fn a_method_call_is_refused() {
        let program = with_body(vec![Instruction::CallMethod("run".to_string(), 0)]);
        assert!(
            check_function(&program, "f")
                .unwrap_err()
                .contains("method")
        );
    }

    #[test]
    fn a_function_that_is_not_in_the_program_is_an_error() {
        let program = with_body(vec![]);
        assert!(check_function(&program, "g").is_err());
    }

    #[test]
    fn a_function_that_calls_itself_is_checked_once_and_ends() {
        let program = with_body(vec![Instruction::Call("f".to_string(), 0)]);
        assert_eq!(check_function(&program, "f"), Ok(()));
    }

    #[test]
    fn a_value_that_carries_a_function_is_refused() {
        let program = with_body(vec![]);
        let carried = Value::Function(Box::new(FunctionValue {
            name: "_env".to_string(),
            captured: vec![],
        }));
        for captured in [
            carried.clone(),
            Value::Array(vec![carried.clone()]),
            Value::Ok(Box::new(carried.clone())),
        ] {
            let value = FunctionValue {
                name: "f".to_string(),
                captured: vec![captured],
            };
            assert!(check_value(&program, &value).is_err());
        }
    }

    #[test]
    fn a_value_for_a_function_that_computes_is_accepted() {
        let program = with_body(vec![]);
        let value = FunctionValue {
            name: "f".to_string(),
            captured: vec![Value::Null],
        };
        assert_eq!(check_value(&program, &value), Ok(()));
    }

    #[test]
    fn a_value_for_a_forbidden_builtin_is_refused() {
        let program = with_body(vec![]);
        let value = FunctionValue {
            name: "_env".to_string(),
            captured: vec![],
        };
        assert!(check_value(&program, &value).is_err());
    }

    #[test]
    fn a_body_that_cannot_be_found_is_an_error() {
        let mut program = with_body(vec![]);
        program.instructions[0] = Instruction::Nop; // the jump over the body is gone
        assert!(
            check_function(&program, "f")
                .unwrap_err()
                .contains("could not be found")
        );
    }
}
