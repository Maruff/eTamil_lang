// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! SQLite backend, using the blocking rusqlite driver.
//!
//! Blocking rather than async is deliberate: the VM is synchronous and the
//! HTTP server runs a thread per request, so a blocking driver fits without
//! adding yield points to every I/O instruction.

use rusqlite::types::{ToSqlOutput, Value as SqlValue, ValueRef};
use rusqlite::{Connection, ToSql};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;

use rust_decimal::prelude::ToPrimitive;

use super::{Database, Numbers, RowFunction};
use crate::vm::Value;

pub struct SqliteDatabase {
    connection: Connection,
    /// Functions registered on this connection, by name and arity, so they can be removed.
    functions: Vec<(String, i32)>,
}

impl SqliteDatabase {
    /// Open a database file, or an in-memory one for ":memory:".
    pub fn open(path: &str) -> Result<Self, String> {
        let connection = if path == ":memory:" {
            Connection::open_in_memory()
        } else {
            Connection::open(path)
        }
        .map_err(|e| {
            format!(
                "தரவுத்தளம் திறக்க முடியவில்லை  (cannot open database '{}'): {}",
                path, e
            )
        })?;

        Ok(SqliteDatabase {
            connection,
            functions: Vec::new(),
        })
    }
}

/// An eTamil value on its way into a bound parameter.
struct Bound<'a>(&'a Value);

impl ToSql for Bound<'_> {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(match self.0 {
            // Decimals go over as text so no precision is lost on the way;
            // SQLite has no exact decimal type, and REAL would defeat the
            // point of using decimals in the first place.
            Value::Number(n) => ToSqlOutput::Owned(SqlValue::Text(n.normalize().to_string())),
            Value::String(s) => ToSqlOutput::Owned(SqlValue::Text(s.clone())),
            Value::Boolean(b) => ToSqlOutput::Owned(SqlValue::Integer(i64::from(*b))),
            Value::Null => ToSqlOutput::Owned(SqlValue::Null),
            other => ToSqlOutput::Owned(SqlValue::Text(other.to_string())),
        })
    }
}

/// A column coming back out.
fn value_from(raw: ValueRef<'_>) -> Value {
    match raw {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::Number(Decimal::from(i)),
        // Text that parses as a decimal comes back as a number, which is what
        // makes the text round-trip above lossless.
        ValueRef::Text(bytes) => {
            let text = String::from_utf8_lossy(bytes).to_string();
            match Decimal::from_str(&text) {
                Ok(number) => Value::Number(number),
                Err(_) => Value::String(text),
            }
        }
        ValueRef::Real(f) => Decimal::from_str(&f.to_string())
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ValueRef::Blob(bytes) => Value::String(String::from_utf8_lossy(bytes).to_string()),
    }
}

/// A result going out of a registered function. A collection or a function has no SQL
/// form, and says so.
fn into_sql(value: Value, numbers: Numbers) -> Result<SqlValue, String> {
    Ok(match value {
        Value::Number(n) => match numbers {
            Numbers::ExactText => SqlValue::Text(n.normalize().to_string()),
            Numbers::Native => {
                let n = n.normalize();
                match n.to_i64().filter(|i| Decimal::from(*i) == n) {
                    Some(whole) => SqlValue::Integer(whole),
                    None => SqlValue::Real(n.to_f64().ok_or("the number does not fit a REAL")?),
                }
            }
        },
        Value::String(s) => SqlValue::Text(s),
        Value::Boolean(b) => SqlValue::Integer(i64::from(b)),
        Value::Null => SqlValue::Null,
        // A சரி is its value; a தவறு is the function failing.
        Value::Ok(inner) => return into_sql(*inner, numbers),
        Value::Err(inner) => return Err(format!("the function returned a தவறு: {inner}")),
        Value::Array(_) => return Err("a SQL function cannot return an array".to_string()),
        Value::Map(_) => return Err("a SQL function cannot return a record".to_string()),
        Value::Function(_) => return Err("a SQL function cannot return a function".to_string()),
    })
}

impl Database for SqliteDatabase {
    fn execute(&mut self, sql: &str, params: &[Value]) -> Result<i64, String> {
        let bound: Vec<Bound<'_>> = params.iter().map(Bound).collect();
        let refs: Vec<&dyn ToSql> = bound.iter().map(|b| b as &dyn ToSql).collect();

        self.connection
            .execute(sql, refs.as_slice())
            .map(|affected| affected as i64)
            .map_err(|e| format!("தரவுத்தளப் பிழை  (database error): {}", e))
    }

    fn query(&mut self, sql: &str, params: &[Value]) -> Result<Vec<Value>, String> {
        let mut statement = self
            .connection
            .prepare(sql)
            .map_err(|e| format!("வினா தயாரிக்க முடியவில்லை  (cannot prepare query): {}", e))?;

        let column_names: Vec<String> = statement
            .column_names()
            .iter()
            .map(|c| c.to_string())
            .collect();

        let bound: Vec<Bound<'_>> = params.iter().map(Bound).collect();
        let refs: Vec<&dyn ToSql> = bound.iter().map(|b| b as &dyn ToSql).collect();

        let mut rows = statement
            .query(refs.as_slice())
            .map_err(|e| format!("தரவுத்தளப் பிழை  (database error): {}", e))?;

        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("வரிசை படிக்க முடியவில்லை  (cannot read row): {}", e))?
        {
            let mut record = HashMap::with_capacity(column_names.len());
            for (index, name) in column_names.iter().enumerate() {
                let raw = row.get_ref(index).map_err(|e| {
                    format!("நெடுவரிசை படிக்க முடியவில்லை  (cannot read column): {}", e)
                })?;
                record.insert(name.clone(), value_from(raw));
            }
            out.push(Value::Map(record.into()));
        }

        Ok(out)
    }

    fn register_function(
        &mut self,
        name: &str,
        arity: i32,
        numbers: Numbers,
        mut function: RowFunction,
    ) -> Result<(), String> {
        use rusqlite::functions::FunctionFlags;

        self.connection
            .create_scalar_function(name, arity, FunctionFlags::SQLITE_UTF8, move |context| {
                let arguments: Vec<Value> = (0..context.len())
                    .map(|index| value_from(context.get_raw(index)))
                    .collect();
                let result = function(&arguments).and_then(|value| into_sql(value, numbers));
                result.map_err(|message| rusqlite::Error::UserFunctionError(message.into()))
            })
            .map_err(|e| {
                format!(
                    "செயலைப் பதிவு செய்ய முடியவில்லை  (cannot register the function '{}'): {}",
                    name, e
                )
            })?;

        if !self.functions.iter().any(|(n, a)| n == name && *a == arity) {
            self.functions.push((name.to_string(), arity));
        }
        Ok(())
    }

    fn unregister_functions(&mut self) {
        for (name, arity) in self.functions.drain(..) {
            let _ = self.connection.remove_function(&name, arity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(text: &str) -> Value {
        Value::Number(Decimal::from_str(text).unwrap())
    }

    /// An in-memory database with three invoices: 1000, 250.50 and 100.
    fn with_invoices() -> SqliteDatabase {
        let mut db = SqliteDatabase::open(":memory:").unwrap();
        db.execute("CREATE TABLE paRRuccIttu (eN INTEGER, qokY TEXT)", &[])
            .unwrap();
        for (n, amount) in [(1, "1000"), (2, "250.50"), (3, "100")] {
            db.execute(
                "INSERT INTO paRRuccIttu VALUES (?, ?)",
                &[number(&n.to_string()), number(amount)],
            )
            .unwrap();
        }
        db
    }

    /// The amount with tax added: `qokY * (1 + vikiqam / 100)`.
    fn moqqam_vari() -> RowFunction {
        Box::new(|args: &[Value]| match args {
            [Value::Number(amount), Value::Number(rate)] => Ok(Value::Number(
                *amount * (Decimal::ONE + *rate / Decimal::from(100)),
            )),
            other => Err(format!("unexpected arguments: {other:?}")),
        })
    }

    fn totals(db: &mut SqliteDatabase) -> Vec<Value> {
        let rows = db
            .query(
                "SELECT moqqam_vari(qokY, 18) AS moqqam FROM paRRuccIttu ORDER BY eN",
                &[],
            )
            .unwrap();
        rows.into_iter()
            .map(|row| match row {
                Value::Map(record) => record.get("moqqam").cloned().unwrap(),
                other => panic!("a row should be a record, not {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_registered_function_is_called_once_per_row_and_stays_exact() {
        let mut db = with_invoices();
        db.register_function("moqqam_vari", 2, Numbers::ExactText, moqqam_vari())
            .unwrap();
        assert_eq!(
            totals(&mut db),
            vec![number("1180"), number("295.59"), number("118")]
        );
    }

    /// SQLite orders TEXT after every number, so an exact result compares as larger
    /// than any number. The reason `Numbers::Native` exists, pinned here so nobody
    /// changes the default thinking it a detail.
    #[test]
    fn an_exact_text_result_compares_as_larger_than_every_number() {
        let mut db = with_invoices();
        db.register_function("moqqam_vari", 2, Numbers::ExactText, moqqam_vari())
            .unwrap();
        let rows = db
            .query(
                "SELECT eN FROM paRRuccIttu WHERE moqqam_vari(qokY, 18) > 500",
                &[],
            )
            .unwrap();
        assert_eq!(rows.len(), 3, "every row passes, which is the trap");
    }

    #[test]
    fn a_native_result_compares_as_a_number() {
        let mut db = with_invoices();
        db.register_function("moqqam_vari", 2, Numbers::Native, moqqam_vari())
            .unwrap();
        let rows = db
            .query(
                "SELECT eN FROM paRRuccIttu WHERE moqqam_vari(qokY, 18) > 500",
                &[],
            )
            .unwrap();
        assert_eq!(rows.len(), 1, "only the 1000 invoice is over 500 with tax");
    }

    #[test]
    fn null_goes_in_and_comes_out() {
        let mut db = SqliteDatabase::open(":memory:").unwrap();
        db.register_function(
            "veRRu",
            1,
            Numbers::ExactText,
            Box::new(|args: &[Value]| Ok(args[0].clone())),
        )
        .unwrap();
        let rows = db.query("SELECT veRRu(NULL) AS x", &[]).unwrap();
        match &rows[0] {
            Value::Map(record) => assert_eq!(record.get("x"), Some(&Value::Null)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_error_in_the_function_is_a_sql_error_with_its_message() {
        let mut db = SqliteDatabase::open(":memory:").unwrap();
        db.register_function(
            "pizY",
            0,
            Numbers::ExactText,
            Box::new(|_: &[Value]| Err("the rate is missing".to_string())),
        )
        .unwrap();
        let error = db.query("SELECT pizY()", &[]).unwrap_err();
        assert!(error.contains("the rate is missing"), "{error}");
    }

    #[test]
    fn a_wrong_number_of_arguments_is_refused_by_sqlite() {
        let mut db = with_invoices();
        db.register_function("moqqam_vari", 2, Numbers::ExactText, moqqam_vari())
            .unwrap();
        assert!(db.query("SELECT moqqam_vari(1)", &[]).is_err());
    }

    #[test]
    fn a_result_with_no_sql_form_is_an_error() {
        let mut db = SqliteDatabase::open(":memory:").unwrap();
        db.register_function(
            "irattY",
            0,
            Numbers::ExactText,
            Box::new(|_: &[Value]| Ok(Value::Array(vec![]))),
        )
        .unwrap();
        let error = db.query("SELECT irattY()", &[]).unwrap_err();
        assert!(error.contains("array"), "{error}");
    }

    #[test]
    fn a_sari_is_its_value_and_a_thavaru_is_a_failure() {
        let mut db = SqliteDatabase::open(":memory:").unwrap();
        db.register_function(
            "cariY",
            0,
            Numbers::Native,
            Box::new(|_: &[Value]| Ok(Value::Ok(Box::new(number("7"))))),
        )
        .unwrap();
        db.register_function(
            "paZuY",
            0,
            Numbers::Native,
            Box::new(|_: &[Value]| Ok(Value::Err(Box::new(Value::String("no".into()))))),
        )
        .unwrap();
        let rows = db.query("SELECT cariY() = 7 AS ok", &[]).unwrap();
        match &rows[0] {
            Value::Map(record) => assert_eq!(record.get("ok"), Some(&number("1"))),
            other => panic!("{other:?}"),
        }
        assert!(db.query("SELECT paZuY()", &[]).is_err());
    }

    #[test]
    fn registering_a_name_again_replaces_the_function() {
        let mut db = SqliteDatabase::open(":memory:").unwrap();
        for answer in ["1", "2"] {
            let answer = number(answer);
            db.register_function(
                "veRRu",
                0,
                Numbers::Native,
                Box::new(move |_: &[Value]| Ok(answer.clone())),
            )
            .unwrap();
        }
        let rows = db.query("SELECT veRRu() AS x", &[]).unwrap();
        match &rows[0] {
            Value::Map(record) => assert_eq!(record.get("x"), Some(&number("2"))),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unregistering_removes_every_function() {
        let mut db = with_invoices();
        db.register_function("moqqam_vari", 2, Numbers::ExactText, moqqam_vari())
            .unwrap();
        assert!(db.query("SELECT moqqam_vari(1, 1)", &[]).is_ok());

        db.unregister_functions();

        let error = db.query("SELECT moqqam_vari(1, 1)", &[]).unwrap_err();
        assert!(error.contains("no such function"), "{error}");
    }
}
