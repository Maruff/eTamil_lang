// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! A database function: a body, its parameter names, and a call.
//!
//! `CREATE FUNCTION gst_total(amount numeric, rate numeric) RETURNS numeric LANGUAGE pletamil AS $$
//! திரும்பு amount * (1 + rate / 100); $$` gives the body `திரும்பு amount * ...` and the
//! parameter names `amount` and `rate`. This turns that into something callable, with the same
//! rules as everything else here: only a program that computes may run.
//!
//! Pure Rust with no PostgreSQL, so it is tested everywhere. Turning a PostgreSQL value into a
//! `Value` and back is the extension's job (`../../src/lib.rs`).

use etamil_compiler::check;
use etamil_compiler::lexer;
use etamil_compiler::parser::{Parser, Stmt};
use etamil_compiler::vm::{host, Bytecode, BytecodeCompiler, Value, VM};

use crate::{require_pure, STEP_LIMIT};

/// The name the body is given. Nothing outside this file sees it.
const BODY: &str = "pl_body";

/// A body that has been parsed, checked, found pure and compiled.
pub struct Function {
    bytecode: Bytecode,
    arity: usize,
}

impl Function {
    pub fn arity(&self) -> usize {
        self.arity
    }
}

/// Is `name` something the eTamil parser will take as a parameter? SQL allows far more in an
/// argument name (spaces, quotes, a leading digit), and a name pasted into source unchecked would
/// let one close the parameter list and write code.
fn valid_name(name: &str) -> bool {
    let tamil = |c: char| ('\u{0B80}'..='\u{0BFF}').contains(&c);
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' || tamil(first) => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || tamil(c))
}

fn parse(source: &str) -> Result<Vec<Stmt>, String> {
    let tokens = lexer::tokenize(source)
        .map_err(|errors| errors.first().map(|e| e.to_string()).unwrap_or_else(|| "lexical error".into()))?;
    Parser::new(tokens.iter()).parse().map_err(|e| e.to_string())
}

/// Prepare `body` as a function of `params`.
///
/// The body is wrapped as `செயல் pl_body(a, b) { <body> }` with the header on the body's own first
/// line, so a position in an error message is a position in the body the author wrote, not one
/// line further down.
pub fn prepare(params: &[String], body: &str) -> Result<Function, String> {
    for (index, name) in params.iter().enumerate() {
        if !valid_name(name) {
            return Err(format!("argument {} is called {name:?}, which is not a name eTamil accepts", index + 1));
        }
        if params[..index].contains(name) {
            return Err(format!("two arguments are called {name}"));
        }
    }

    let function = parse(&format!("செயல் {BODY}({}) {{ {body}\n}}\n", params.join(", ")))?;

    // Exactly the one definition. A `}` in the body would end the wrapper early, and what
    // followed would run as top-level statements on every call. It could do no more than the
    // body can (it passes the same allowlist), but "the body is one function" is the contract,
    // and an unbalanced brace is almost certainly a mistake worth reporting as one.
    match function.as_slice() {
        [Stmt::FunctionDef { name, .. }] if name == BODY => {}
        _ => return Err("the body's braces do not balance: a } closes the function early".into()),
    }
    require_pure(&function)?;
    if let Err(errors) = check::check(&function) {
        return Err(errors.first().map(|e| e.to_string()).unwrap_or_else(|| "type error".into()));
    }

    // A body may declare more than one function of its own; only one definition is the body.
    // Anything the author wrote at the top level of the *body* is inside the braces, so it
    // cannot become a top-level statement of the program.
    let arguments: Vec<String> = (0..params.len()).map(|i| format!("pl_arg{i}")).collect();
    let mut program = function;
    program.extend(parse(&format!("pl_result = {BODY}({});", arguments.join(", ")))?);
    Ok(Function { bytecode: BytecodeCompiler::compile_statements(program), arity: params.len() })
}

/// Call a prepared function in a fresh VM, so no call sees another's variables.
pub fn call(function: &Function, args: Vec<Value>) -> Result<Value, String> {
    if args.len() != function.arity {
        return Err(format!("the function takes {} argument(s) and was given {}", function.arity, args.len()));
    }
    let mut vm = VM::new();
    for (index, value) in args.into_iter().enumerate() {
        vm.variables.insert(format!("pl_arg{index}"), value);
    }

    // Anything the body prints has nowhere to go inside a query; capture it and drop it.
    host::begin_capture();
    let outcome = vm.execute_limited(function.bytecode.clone(), STEP_LIMIT);
    host::end_capture();
    outcome?;

    Ok(vm.variables.remove("pl_result").unwrap_or(Value::Null))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use etamil_compiler::vm::Value;
    use rust_decimal::Decimal;

    use super::*;

    fn number(text: &str) -> Value {
        Value::Number(Decimal::from_str(text).unwrap())
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn run(params: &[&str], body: &str, args: Vec<Value>) -> Result<Value, String> {
        call(&prepare(&names(params), body)?, args)
    }

    #[test]
    fn a_body_with_arguments_returns_its_value_exactly() {
        let result = run(&["amount", "rate"], "திரும்பு amount * (1 + rate / 100);", vec![number("250.50"), number("18")]);
        assert_eq!(result, Ok(number("295.5900")));
    }

    #[test]
    fn arguments_may_have_tamil_names_and_text_survives() {
        let result = run(&["பெயர்", "ஊர்"], "திரும்பு பெயர் & \" - \" & ஊர்;", vec![Value::String("மாறன்".into()), Value::String("சென்னை".into())]);
        assert_eq!(result, Ok(Value::String("மாறன் - சென்னை".into())));
    }

    #[test]
    fn a_body_may_define_and_use_helper_functions() {
        let body = "செயல் double(x) {\n  திரும்பு x * 2;\n}\nதிரும்பு double(n) + 1;";
        assert_eq!(run(&["n"], body, vec![number("5")]), Ok(number("11")));
    }

    #[test]
    fn a_body_with_no_return_gives_null() {
        assert_eq!(run(&[], "x = 1;", vec![]), Ok(Value::Null));
    }

    #[test]
    fn a_null_argument_arrives_as_null() {
        let body = "திரும்பு _typeof(a);";
        let null = run(&["a"], body, vec![Value::Null]).unwrap();
        let text = run(&["a"], body, vec![Value::String("x".into())]).unwrap();
        assert_ne!(null, text);
    }

    #[test]
    fn calls_do_not_share_variables() {
        let f = prepare(&names(&["a"]), "திரும்பு a;").unwrap();
        assert_eq!(call(&f, vec![number("1")]), Ok(number("1")));
        assert_eq!(call(&f, vec![number("2")]), Ok(number("2")));
    }

    #[test]
    fn a_wrong_number_of_arguments_is_an_error() {
        let f = prepare(&names(&["a", "b"]), "திரும்பு a;").unwrap();
        assert!(call(&f, vec![number("1")]).unwrap_err().contains("2 argument"));
    }

    // --- names ---

    #[test]
    fn an_argument_name_cannot_smuggle_code() {
        for bad in ["a) { திரும்பு 1; } செயல் x(b", "a b", "1a", "", "a-b", "a\"b"] {
            let error = prepare(&names(&[bad]), "திரும்பு 1;").err().expect(bad);
            assert!(error.contains("not a name"), "{bad:?}: {error}");
        }
    }

    #[test]
    fn two_arguments_cannot_share_a_name() {
        assert!(prepare(&names(&["a", "a"]), "திரும்பு a;").err().unwrap().contains("two arguments"));
    }

    // --- refusals and limits ---

    #[test]
    fn a_body_that_reaches_outside_is_refused_when_prepared() {
        let error = prepare(&names(&[]), "திரும்பு _env(\"HOME\");").err().unwrap();
        assert_eq!(error, "a database function may not use `_env`: it reaches outside the database");
        assert!(prepare(&names(&[]), "இறக்கு \"nUlakam/paNam/paNam.qmz\";").is_err());
    }

    #[test]
    fn an_endless_body_stops_at_the_step_limit() {
        let result = run(&[], "i = 0;\n(i >= 0) சுற்று {\n  i = i + 1;\n}\nதிரும்பு i;", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn a_runtime_error_is_an_error() {
        assert!(run(&["a"], "திரும்பு a / 0;", vec![number("1")]).is_err());
    }

    #[test]
    fn a_syntax_error_points_into_the_body_not_past_it() {
        // The error is on line 2 of the body. The header shares line 1, so it says 2, not 3.
        let error = prepare(&names(&[]), "x = 1;\ny = ;").err().unwrap();
        assert!(error.contains(" 2"), "{error}");
        assert!(!error.contains(" 3"), "{error}");
    }

    #[test]
    fn a_body_cannot_close_the_function_and_run_top_level_statements() {
        // Harmless code, so the allowlist alone would let it through: only the check that the
        // body is one definition stops it.
        let error = prepare(&names(&[]), "திரும்பு 1; } அச்சு(\"hi\"); செயல் g() {").err().unwrap();
        assert!(error.contains("braces do not balance"), "{error}");
        // And with a forbidden call in it, still refused.
        assert!(prepare(&names(&[]), "திரும்பு 1; } அச்சு(_env(\"HOME\")); செயல் g() {").is_err());
    }
}
