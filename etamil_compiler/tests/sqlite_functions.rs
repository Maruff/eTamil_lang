// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! `தளம்_செயல்_பதிவு`: a program's function, called by SQLite once per row.
//!
//! These run whole eTamil programs against a real SQLite database, so they cover
//! what the unit tests of the pieces cannot: the builtin's arguments, the check
//! that refuses a function that could reach outside the database, the running of
//! the function in a fresh VM, and what a pooled connection forgets.

use std::str::FromStr;

use etamil_compiler::lexer;
use etamil_compiler::parser::Parser;
use etamil_compiler::vm::{BytecodeCompiler, VM, Value, host};
use rust_decimal::Decimal;

fn run(source: &str) -> Result<VM, String> {
    let tokens = lexer::tokenize(source).map_err(|errors| {
        errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    let ast = Parser::new(tokens.iter())
        .parse()
        .map_err(|error| error.to_string())?;
    etamil_compiler::check::check(&ast).map_err(|errors| {
        errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    let mut vm = VM::new();
    vm.execute(BytecodeCompiler::compile_statements(ast))?;
    Ok(vm)
}

fn number(text: &str) -> Value {
    Value::Number(Decimal::from_str(text).unwrap())
}

/// A program that has a function, an open in-memory database, and three invoices.
fn with_invoices(rest: &str) -> String {
    format!(
        r#"செயல் மொத்தம்_வரி(தொகை, விகிதம்) {{
               திரும்பு தொகை * (1 + விகிதம் / 100);
           }}
           தளம்_இணை சீகுலைட், ":memory:";
           தளம்_செய் "CREATE TABLE paRRuccIttu (eN INTEGER, qokY TEXT)", [];
           தளம்_செய் "INSERT INTO paRRuccIttu VALUES (1, '1000')", [];
           தளம்_செய் "INSERT INTO paRRuccIttu VALUES (2, '250.50')", [];
           தளம்_செய் "INSERT INTO paRRuccIttu VALUES (3, '100')", [];
           {rest}"#
    )
}

/// One column of a query's rows, in order.
fn column(vm: &VM, variable: &str, field: &str) -> Vec<Value> {
    match vm.variables.get(variable) {
        Some(Value::Array(rows)) => rows
            .iter()
            .map(|row| match row {
                Value::Map(record) => record.get(field).cloned().unwrap_or(Value::Null),
                other => panic!("a row should be a record, not {other:?}"),
            })
            .collect(),
        other => panic!("{variable} should be an array of rows, not {other:?}"),
    }
}

fn outcome(vm: &VM, variable: &str) -> Result<(), String> {
    match vm.variables.get(variable) {
        Some(Value::Ok(_)) => Ok(()),
        Some(Value::Err(why)) => Err(why.to_string()),
        other => panic!("{variable} should be a சரி or a தவறு, not {other:?}"),
    }
}

#[test]
fn a_function_of_the_program_is_called_once_per_row_and_stays_exact() {
    let vm = run(&with_invoices(
        r#"_r = தளம்_செயல்_பதிவு("moqqam_vari", மொத்தம்_வரி, 2);
           தளம்_வினா "SELECT moqqam_vari(qokY, 18) AS moqqam FROM paRRuccIttu ORDER BY eN", [], வ;"#,
    ))
    .unwrap();

    assert_eq!(outcome(&vm, "_r"), Ok(()));
    assert_eq!(
        column(&vm, "வ", "moqqam"),
        vec![number("1180"), number("295.59"), number("118")]
    );
}

#[test]
fn exact_results_compare_wrongly_and_numeric_ones_compare_right() {
    // The trap, pinned: an exact (text) result is larger than any number to SQLite.
    let exact = run(&with_invoices(
        r#"தளம்_செயல்_பதிவு("moqqam_vari", மொத்தம்_வரி, 2);
           தளம்_வினா "SELECT eN FROM paRRuccIttu WHERE moqqam_vari(qokY, 18) > 500", [], வ;
           எத்தனை = நீளம்(வ);"#,
    ))
    .unwrap();
    assert_eq!(exact.variables.get("எத்தனை"), Some(&number("3")));

    let numeric = run(&with_invoices(
        r#"தளம்_செயல்_பதிவு("moqqam_vari", மொத்தம்_வரி, 2, "numeric");
           தளம்_வினா "SELECT eN FROM paRRuccIttu WHERE moqqam_vari(qokY, 18) > 500", [], வ;
           எத்தனை = நீளம்(வ);"#,
    ))
    .unwrap();
    assert_eq!(numeric.variables.get("எத்தனை"), Some(&number("1")));
}

#[test]
fn a_function_value_that_carries_a_number_is_registered_and_uses_it() {
    let vm = run(&with_invoices(
        r#"செயல் வரி_செயல்(விகிதம்) {
               திரும்பு செயல்(தொகை) { திரும்பு தொகை * (1 + விகிதம் / 100); };
           }
           பதினெட்டு = வரி_செயல்(18);
           _r = தளம்_செயல்_பதிவு("moqqam_18", பதினெட்டு, 1);
           தளம்_வினா "SELECT moqqam_18(qokY) AS moqqam FROM paRRuccIttu ORDER BY eN", [], வ;"#,
    ))
    .unwrap();

    assert_eq!(outcome(&vm, "_r"), Ok(()));
    assert_eq!(
        column(&vm, "வ", "moqqam"),
        vec![number("1180"), number("295.59"), number("118")]
    );
}

// --- What may not be registered ---------------------------------------------

#[test]
fn a_function_that_reads_the_environment_is_refused_and_nothing_is_registered() {
    let vm = run(&with_invoices(
        r#"செயல் கசிவு(தொகை) {
               திரும்பு சூழல்("HOME");
           }
           _r = தளம்_செயல்_பதிவு("kacivu", கசிவு, 1);
           _q = தளம்_வினா_முயற்சி("SELECT kacivu(1)", []);"#,
    ))
    .unwrap();

    let why = outcome(&vm, "_r").unwrap_err();
    assert!(why.contains("reaches outside the database"), "{why}");
    let query = outcome(&vm, "_q").unwrap_err();
    assert!(query.contains("no such function"), "{query}");
}

#[test]
fn a_function_that_reaches_outside_through_another_is_refused() {
    let vm = run(&with_invoices(
        r#"செயல் உள்ளே() {
               திரும்பு சூழல்("HOME");
           }
           செயல் நடுவே(தொகை) {
               திரும்பு உள்ளே();
           }
           _r = தளம்_செயல்_பதிவு("naTuvE", நடுவே, 1);"#,
    ))
    .unwrap();

    assert!(outcome(&vm, "_r").unwrap_err().contains("reaches outside"));
}

#[test]
fn a_builtin_that_reaches_outside_cannot_be_registered_directly() {
    let vm = run(&with_invoices(r#"_r = தளம்_செயல்_பதிவு("cUzal", சூழல், 1);"#)).unwrap();

    assert!(outcome(&vm, "_r").unwrap_err().contains("reaches outside"));
}

#[test]
fn a_function_that_reads_a_file_is_refused() {
    let vm = run(&with_invoices(
        r#"செயல் கோப்புத்_தேடல்(தொகை) {
               திரும்பு கோப்பு_உள்ளதா("x.txt");
           }
           _r = தளம்_செயல்_பதிவு("kOppuqqEqal", கோப்புத்_தேடல், 1);"#,
    ))
    .unwrap();

    assert!(outcome(&vm, "_r").unwrap_err().contains("reaches outside"));
}

// --- Arguments and setup that are wrong -------------------------------------

#[test]
fn the_wrong_number_of_arguments_for_the_function_is_refused() {
    let vm = run(&with_invoices(
        r#"_r = தளம்_செயல்_பதிவு("moqqam_vari", மொத்தம்_வரி, 3);"#,
    ))
    .unwrap();

    let why = outcome(&vm, "_r").unwrap_err();
    assert!(why.contains("takes 2 argument"), "{why}");
}

#[test]
fn a_second_argument_that_is_not_a_function_is_refused() {
    let vm = run(&with_invoices(
        r#"_r = தளம்_செயல்_பதிவு("moqqam_vari", 5, 2);"#,
    ))
    .unwrap();

    assert!(
        outcome(&vm, "_r")
            .unwrap_err()
            .contains("must be a function")
    );
}

#[test]
fn an_unknown_number_mode_is_refused() {
    let vm = run(&with_invoices(
        r#"_r = தளம்_செயல்_பதிவு("moqqam_vari", மொத்தம்_வரி, 2, "float");"#,
    ))
    .unwrap();

    assert!(outcome(&vm, "_r").unwrap_err().contains("exact"));
}

#[test]
fn with_no_database_open_it_says_so_and_does_not_fail_the_program() {
    let vm = run(r#"செயல் மொத்தம்_வரி(தொகை, விகிதம்) {
               திரும்பு தொகை * (1 + விகிதம் / 100);
           }
           _r = தளம்_செயல்_பதிவு("moqqam_vari", மொத்தம்_வரி, 2);"#)
    .unwrap();

    assert!(outcome(&vm, "_r").is_err());
}

// --- Running it -------------------------------------------------------------

#[test]
fn a_runtime_error_in_the_function_fails_the_query() {
    let failure = run(&with_invoices(
        r#"செயல் வகு(தொகை) {
               திரும்பு தொகை / 0;
           }
           தளம்_செயல்_பதிவு("vaku", வகு, 1);
           தளம்_வினா "SELECT vaku(qokY) FROM paRRuccIttu", [], வ;"#,
    ))
    .unwrap_err();

    assert!(!failure.is_empty());
}

#[test]
fn a_function_that_never_ends_is_stopped() {
    let failure = run(&with_invoices(
        r#"செயல் முடிவில்லா(தொகை) {
               எண்ணி = 0;
               (எண்ணி >= 0) சுற்று {
                   எண்ணி = எண்ணி + 1;
               }
               திரும்பு எண்ணி;
           }
           தளம்_செயல்_பதிவு("muTivillA", முடிவில்லா, 1);
           தளம்_வினா "SELECT muTivillA(qokY) FROM paRRuccIttu", [], வ;"#,
    ))
    .unwrap_err();

    assert!(failure.contains("100000"), "{failure}");
}

#[test]
fn what_a_function_prints_goes_nowhere() {
    host::begin_capture();
    let outcome = run(&with_invoices(
        r#"செயல் பேசு(தொகை) {
               அச்சு "வணக்கம்";
               திரும்பு தொகை;
           }
           தளம்_செயல்_பதிவு("pEcu", பேசு, 1);
           தளம்_வினா "SELECT pEcu(qokY) FROM paRRuccIttu", [], வ;"#,
    ));
    let printed = host::end_capture();

    assert!(outcome.is_ok(), "{:?}", outcome.err());
    assert_eq!(printed, "", "the function's output must not leak");
}

// --- A pooled connection forgets --------------------------------------------

#[test]
fn a_function_registered_by_one_program_is_gone_for_the_next() {
    let path = std::env::temp_dir().join("etamil_function_pool.db");
    let _ = std::fs::remove_file(&path);
    let shown = path
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");

    // The first program registers a function and ends; its VM, and so its lease on
    // the connection, are dropped, and the connection goes back to the pool.
    let first = run(&format!(
        r#"செயல் மொத்தம்_வரி(தொகை, விகிதம்) {{
               திரும்பு தொகை * (1 + விகிதம் / 100);
           }}
           தளம்_இணை சீகுலைட், "{shown}";
           _r = தளம்_செயல்_பதிவு("moqqam_vari", மொத்தம்_வரி, 2);
           தளம்_வினா "SELECT moqqam_vari(100, 18) AS moqqam", [], வ;"#
    ))
    .unwrap();
    assert_eq!(outcome(&first, "_r"), Ok(()));
    drop(first);

    // The second does not register it, and must not find the first one's.
    let second = run(&format!(
        r#"தளம்_இணை சீகுலைட், "{shown}";
           _q = தளம்_வினா_முயற்சி("SELECT moqqam_vari(100, 18)", []);"#
    ))
    .unwrap();
    let _ = std::fs::remove_file(&path);

    let why = outcome(&second, "_q").unwrap_err();
    assert!(why.contains("no such function"), "{why}");
}
