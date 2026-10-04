// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! PL/eTamil, step one: a SQL function that runs an eTamil program.
//!
//! Deliberately thin. Running a program, and refusing one that reaches outside the
//! database, is `etamil_pg_eval`, which is tested without PostgreSQL. This file is
//! only the part that has to be PostgreSQL: turning arguments and results into SQL
//! values, and a failure into a SQL error.

use pgrx::prelude::*;

::pgrx::pg_module_magic!(name, version);

// `LANGUAGE pletamil`: the call handler, and the code that reads a function's definition and
// converts its arguments and result.
mod handler;

/// Run an eTamil program and return what it printed, without the final newline.
///
/// The program may only compute: statements and builtins that reach outside the
/// database (files, the network, other databases, the host) are refused with an error.
#[pg_extern]
fn etamil_eval(program: &str) -> String {
    let outcome = etamil_pg_eval::run(program);
    if outcome.ok {
        outcome.output.trim_end_matches('\n').to_string()
    } else {
        error!("{}", outcome.error.unwrap_or_else(|| "the eTamil program failed".to_string()))
    }
}

/// The text of one eTamil expression: `etamil_expr('0.1 + 0.2')` is `0.3`, exactly.
#[pg_extern]
fn etamil_expr(expression: &str) -> String {
    match etamil_pg_eval::eval_expression(expression) {
        Ok(text) => text,
        Err(message) => error!("{}", message),
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn runs_a_program_inside_postgres() {
        let out = Spi::get_one::<String>("SELECT etamil_eval('அச்சு(1 + 2);')").unwrap();
        assert_eq!(out.as_deref(), Some("3"));
    }

    #[pg_test]
    fn decimals_stay_exact() {
        let out = Spi::get_one::<String>("SELECT etamil_expr('0.1 + 0.2')").unwrap();
        assert_eq!(out.as_deref(), Some("0.3"));
    }

    #[pg_test]
    fn tamil_text_survives_the_round_trip() {
        let out = Spi::get_one::<String>("SELECT etamil_expr('நீளம்(\"வணக்கம்\")')").unwrap();
        assert_eq!(out.as_deref(), Some("5"));
        let echoed = Spi::get_one::<String>("SELECT etamil_eval('அச்சு(\"வணக்கம்\");')").unwrap();
        assert_eq!(echoed.as_deref(), Some("வணக்கம்"));
    }

    #[pg_test]
    fn runs_once_per_row_of_a_query() {
        let out = Spi::get_one::<String>(
            "SELECT string_agg(etamil_expr(n || ' * 2'), ',' ORDER BY n) FROM generate_series(1, 3) AS n",
        )
        .unwrap();
        assert_eq!(out.as_deref(), Some("2,4,6"));
    }

    #[pg_test]
    fn calls_do_not_share_output() {
        let out = Spi::get_one::<String>(
            "SELECT etamil_eval('அச்சு(\"a\");') || etamil_eval('அச்சு(\"b\");')",
        )
        .unwrap();
        assert_eq!(out.as_deref(), Some("ab"));
    }

    // pgrx compares the message with `error = ` exactly, not as a prefix.
    #[pg_test(error = "a database function may not use this statement (Import)")]
    fn an_import_is_refused() {
        let _ = Spi::get_one::<String>("SELECT etamil_eval('இறக்கு \"nUlakam/paNam/paNam.qmz\";')");
    }

    #[pg_test(error = "a database function may not use `_env`: it reaches outside the database")]
    fn reading_the_environment_is_refused() {
        let _ = Spi::get_one::<String>("SELECT etamil_eval('அச்சு(_env(\"HOME\"));')");
    }

    // The parser's message is bilingual and carries a position; what matters is that it is a
    // SQL error and not a crash, so it is caught rather than matched.
    #[pg_test]
    fn a_syntax_error_is_a_sql_error() {
        let raised = PgTryBuilder::new(|| {
            let _ = Spi::get_one::<String>("SELECT etamil_eval('அச்சு(')");
            false
        })
        .catch_others(|_| true)
        .execute();
        assert!(raised, "a syntax error must raise a SQL error");
    }

    // --- LANGUAGE pletamil: a function with typed arguments and a typed result ---

    #[pg_test]
    fn a_function_takes_numeric_arguments_and_returns_an_exact_numeric() {
        Spi::run(
            r#"CREATE FUNCTION gst_total(amount numeric, rate numeric) RETURNS numeric LANGUAGE pletamil AS $$
                திரும்பு amount * (1 + rate / 100);
            $$"#,
        )
        .unwrap();
        let out = Spi::get_one::<AnyNumeric>("SELECT gst_total(250.50, 18)").unwrap();
        assert_eq!(out.unwrap().to_string(), "295.59");
    }

    #[pg_test]
    fn a_function_runs_over_a_column_and_the_sum_stays_exact() {
        Spi::run(
            r#"CREATE FUNCTION gst_total(amount numeric, rate numeric) RETURNS numeric LANGUAGE pletamil AS $$
                திரும்பு amount * (1 + rate / 100);
            $$"#,
        )
        .unwrap();
        Spi::run("CREATE TEMP TABLE invoices (amount numeric)").unwrap();
        Spi::run("INSERT INTO invoices VALUES (1000), (250.50), (100)").unwrap();
        // 1180 + 295.59 + 118. Exact, which a float-based path would not promise.
        let sum = Spi::get_one::<String>("SELECT sum(gst_total(amount, 18))::text FROM invoices").unwrap();
        assert_eq!(sum.as_deref(), Some("1593.59"));
    }

    #[pg_test]
    fn integer_bigint_text_and_boolean_arguments_and_results() {
        Spi::run("CREATE FUNCTION twice(n integer) RETURNS integer LANGUAGE pletamil AS $$ திரும்பு n * 2; $$").unwrap();
        assert_eq!(Spi::get_one::<i32>("SELECT twice(21)").unwrap(), Some(42));

        Spi::run("CREATE FUNCTION big(n bigint) RETURNS bigint LANGUAGE pletamil AS $$ திரும்பு n + 1; $$").unwrap();
        assert_eq!(Spi::get_one::<i64>("SELECT big(9000000000)").unwrap(), Some(9_000_000_001));

        Spi::run(r#"CREATE FUNCTION greet(t text) RETURNS text LANGUAGE pletamil AS $$ திரும்பு "வணக்கம் " & t; $$"#).unwrap();
        assert_eq!(Spi::get_one::<String>("SELECT greet('உலகம்')").unwrap().as_deref(), Some("வணக்கம் உலகம்"));

        Spi::run("CREATE FUNCTION big_enough(n numeric) RETURNS boolean LANGUAGE pletamil AS $$ திரும்பு n > 100; $$").unwrap();
        assert_eq!(Spi::get_one::<bool>("SELECT big_enough(150)").unwrap(), Some(true));
        assert_eq!(Spi::get_one::<bool>("SELECT big_enough(50)").unwrap(), Some(false));
    }

    #[pg_test]
    fn a_function_may_have_a_tamil_name() {
        Spi::run(r#"CREATE FUNCTION வணக்கம்(t text) RETURNS text LANGUAGE pletamil AS $$ திரும்பு "வணக்கம் " & t; $$"#).unwrap();
        assert_eq!(Spi::get_one::<String>("SELECT வணக்கம்('உலகம்')").unwrap().as_deref(), Some("வணக்கம் உலகம்"));
    }

    #[pg_test]
    fn unnamed_arguments_are_arg1_arg2_by_position() {
        Spi::run("CREATE FUNCTION plus(integer, integer) RETURNS integer LANGUAGE pletamil AS $$ திரும்பு arg1 + arg2; $$").unwrap();
        assert_eq!(Spi::get_one::<i32>("SELECT plus(3, 4)").unwrap(), Some(7));
    }

    #[pg_test]
    fn null_in_and_null_out() {
        // STRICT: PostgreSQL never calls the function for a NULL, and answers NULL.
        Spi::run("CREATE FUNCTION strict_id(a integer) RETURNS integer STRICT LANGUAGE pletamil AS $$ திரும்பு a; $$").unwrap();
        assert_eq!(Spi::get_one::<i32>("SELECT strict_id(NULL)").unwrap(), None);
        // Not STRICT: the body runs, sees null, and can return it.
        Spi::run("CREATE FUNCTION pass(a integer) RETURNS integer LANGUAGE pletamil AS $$ திரும்பு a; $$").unwrap();
        assert_eq!(Spi::get_one::<i32>("SELECT pass(NULL)").unwrap(), None);
        Spi::run("CREATE FUNCTION nil() RETURNS integer LANGUAGE pletamil AS $$ திரும்பு இன்மை; $$").unwrap();
        assert_eq!(Spi::get_one::<i32>("SELECT nil()").unwrap(), None);
    }

    #[pg_test]
    fn replacing_a_function_replaces_its_body() {
        Spi::run("CREATE FUNCTION f() RETURNS integer LANGUAGE pletamil AS $$ திரும்பு 1; $$").unwrap();
        assert_eq!(Spi::get_one::<i32>("SELECT f()").unwrap(), Some(1));
        Spi::run("CREATE OR REPLACE FUNCTION f() RETURNS integer LANGUAGE pletamil AS $$ திரும்பு 2; $$").unwrap();
        assert_eq!(Spi::get_one::<i32>("SELECT f()").unwrap(), Some(2));
    }

    #[pg_test(error = "a database function may not use `_env`: it reaches outside the database")]
    fn a_function_body_that_reaches_outside_is_refused() {
        Spi::run(r#"CREATE FUNCTION leak() RETURNS text LANGUAGE pletamil AS $$ திரும்பு _env("HOME"); $$"#).unwrap();
        let _ = Spi::get_one::<String>("SELECT leak()");
    }

    // The validator refuses a bad body when the function is created, not when it is first called.
    #[pg_test]
    fn a_bad_body_is_refused_at_create_function() {
        let raised = PgTryBuilder::new(|| {
            Spi::run("CREATE FUNCTION bad() RETURNS integer LANGUAGE pletamil AS $$ திரும்பு ; $$").unwrap();
            false
        })
        .catch_others(|_| true)
        .execute();
        assert!(raised, "a syntax error must be refused by CREATE FUNCTION");
    }

    // pg_dump output turns the check off so a restore does not re-check every body.
    #[pg_test]
    fn the_check_can_be_turned_off() {
        Spi::run("SET check_function_bodies = off").unwrap();
        Spi::run("CREATE FUNCTION bad() RETURNS integer LANGUAGE pletamil AS $$ திரும்பு ; $$").unwrap();
    }

    #[pg_test(error = "pletamil: argument 1 has type jsonb, which is not supported")]
    fn an_unsupported_argument_type_is_refused_by_name() {
        Spi::run("CREATE FUNCTION j(a jsonb) RETURNS text LANGUAGE pletamil AS $$ திரும்பு \"x\"; $$").unwrap();
        let _ = Spi::get_one::<String>("SELECT j('{}'::jsonb)");
    }

    #[pg_test(error = "pletamil: the result 1.5 is not a whole number that fits integer")]
    fn a_result_that_does_not_fit_the_declared_type_is_an_error() {
        Spi::run("CREATE FUNCTION frac() RETURNS integer LANGUAGE pletamil AS $$ திரும்பு 1.5; $$").unwrap();
        let _ = Spi::get_one::<i32>("SELECT frac()");
    }

    #[pg_test(error = "pletamil: the function returns integer, but its body returned text")]
    fn a_result_of_the_wrong_kind_is_an_error() {
        Spi::run("CREATE FUNCTION wrong() RETURNS integer LANGUAGE pletamil AS $$ திரும்பு \"abc\"; $$").unwrap();
        let _ = Spi::get_one::<i32>("SELECT wrong()");
    }
}

/// Required by `cargo pgrx test`; it must be visible at the root of the crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
