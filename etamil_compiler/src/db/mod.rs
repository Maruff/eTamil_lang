// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Database access.
//!
//! The trait and everything around it are plain Rust with no driver
//! dependency, so the VM wiring, parameter binding and row conversion can be
//! tested against a stand-in backend. Real drivers live in submodules behind
//! cargo features.
//!
//! Queries are always parameterised. There is deliberately no way to splice
//! a value into the SQL text from eTamil: for a language aimed at financial
//! systems, the first example anyone copies should not contain an injection.

use crate::vm::Value;

pub mod pool;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "mysql")]
pub mod mysql;

/// One open connection.
pub trait Database: Send {
    /// Run a statement that returns no rows. Yields the number affected.
    fn execute(&mut self, sql: &str, params: &[Value]) -> Result<i64, String>;

    /// Run a query. Yields one record (பொருள்) per row, so a result set is
    /// an array of records — a table in the language's own terms.
    fn query(&mut self, sql: &str, params: &[Value]) -> Result<Vec<Value>, String>;

    /// Release the connection. Dropping is expected to do this too.
    fn close(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Let queries on this connection call `function` by `name`, once per row.
    ///
    /// Only a driver whose database can call back into the program says yes; the
    /// rest keep this default. Registering a name again replaces the function.
    /// `arity` is the number of arguments, or -1 for any number.
    fn register_function(
        &mut self,
        name: &str,
        arity: i32,
        numbers: Numbers,
        function: RowFunction,
    ) -> Result<(), String> {
        let _ = (name, arity, numbers, function);
        Err("இந்தத் தரவுத்தளத்தில் செயல்களைப் பதிவு செய்ய முடியாது  (this database cannot have functions registered on it)"
            .to_string())
    }

    /// Forget every function registered on this connection. A pooled connection
    /// is lent to one request after another, so a function registered for one
    /// must not still be there, running that request's program, for the next.
    fn unregister_functions(&mut self) {}
}

/// A function a database calls once per row, with the row's arguments.
///
/// It owns what it needs (the program it runs, the function to call), because the
/// database keeps it for as long as the connection lives.
pub type RowFunction = Box<dyn FnMut(&[Value]) -> Result<Value, String> + Send + 'static>;

/// How a number returned from such a function goes into the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Numbers {
    /// As exact text: no digit is lost, and it is how the compiler's own SQLite
    /// adapter stores a decimal. SQLite orders text after every number, so
    /// `WHERE f(x) > 500` is true of every row; compare with `CAST(f(x) AS REAL)`.
    ExactText,
    /// As an INTEGER when whole and a REAL otherwise, so comparisons, `ORDER BY` and
    /// `SUM` behave. A decimal that is not whole becomes a float.
    Native,
}

/// Open a connection for a database type as named in eTamil source.
///
/// `db_type` arrives as the parser's token name — "SQLite", "PostgreSQL" and
/// so on — not as the user's spelling.
pub fn open(db_type: &str, connection: &str) -> Result<Box<dyn Database>, String> {
    match db_type {
        #[cfg(feature = "sqlite")]
        "SQLite" | "SQL" => Ok(Box::new(sqlite::SqliteDatabase::open(connection)?)),

        #[cfg(not(feature = "sqlite"))]
        "SQLite" | "SQL" => Err(format!(
            "SQLite ஆதரவு இல்லாமல் கட்டப்பட்டது  \
             (this build has no SQLite support): rebuild with --features sqlite"
        )),

        #[cfg(feature = "postgres")]
        "PostgreSQL" => Ok(Box::new(postgres::PostgresDatabase::open(connection)?)),

        #[cfg(not(feature = "postgres"))]
        "PostgreSQL" => Err("போச்குரசீகுல் ஆதரவு இல்லாமல் கட்டப்பட்டது  \
             (this build has no PostgreSQL support): rebuild with --features postgres"
            .to_string()),

        #[cfg(feature = "mysql")]
        "MySQL" => Ok(Box::new(mysql::MysqlDatabase::open(connection)?)),

        #[cfg(not(feature = "mysql"))]
        "MySQL" => Err("மைசீகுல் ஆதரவு இல்லாமல் கட்டப்பட்டது  \
             (this build has no MySQL support): rebuild with --features mysql"
            .to_string()),

        "MongoDB" | "Redis" | "JSONdb" | "NoSQL" => Err(format!(
            "{} இன்னும் ஆதரிக்கப்படவில்லை  ({} is not supported yet); \
             SQLite, PostgreSQL and MySQL are the backends today",
            db_type, db_type
        )),

        other => Err(format!(
            "அறியப்படாத தரவுத்தள வகை '{}'  (unknown database type '{}')",
            other, other
        )),
    }
}

/// Convert an eTamil array of parameters into a slice the drivers can bind.
/// Anything that is not an array is a mistake worth naming clearly, since
/// the syntax always requires one.
pub fn params_from(value: &Value) -> Result<Vec<Value>, String> {
    match value {
        Value::Array(items) => Ok(items.clone()),
        other => Err(format!(
            "அளவுருக்கள் ஒரு அணியாக இருக்க வேண்டும்  \
             (query parameters must be an array, got {})",
            match other {
                Value::Number(_) => "a number",
                Value::String(_) => "a string",
                Value::Boolean(_) => "a boolean",
                Value::Map(_) => "a record",
                Value::Ok(_) | Value::Err(_) => "a result",
                Value::Function(_) => "a function",
                Value::Null => "nil",
                Value::Array(_) => unreachable!(),
            }
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A driver that says nothing about functions, as PostgreSQL and MySQL do.
    struct Plain;

    impl Database for Plain {
        fn execute(&mut self, _sql: &str, _params: &[Value]) -> Result<i64, String> {
            Ok(0)
        }
        fn query(&mut self, _sql: &str, _params: &[Value]) -> Result<Vec<Value>, String> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn a_driver_that_does_not_support_functions_says_so() {
        let mut db = Plain;
        let error = db
            .register_function(
                "moqqam_vari",
                2,
                Numbers::ExactText,
                Box::new(|_: &[Value]| Ok(Value::Null)),
            )
            .unwrap_err();
        assert!(
            error.contains("cannot have functions registered"),
            "{error}"
        );
        db.unregister_functions(); // the default does nothing, and must not panic
    }
}
