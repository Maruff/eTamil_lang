// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>

use std::time::{Duration, Instant};

use etamil_compiler::db::{self, Database as _};

use super::*;

const GST: &str = "செயல் gst_total(amount, rate) {\n  திரும்பு amount * (1 + rate / 100);\n}\n";

fn db_with_invoices() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE invoices (id INTEGER PRIMARY KEY, amount TEXT);
             INSERT INTO invoices VALUES (1, '1000'), (2, '250.50'), (3, '100');",
        )
        .unwrap();
    connection
}

fn one<T: rusqlite::types::FromSql>(connection: &Connection, sql: &str) -> T {
    connection.query_row(sql, [], |row| row.get(0)).unwrap_or_else(|e| panic!("{sql}: {e}"))
}

fn error_of(connection: &Connection, sql: &str) -> String {
    connection.query_row(sql, [], |row| row.get::<_, SqlValue>(0)).expect_err(sql).to_string()
}

#[test]
fn an_etamil_function_runs_once_per_row_of_a_query() {
    let connection = db_with_invoices();
    register(&connection, GST, &[SqlFunction::new("gst_total", "gst_total", 2)]).unwrap();
    let rows: Vec<(i64, String)> = connection
        .prepare("SELECT id, gst_total(amount, 18) FROM invoices ORDER BY id")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    // Exact: 250.50 * 1.18 is 295.59, not 295.5899999...
    assert_eq!(rows, vec![(1, "1180".into()), (2, "295.59".into()), (3, "118".into())]);
}

#[test]
fn money_stays_exact() {
    let connection = Connection::open_in_memory().unwrap();
    // Not `add`: that is an SQL keyword, and SQLite refuses it as a function name.
    register(&connection, "செயல் plus(a, b) {\n  திரும்பு a + b;\n}\n", &[SqlFunction::new("plus", "plus", 2)]).unwrap();
    assert_eq!(one::<String>(&connection, "SELECT plus('0.1', '0.2')"), "0.3");
}

#[test]
fn a_function_can_have_a_tamil_name_in_sql_and_tamil_text_survives() {
    let connection = Connection::open_in_memory().unwrap();
    // Not `ஒட்டு`: that is an eTamil keyword, and the eTamil parser refuses it as a name.
    let source = "செயல் பெயர்_ஒட்டு(a, b) {\n  திரும்பு a & b;\n}\n";
    register(&connection, source, &[SqlFunction::new("பெயர்_ஒட்டு", "பெயர்_ஒட்டு", 2)]).unwrap();
    assert_eq!(one::<String>(&connection, "SELECT பெயர்_ஒட்டு('வண', 'க்கம்')"), "வணக்கம்");
}

#[test]
fn the_sql_name_need_not_be_the_etamil_name() {
    let connection = Connection::open_in_memory().unwrap();
    register(&connection, GST, &[SqlFunction::new("with_tax", "gst_total", 2)]).unwrap();
    assert_eq!(one::<String>(&connection, "SELECT with_tax('100', 5)"), "105");
}

// --- the pitfall that decides the API: SQLite orders TEXT after every number ---

#[test]
fn exact_text_results_compare_wrongly_in_sql_and_native_ones_do_not() {
    let connection = db_with_invoices();
    register(
        &connection,
        GST,
        &[
            SqlFunction::new("gst_text", "gst_total", 2),
            SqlFunction::new("gst_native", "gst_total", 2).numeric(),
        ],
    )
    .unwrap();
    // Only the 1000 row (1180) is over 500.
    assert_eq!(one::<i64>(&connection, "SELECT count(*) FROM invoices WHERE gst_native(amount, 18) > 500"), 1);
    // The text version is "greater" for every row, because TEXT sorts after INTEGER: all 3.
    assert_eq!(one::<i64>(&connection, "SELECT count(*) FROM invoices WHERE gst_text(amount, 18) > 500"), 3);
    // CAST is the way to compare an exact result.
    assert_eq!(one::<i64>(&connection, "SELECT count(*) FROM invoices WHERE CAST(gst_text(amount, 18) AS REAL) > 500"), 1);
}

#[test]
fn native_results_are_integers_when_whole_and_reals_when_not() {
    let connection = db_with_invoices();
    register(&connection, GST, &[SqlFunction::new("g", "gst_total", 2).numeric()]).unwrap();
    assert_eq!(one::<String>(&connection, "SELECT typeof(g('1000', 18))"), "integer");
    assert_eq!(one::<String>(&connection, "SELECT typeof(g('250.50', 18))"), "real");
}

#[test]
fn a_sum_of_exact_text_is_a_float_and_says_so_here() {
    let connection = db_with_invoices();
    register(&connection, GST, &[SqlFunction::new("g", "gst_total", 2)]).unwrap();
    // SQLite converts the text to REAL to add it. The sum is right to a float's precision,
    // not to the paisa, which is the cost of aggregating in SQL: sum in eTamil to stay exact.
    let sum: f64 = one(&connection, "SELECT SUM(g(amount, 18)) FROM invoices");
    assert!((sum - (1180.0 + 295.59 + 118.0)).abs() < 1e-9, "{sum}");
}

// --- how it fails ---

#[test]
fn a_runtime_error_is_a_sql_error() {
    let connection = Connection::open_in_memory().unwrap();
    register(&connection, "செயல் boom(a) {\n  திரும்பு a / 0;\n}\n", &[SqlFunction::new("boom", "boom", 1)]).unwrap();
    let message = error_of(&connection, "SELECT boom(1)");
    assert!(!message.is_empty(), "{message}");
}

#[test]
fn an_endless_loop_stops_at_the_step_limit() {
    let connection = Connection::open_in_memory().unwrap();
    let source = "செயல் spin(n) {\n  i = 0;\n  (i >= 0) சுற்று {\n    i = i + 1;\n  }\n  திரும்பு i;\n}\n";
    register(&connection, source, &[SqlFunction::new("spin", "spin", 1)]).unwrap();
    let started = Instant::now();
    let _ = error_of(&connection, "SELECT spin(1)");
    assert!(started.elapsed() < Duration::from_secs(5), "took {:?}", started.elapsed());
}

#[test]
fn a_function_that_reaches_outside_cannot_be_registered() {
    let connection = Connection::open_in_memory().unwrap();
    let source = "செயல் leak() {\n  திரும்பு _env(\"HOME\");\n}\n";
    let error = register(&connection, source, &[SqlFunction::new("leak", "leak", 0)]).unwrap_err();
    assert!(error.0.contains("_env"), "{error}");
    assert!(error_of(&connection, "SELECT leak()").contains("no such function"));
}

#[test]
fn a_top_level_statement_is_refused() {
    let connection = Connection::open_in_memory().unwrap();
    let source = "x = 1;\nசெயல் f(a) {\n  திரும்பு a;\n}\n";
    let error = register(&connection, source, &[SqlFunction::new("f", "f", 1)]).unwrap_err();
    assert!(error.0.contains("top level"), "{error}");
}

#[test]
fn the_function_must_exist_and_have_the_arity_asked_for() {
    let connection = Connection::open_in_memory().unwrap();
    let missing = register(&connection, GST, &[SqlFunction::new("g", "nope", 2)]).unwrap_err();
    assert!(missing.0.contains("nope"), "{missing}");
    let arity = register(&connection, GST, &[SqlFunction::new("g", "gst_total", 3)]).unwrap_err();
    assert!(arity.0.contains("2 argument"), "{arity}");
}

#[test]
fn nothing_is_registered_unless_everything_can_be() {
    let connection = Connection::open_in_memory().unwrap();
    let both = [SqlFunction::new("first", "gst_total", 2), SqlFunction::new("second", "missing", 1)];
    assert!(register(&connection, GST, &both).is_err());
    assert!(error_of(&connection, "SELECT first('1', 1)").contains("no such function"));
}

#[test]
fn registering_again_replaces_the_function() {
    let connection = Connection::open_in_memory().unwrap();
    register(&connection, "செயல் f() {\n  திரும்பு 1;\n}\n", &[SqlFunction::new("f", "f", 0)]).unwrap();
    assert_eq!(one::<String>(&connection, "SELECT f()"), "1");
    register(&connection, "செயல் f() {\n  திரும்பு 2;\n}\n", &[SqlFunction::new("f", "f", 0)]).unwrap();
    assert_eq!(one::<String>(&connection, "SELECT f()"), "2");
}

// --- values ---

#[test]
fn null_in_is_null_and_null_out_is_null() {
    let connection = Connection::open_in_memory().unwrap();
    // Not `nothing`: that is an SQL keyword (`DO NOTHING`).
    let source = "செயல் nil_value() {\n  திரும்பு இன்மை;\n}\nசெயல் kind(a) {\n  திரும்பு _typeof(a);\n}\n";
    register(&connection, source, &[SqlFunction::new("nil_value", "nil_value", 0), SqlFunction::new("kind", "kind", 1)]).unwrap();
    assert!(connection.query_row("SELECT nil_value()", [], |r| r.get::<_, Option<String>>(0)).unwrap().is_none());
    // What eTamil sees: NULL is its own kind; '12.5' and 7 are both numbers; 'abc' is text.
    let kind = |sql: &str| one::<String>(&connection, sql);
    assert_ne!(kind("SELECT kind(NULL)"), kind("SELECT kind('abc')"));
    assert_ne!(kind("SELECT kind('abc')"), kind("SELECT kind(7)"));
    assert_eq!(kind("SELECT kind('12.5')"), kind("SELECT kind(7)"));
}

#[test]
fn a_collection_has_no_sql_form() {
    let connection = Connection::open_in_memory().unwrap();
    register(&connection, "செயல் arr() {\n  திரும்பு [1, 2];\n}\n", &[SqlFunction::new("arr", "arr", 0)]).unwrap();
    assert!(error_of(&connection, "SELECT arr()").contains("array"));
}

#[test]
fn a_result_value_is_unwrapped_or_fails() {
    let connection = Connection::open_in_memory().unwrap();
    let source = "செயல் good() {\n  திரும்பு _ok(5);\n}\nசெயல் bad() {\n  திரும்பு _err(\"no\");\n}\n";
    register(&connection, source, &[SqlFunction::new("good", "good", 0), SqlFunction::new("bad", "bad", 0)]).unwrap();
    assert_eq!(one::<String>(&connection, "SELECT good()"), "5");
    assert!(error_of(&connection, "SELECT bad()").contains("தவறு"));
}

// --- agreement with the compiler's own SQLite adapter ---

#[test]
fn a_decimal_stored_by_the_compilers_adapter_is_read_the_same_way() {
    let path = std::env::temp_dir().join(format!("etamil_sqlite_fn_{}.db", std::process::id()));
    let path_text = path.to_string_lossy().to_string();
    let _ = std::fs::remove_file(&path);
    {
        let mut compiler_db = db::open("SQLite", &path_text).unwrap();
        compiler_db.execute("CREATE TABLE t (v TEXT)", &[]).unwrap();
        compiler_db.execute("INSERT INTO t VALUES (?)", &[Value::Number(Decimal::from_str("250.50").unwrap())]).unwrap();
    }
    let connection = Connection::open(&path).unwrap();
    register(&connection, GST, &[SqlFunction::new("g", "gst_total", 2)]).unwrap();
    assert_eq!(one::<String>(&connection, "SELECT g(v, 18) FROM t"), "295.59");
    drop(connection);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn conversions_match_the_adapters_rules() {
    assert_eq!(from_sql(ValueRef::Integer(7)), Value::Number(Decimal::from(7)));
    assert_eq!(from_sql(ValueRef::Text(b"0.10")), Value::Number(Decimal::from_str("0.10").unwrap()));
    assert_eq!(from_sql(ValueRef::Text("வணக்கம்".as_bytes())), Value::String("வணக்கம்".into()));
    assert_eq!(from_sql(ValueRef::Null), Value::Null);
    assert_eq!(into_sql(Value::Number(Decimal::from_str("1.50").unwrap()), Numbers::ExactText).unwrap(), SqlValue::Text("1.5".into()));
    assert_eq!(into_sql(Value::Boolean(true), Numbers::ExactText).unwrap(), SqlValue::Integer(1));
}

// --- cost: run with `cargo test --release -- --ignored --nocapture` ---

#[test]
#[ignore]
fn per_row_cost() {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch("CREATE TABLE n (v INTEGER)").unwrap();
    let rows = 20_000;
    let mut insert = connection.prepare("INSERT INTO n VALUES (?)").unwrap();
    for i in 0..rows {
        insert.execute([i]).unwrap();
    }
    register(&connection, GST, &[SqlFunction::new("g", "gst_total", 2)]).unwrap();
    let started = Instant::now();
    let total: i64 = one(&connection, "SELECT count(g(v, 18)) FROM n");
    let elapsed = started.elapsed();
    assert_eq!(total, rows);
    println!("TIMING {rows} rows in {elapsed:?}: {:?} per row", elapsed / rows as u32);
}
