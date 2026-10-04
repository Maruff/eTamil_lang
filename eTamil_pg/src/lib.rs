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
