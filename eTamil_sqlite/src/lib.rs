// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Call eTamil functions from SQLite queries.
//!
//! ```text
//! SELECT id, gst_total(amount, 18) FROM invoices WHERE gst_total(amount, 18) > 500
//! ```
//!
//! This is a spike of design A in `docs/architecture/DATABASE_EXTENSIONS.md`: functions
//! registered on a connection, not a loadable extension. It uses only the compiler's public
//! API, so it changes nothing in the compiler, and answers the questions that decide whether
//! changing it is worth doing.
//!
//! How it works. A program (only function and shape definitions) is parsed, checked and
//! passed through the allowlist from the PL/eTamil spike, so a SQL function cannot touch the
//! outside world. For each function it is to expose, a wrapper program is compiled once:
//! the definitions, then `sql_result = f(sql_arg0, sql_arg1);`. Each call from SQLite builds
//! a fresh VM, binds the arguments as variables, runs the wrapper, and reads `sql_result`:
//! the pattern the compiler's HTTP workers use for a request.
//!
//! What a value looks like on the way through is the compiler's own rule, not a new one: a
//! decimal crosses as exact text, and text that parses as a decimal comes back as a number.

use std::fmt;
use std::str::FromStr;

use etamil_compiler::check;
use etamil_compiler::lexer;
use etamil_compiler::parser::{Parser, Stmt};
use etamil_compiler::vm::{host, Bytecode, BytecodeCompiler, Value, VM};
use rusqlite::functions::{Context, FunctionFlags};
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::Connection;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/// How a decimal result is handed back to SQLite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Numbers {
    /// As exact text, `'295.59'`: no precision is lost, which is why the compiler's own
    /// database statements do it. **But SQLite orders TEXT after every number**, so
    /// `WHERE gst_total(x, 18) > 500` is true for every row, and `SUM(...)` converts to
    /// floating point. Use `CAST(... AS REAL)` where the comparison matters.
    #[default]
    ExactText,
    /// As an INTEGER when the value is a whole number, otherwise as a REAL. Comparisons, `ORDER
    /// BY` and `SUM` then behave as a reader expects, at the cost of floating-point
    /// representation for a fractional result.
    Native,
}

/// One eTamil function to expose to SQL.
#[derive(Debug, Clone)]
pub struct SqlFunction<'a> {
    /// The name SQL calls it by. Tamil names work: SQLite accepts them unquoted.
    pub sql_name: &'a str,
    /// The `செயல்` in the source.
    pub etamil_name: &'a str,
    /// How many arguments it takes.
    pub arity: usize,
    pub numbers: Numbers,
}

impl<'a> SqlFunction<'a> {
    pub fn new(sql_name: &'a str, etamil_name: &'a str, arity: usize) -> Self {
        SqlFunction { sql_name, etamil_name, arity, numbers: Numbers::default() }
    }

    pub fn numeric(mut self) -> Self {
        self.numbers = Numbers::Native;
        self
    }
}

/// Instructions one call may retire: a function called per row must be cheap, so this is a
/// hundredth of the browser's limit.
pub const STEP_LIMIT: u64 = 100_000;

/// Why a source or a function could not be registered.
#[derive(Debug, PartialEq)]
pub struct RegisterError(pub String);

impl fmt::Display for RegisterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RegisterError {}

fn refuse<T>(message: impl Into<String>) -> Result<T, RegisterError> {
    Err(RegisterError(message.into()))
}

/// Register `functions` from `source` on `connection`. Nothing is registered unless every
/// function can be.
pub fn register(connection: &Connection, source: &str, functions: &[SqlFunction<'_>]) -> Result<(), RegisterError> {
    let statements = parse(source)?;

    // The same rule as a database function in PostgreSQL: it may only compute.
    etamil_pg_eval::require_pure(&statements).map_err(RegisterError)?;
    if let Err(errors) = check::check(&statements) {
        let first = errors.first().map(|e| e.to_string()).unwrap_or_else(|| "type error".into());
        return refuse(first);
    }

    // A library of definitions, not a program: a top-level statement would run once per
    // call, or never, and either is a surprise.
    if let Some(other) = statements.iter().find(|s| !matches!(s, Stmt::FunctionDef { .. } | Stmt::ShapeDef { .. })) {
        let kind = format!("{other:?}");
        let kind = kind.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("statement");
        return refuse(format!("only function and shape definitions may appear at the top level of a SQL function's source, not a {kind}"));
    }

    // Everything is built before anything is registered, so a mistake in the third function
    // does not leave two registered.
    let mut prepared = Vec::with_capacity(functions.len());
    for function in functions {
        let arity = statements.iter().find_map(|s| match s {
            Stmt::FunctionDef { name, params, .. } if name == function.etamil_name => Some(params.len()),
            _ => None,
        });
        match arity {
            None => return refuse(format!("the source defines no function named {}", function.etamil_name)),
            Some(n) if n != function.arity => {
                return refuse(format!("{} takes {n} argument(s), not {}", function.etamil_name, function.arity));
            }
            Some(_) => {}
        }
        prepared.push((function, wrapper(&statements, function)?));
    }

    for (function, bytecode) in prepared {
        let mode = function.numbers;
        connection
            .create_scalar_function(function.sql_name, function.arity as i32, FunctionFlags::SQLITE_UTF8, move |ctx| {
                call(&bytecode, ctx, mode)
            })
            .map_err(|e| RegisterError(format!("SQLite refused {}: {e}", function.sql_name)))?;
    }
    Ok(())
}

fn parse(source: &str) -> Result<Vec<Stmt>, RegisterError> {
    let tokens = lexer::tokenize(source)
        .map_err(|errors| RegisterError(errors.first().map(|e| e.to_string()).unwrap_or_else(|| "lexical error".into())))?;
    Parser::new(tokens.iter()).parse().map_err(|e| RegisterError(e.to_string()))
}

/// The definitions, then the call that leaves its answer in `sql_result`.
fn wrapper(definitions: &[Stmt], function: &SqlFunction<'_>) -> Result<Bytecode, RegisterError> {
    let arguments: Vec<String> = (0..function.arity).map(|i| format!("sql_arg{i}")).collect();
    let mut program = definitions.to_vec();
    program.extend(parse(&format!("sql_result = {}({});", function.etamil_name, arguments.join(", ")))?);
    Ok(BytecodeCompiler::compile_statements(program))
}

fn user_error(message: String) -> rusqlite::Error {
    rusqlite::Error::UserFunctionError(message.into())
}

/// One call from SQLite: a fresh VM, so no call sees another's variables.
fn call(bytecode: &Bytecode, ctx: &Context<'_>, mode: Numbers) -> rusqlite::Result<SqlValue> {
    let mut vm = VM::new();
    for index in 0..ctx.len() {
        vm.variables.insert(format!("sql_arg{index}"), from_sql(ctx.get_raw(index)));
    }

    // A function that prints has nowhere useful to print to; capture it and drop it.
    host::begin_capture();
    let outcome = vm.execute_limited(bytecode.clone(), STEP_LIMIT);
    host::end_capture();
    outcome.map_err(user_error)?;

    let result = vm.variables.remove("sql_result").unwrap_or(Value::Null);
    into_sql(result, mode).map_err(user_error)
}

// --- Values, as the compiler's own SQLite adapter treats them (src/db/sqlite.rs) ---------

/// An argument coming in: text that parses as a decimal is a number, and a whole number
/// stays whole.
pub fn from_sql(raw: ValueRef<'_>) -> Value {
    match raw {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::Number(Decimal::from(i)),
        ValueRef::Text(bytes) => {
            let text = String::from_utf8_lossy(bytes).to_string();
            match Decimal::from_str(&text) {
                Ok(number) => Value::Number(number),
                Err(_) => Value::String(text),
            }
        }
        ValueRef::Real(f) => Decimal::from_str(&f.to_string()).map(Value::Number).unwrap_or(Value::Null),
        ValueRef::Blob(bytes) => Value::String(String::from_utf8_lossy(bytes).to_string()),
    }
}

/// A result going out. A collection or a function has no SQL form, and says so.
pub fn into_sql(value: Value, mode: Numbers) -> Result<SqlValue, String> {
    Ok(match value {
        Value::Number(n) => match mode {
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
        Value::Ok(inner) => return into_sql(*inner, mode),
        Value::Err(inner) => return Err(format!("the function returned a தவறு: {inner}")),
        other => return Err(format!("a SQL function cannot return {}: it has no SQL form", kind_of(&other))),
    })
}

fn kind_of(value: &Value) -> &'static str {
    match value {
        Value::Array(_) => "an array",
        Value::Map(_) => "a record",
        Value::Function(_) => "a function",
        _ => "that value",
    }
}

#[cfg(test)]
mod tests;
