// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! A program's function, made into one a database can call once per row.
//!
//! The function lives in the program's bytecode, and the VM that registered it is
//! busy, or gone, by the time a query calls it. So registration takes a copy of the
//! program with a few instructions added at the end:
//!
//! ```text
//! LoadVar  #function            the function value
//! LoadVar  #arg0 ... #argN      the row's arguments
//! CallValue N
//! StoreVar #result
//! Halt
//! ```
//!
//! and each call builds a fresh VM, binds the arguments to those names, and runs from
//! the first of them, so no call sees another's variables. The `#` names are not
//! writable in source, so a program's own names cannot collide with them. This is how
//! the HTTP server's workers run a handler too: a fresh VM over shared bytecode.
//!
//! What the function may do is decided before this is built, by `crate::purity`.

use std::sync::Arc;

use super::RowFunction;
use crate::vm::host;
use crate::vm::value::FunctionValue;
use crate::vm::{Bytecode, Instruction, VM, Value};

/// The most instructions one call may run. A query calls the function once per row,
/// and a function that never ends would hold the database for good.
const STEP_LIMIT: u64 = 100_000;

const FUNCTION: &str = "#function";
const RESULT: &str = "#result";

fn argument(index: usize) -> String {
    format!("#arg{index}")
}

/// Make `function`, taking `arity` arguments, into a function a database can call.
pub fn row_function(program: &Bytecode, function: FunctionValue, arity: usize) -> RowFunction {
    let mut shared = program.clone();
    let start = shared.len();
    shared.push(Instruction::LoadVar(FUNCTION.to_string()));
    for index in 0..arity {
        shared.push(Instruction::LoadVar(argument(index)));
    }
    shared.push(Instruction::CallValue(arity));
    shared.push(Instruction::StoreVar(RESULT.to_string()));
    shared.push(Instruction::Halt);

    let shared = Arc::new(shared);
    let function = Value::Function(Box::new(function));

    Box::new(move |args: &[Value]| {
        if args.len() != arity {
            return Err(format!(
                "the function takes {arity} argument(s) and was given {}",
                args.len()
            ));
        }
        let mut vm = VM::new();
        vm.variables.insert(FUNCTION.to_string(), function.clone());
        for (index, value) in args.iter().enumerate() {
            vm.variables.insert(argument(index), value.clone());
        }

        // Anything the function prints has nowhere to go inside a query. Capture it
        // and drop it, so it neither reaches the terminal nor leaks into the next call.
        host::begin_capture();
        let outcome = vm.execute_shared(shared.clone(), start, Some(STEP_LIMIT));
        host::end_capture();
        outcome?;

        Ok(vm.variables.remove(RESULT).unwrap_or(Value::Null))
    })
}
