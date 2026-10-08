// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! The VM keeps the program it runs.
//!
//! A function value is a name and what it captured; its body lives in the
//! program's bytecode. Calling one from outside the instruction loop, as a
//! SQLite callback will, needs the bytecode to still be there after `execute`
//! has started, so these tests hold the VM to keeping it.

use etamil_compiler::lexer;
use etamil_compiler::parser::Parser;
use etamil_compiler::vm::{BytecodeCompiler, VM};

fn compile(source: &str) -> etamil_compiler::vm::Bytecode {
    let tokens = lexer::tokenize(source).expect("the source should tokenize");
    let ast = Parser::new(tokens.iter())
        .parse()
        .expect("the source should parse");
    BytecodeCompiler::compile_statements(ast)
}

#[test]
fn a_new_vm_has_no_program() {
    assert!(VM::new().program().is_none());
}

#[test]
fn the_program_is_kept_after_it_runs_with_its_functions() {
    let mut vm = VM::new();
    vm.execute(compile("செயல் moqqam(a) {\n  திரும்பு a * 2;\n}\n"))
        .unwrap();

    let program = vm.program().expect("the VM should keep the program it ran");
    assert!(program.functions.contains_key("moqqam"));
}

#[test]
fn running_a_second_program_replaces_the_first() {
    let mut vm = VM::new();
    vm.execute(compile("செயல் moqqam(a) {\n  திரும்பு a;\n}\n"))
        .unwrap();
    vm.instruction_pointer = 0;
    vm.execute(compile("செயல் vari(a) {\n  திரும்பு a;\n}\n"))
        .unwrap();

    let program = vm.program().unwrap();
    assert!(program.functions.contains_key("vari"));
    assert!(!program.functions.contains_key("moqqam"));
}

#[test]
fn keeping_the_program_shares_it_rather_than_copying() {
    let mut vm = VM::new();
    vm.execute(compile("x = 1;")).unwrap();

    // Two handles to one program: the VM's and the caller's.
    let first = vm.program().unwrap();
    let second = vm.program().unwrap();
    assert!(std::sync::Arc::ptr_eq(&first, &second));
}
