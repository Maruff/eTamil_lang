// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>

use super::*;

fn pure(source: &str) -> Result<(), String> {
    let tokens = lexer::tokenize(source).map_err(|e| format!("{e:?}"))?;
    let statements = Parser::new(tokens.iter()).parse().map_err(|e| e.to_string())?;
    require_pure(&statements)
}

#[test]
fn an_expression_is_evaluated_to_its_text() {
    assert_eq!(eval_expression("1 + 2"), Ok("3".to_string()));
    assert_eq!(eval_expression("\"வணக்கம்\" & \"!\""), Ok("வணக்கம்!".to_string()));
    // A decimal stays exact, which is the point of the language for money.
    assert_eq!(eval_expression("0.1 + 0.2"), Ok("0.3".to_string()));
}

#[test]
fn a_program_returns_what_it_printed() {
    let outcome = run("செயல் இரட்டிப்பு(a) {\n  திரும்பு a * 2;\n}\nஅச்சு(இரட்டிப்பு(21));\nஅச்சு(\"முடிந்தது\");");
    assert!(outcome.ok, "{outcome:?}");
    assert_eq!(outcome.output, "42\nமுடிந்தது\n");
}

#[test]
fn an_error_is_reported_with_its_stage() {
    let syntax = run("அச்சு(");
    assert_eq!((syntax.ok, syntax.stage), (false, Some("load")));
    let runtime = run("அச்சு(1 / 0);");
    assert!(!runtime.ok, "{runtime:?}");
}

#[test]
fn capture_is_off_again_after_a_run() {
    run("அச்சு(1);");
    assert!(!host::capturing(), "a leaked capture would swallow the next caller's output");
    run("அச்சு(");
    assert!(!host::capturing());
}

#[test]
fn one_run_does_not_see_the_previous_runs_output() {
    assert_eq!(run("அச்சு(\"a\");").output, "a\n");
    assert_eq!(run("அச்சு(\"b\");").output, "b\n");
}

#[test]
fn an_endless_loop_stops_at_the_step_limit() {
    let outcome = run("i = 0;\n(i >= 0) சுற்று {\n  i = i + 1;\n}");
    assert!(!outcome.ok);
    assert_eq!(outcome.stage, Some("run"));
}

// --- the allowlist ---

#[test]
fn statements_that_reach_outside_are_refused() {
    for source in [
        "இறக்கு \"nUlakam/paNam/paNam.qmz\";",
        "உள்ளிடு x;",
        "கோப்பு_திற \"a.txt\", \"write\";",
        "(1 > 0) எனில் {\n  கோப்பு_திற \"a.txt\", \"write\";\n}",
        "செயல் f() {\n  கோப்பு_திற \"a.txt\", \"write\";\n}",
    ] {
        // Refused by the allowlist itself, not by failing to parse.
        let outcome = run(source);
        assert_eq!(outcome.stage, Some("pure"), "{source}: {outcome:?}");
    }
}

#[test]
fn a_pure_program_passes_the_check() {
    assert_eq!(pure("x = 1;\nஅச்சு(x + 2);"), Ok(()));
    assert_eq!(pure("செயல் f(a) {\n  திரும்பு a;\n}\nஅச்சு(f(1));"), Ok(()));
}

#[test]
fn a_forbidden_builtin_is_refused_by_call_and_by_name() {
    // _env reads the host's environment, which a query has no business doing.
    let called = pure("அச்சு(_env(\"HOME\"));");
    assert!(called.is_err(), "{called:?}");
    let message = called.unwrap_err();
    assert!(message.contains("_env"), "{message}");
    // Named as a value and called later is the same thing.
    assert!(pure("f = _env;\nஅச்சு(f(\"HOME\"));").is_err());
    // And inside a lambda or a method.
    assert!(pure("g = செயல்(x) {\n  திரும்பு _env(x);\n};\nஅச்சு(g(\"a\"));").is_err());
}

#[test]
fn a_pure_builtin_is_allowed_in_every_spelling() {
    assert_eq!(pure("அச்சு(_length(\"abc\"));"), Ok(()));
    assert_eq!(pure("அச்சு(நீளம்(\"abc\"));"), Ok(()));
    assert_eq!(eval_expression("நீளம்(\"வணக்கம்\")"), Ok("5".to_string()));
}

#[test]
fn the_allowed_list_is_sorted_and_covers_what_was_asked_for() {
    // Sorted, because `forbidden_builtin` searches it by bisection.
    assert!(ALLOWED_BUILTINS.windows(2).all(|pair| pair[0] < pair[1]));
    // Every alias on the pure list produced at least one spelling (the build fails if one
    // vanished), and the Tamil spellings came with them.
    assert!(ALLOWED_BUILTINS.len() >= ALLOWED_ALIASES * 2, "{} names for {} aliases", ALLOWED_BUILTINS.len(), ALLOWED_ALIASES);
    assert!(ALLOWED_BUILTINS.contains(&"_length") && ALLOWED_BUILTINS.contains(&"நீளம்"));
}

#[test]
fn nothing_that_reaches_outside_is_on_the_allowed_list() {
    for alias in [
        "_env", "_exit", "_run", "_sleepMs", "_fileExists", "_fileSave", "_readDir", "_httpGet",
        "_httpPost", "_mongoFind", "_redisCommand", "_tryQuery", "_tryExecute", "_pinWrite",
        "_serialOpen", "_issueToken", "_encryptionKey",
    ] {
        assert!(vm::is_builtin(alias), "{alias} is not a builtin any more: update this list");
        assert!(ALLOWED_BUILTINS.binary_search(&alias).is_err(), "{alias} is allowed");
    }
}

// --- the risk the spike was asked to find ---

#[test]
fn runaway_recursion_is_an_error_not_a_crash() {
    // A stack overflow aborts the whole process, which in a database is the backend, and
    // takes the connection with it. Run on a thread with a small stack to see whether the
    // VM stops itself first.
    let handle = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| run("செயல் f(n) {\n  திரும்பு f(n + 1);\n}\nஅச்சு(f(1));"))
        .unwrap();
    let outcome = handle.join().expect("the thread must not crash");
    assert_eq!(outcome.stage, Some("run"), "{outcome:?}");
    // The VM counts call depth itself (256), so the Rust stack is never the limit.
    assert!(outcome.error.unwrap_or_default().contains("256"));
}
