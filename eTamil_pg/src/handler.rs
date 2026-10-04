// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! The call handler of the language: what PostgreSQL runs for `LANGUAGE pletamil`.
//!
//! A function's *body* is the eTamil, and its argument names are the eTamil's variables:
//!
//! ```sql
//! CREATE FUNCTION gst_total(amount numeric, rate numeric) RETURNS numeric LANGUAGE pletamil AS $$
//!     திரும்பு amount * (1 + rate / 100);
//! $$;
//! ```
//!
//! Everything about running the body, and about refusing one that reaches outside the database,
//! is `etamil_pg_eval::function`, which is tested without PostgreSQL. This file is the part that
//! has to be PostgreSQL: reading the function's definition from the catalog, turning each
//! argument into an eTamil value, and the result back into a PostgreSQL one.
//!
//! The definition is read from `pg_proc` on every call, not cached: `CREATE OR REPLACE FUNCTION`
//! keeps the function's OID, so a cache keyed on it would go on running the old body. A cache
//! would need invalidating on a catalog change, which is its own piece of work.

use std::str::FromStr;

use etamil_pg_eval::function;
use etamil_pg_eval::Value;
use pgrx::callconv::{BoxRet, FcInfo};
use pgrx::datum::Datum;
use pgrx::fcinfo::pg_getarg;
use pgrx::pg_sys;
use pgrx::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/// `pgrx` cannot write the `CREATE FUNCTION` for a function whose only argument is the raw call
/// info, so it is written here, as PL/Rust does it.
#[pg_extern(sql = "
CREATE FUNCTION pletamil_call_handler() RETURNS language_handler
    LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
unsafe fn pletamil_call_handler(fcinfo: pg_sys::FunctionCallInfo) -> Reply {
    // SAFETY: PostgreSQL gives a call handler a valid call info, with its function info set.
    let fn_oid = unsafe { (*(*fcinfo).flinfo).fn_oid };
    match unsafe { call_function(fn_oid, fcinfo) } {
        Ok(datum) => datum,
        Err(message) => error!("{}", message),
    }
}

/// Run by `CREATE FUNCTION`, so a body that cannot run (a syntax error, a forbidden builtin, a bad
/// argument name) is refused when it is written, not when it is first called.
///
/// PostgreSQL calls a validator before the new `pg_proc` row is visible to a query, and the
/// definition is read with queries, so the command counter is advanced first. Skipped when
/// `check_function_bodies` is off, which is how `pg_dump` output asks for a restore that does not
/// re-check every body.
#[pg_extern(sql = "
CREATE FUNCTION pletamil_validator(oid) RETURNS void
    LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
fn pletamil_validator(fn_oid: pg_sys::Oid) {
    // SAFETY: reading a plain configuration flag, and advancing the command counter in a backend.
    if !unsafe { pg_sys::check_function_bodies } {
        return;
    }
    unsafe { pg_sys::CommandCounterIncrement() };
    let outcome = read_definition(fn_oid).and_then(|d| function::prepare(&d.names, &d.body).map(|_| ()));
    if let Err(message) = outcome {
        error!("{}", message);
    }
}

/// What the handler hands back: a datum, or `None` for SQL NULL. `pgrx` 0.19 only lets a function
/// return a type it knows how to box, so the raw datum is wrapped in one.
struct Reply(Option<pg_sys::Datum>);

// SAFETY: the datum is one `result` made for the function's declared return type.
unsafe impl BoxRet for Reply {
    unsafe fn box_into<'fcx>(self, fcinfo: &mut FcInfo<'fcx>) -> Datum<'fcx> {
        unsafe { fcinfo.return_optional_datum(self.0) }
    }
}

// The hand-written `CREATE FUNCTION` above decides the SQL; this only satisfies the type check.
pgrx::pgrx_sql_entity_graph::metadata::impl_sql_translatable!(Reply, "language_handler");

extension_sql!(
    r#"
CREATE LANGUAGE pletamil HANDLER pletamil_call_handler VALIDATOR pletamil_validator;
COMMENT ON LANGUAGE pletamil IS 'PL/eTamil: eTamil functions that only compute (a spike)';
"#,
    name = "pletamil_language",
    requires = [pletamil_call_handler, pletamil_validator]
);

/// What `pg_proc` says about a function.
struct Definition {
    body: String,
    names: Vec<String>,
    types: Vec<String>,
    returns: String,
}

fn ask<T: FromDatum + IntoDatum>(oid: u32, what: &str) -> Result<Option<T>, String> {
    Spi::get_one::<T>(&format!("SELECT {what} FROM pg_proc WHERE oid = {oid}")).map_err(|e| e.to_string())
}

fn read_definition(fn_oid: pg_sys::Oid) -> Result<Definition, String> {
    let oid = fn_oid.to_u32();

    // A set-returning function, an OUT or INOUT argument, and VARIADIC each change what a call
    // returns or takes; none is supported, and a clear refusal beats a wrong answer.
    if ask::<bool>(oid, "proretset OR proargmodes IS NOT NULL OR provariadic <> 0")?.unwrap_or(false) {
        return Err("pletamil: set-returning, OUT, INOUT and VARIADIC functions are not supported".into());
    }

    let body = ask::<String>(oid, "prosrc")?.ok_or("pletamil: the function has no body")?;
    let returns = ask::<String>(oid, "format_type(prorettype, NULL)")?.ok_or("pletamil: the function has no return type")?;

    // proargtypes is an oidvector; unnest it in order, and ask PostgreSQL to name each type.
    let types = Spi::get_one::<Vec<Option<String>>>(&format!(
        "SELECT array_agg(format_type(t, NULL) ORDER BY o) \
         FROM pg_proc p, unnest(p.proargtypes::oid[]) WITH ORDINALITY AS u(t, o) WHERE p.oid = {oid}"
    ))
    .map_err(|e| e.to_string())?
    .unwrap_or_default()
    .into_iter()
    .map(|t| t.unwrap_or_default())
    .collect::<Vec<_>>();

    // An argument with no name is `arg1`, `arg2`, ... by position.
    let declared = ask::<Vec<Option<String>>>(oid, "proargnames")?.unwrap_or_default();
    let names = (0..types.len())
        .map(|i| match declared.get(i) {
            Some(Some(name)) if !name.is_empty() => name.clone(),
            _ => format!("arg{}", i + 1),
        })
        .collect();

    Ok(Definition { body, names, types, returns })
}

unsafe fn call_function(fn_oid: pg_sys::Oid, fcinfo: pg_sys::FunctionCallInfo) -> Result<Reply, String> {
    let definition = read_definition(fn_oid)?;
    let function = function::prepare(&definition.names, &definition.body)?;

    let mut args = Vec::with_capacity(definition.types.len());
    for (index, ty) in definition.types.iter().enumerate() {
        args.push(unsafe { argument(fcinfo, index, ty)? });
    }

    let value = function::call(&function, args)?;
    result(&definition.returns, value)
}

/// Argument `index` as an eTamil value. A SQL NULL is eTamil's null.
unsafe fn argument(fcinfo: pg_sys::FunctionCallInfo, index: usize, ty: &str) -> Result<Value, String> {
    let number = |text: String| {
        Decimal::from_str(&text)
            .map(Value::Number)
            .map_err(|_| format!("pletamil: the number {text} cannot be used (NaN, and values beyond 28 digits, are not supported)"))
    };
    Ok(match ty {
        "numeric" => match unsafe { pg_getarg::<AnyNumeric>(fcinfo, index) } {
            None => Value::Null,
            Some(n) => number(n.to_string())?,
        },
        "integer" => unsafe { pg_getarg::<i32>(fcinfo, index) }.map_or(Value::Null, |n| Value::Number(Decimal::from(n))),
        "bigint" => unsafe { pg_getarg::<i64>(fcinfo, index) }.map_or(Value::Null, |n| Value::Number(Decimal::from(n))),
        "smallint" => unsafe { pg_getarg::<i16>(fcinfo, index) }.map_or(Value::Null, |n| Value::Number(Decimal::from(n))),
        "double precision" => match unsafe { pg_getarg::<f64>(fcinfo, index) } {
            None => Value::Null,
            Some(f) => Decimal::try_from(f).map(Value::Number).map_err(|_| format!("pletamil: the number {f} cannot be used"))?,
        },
        "real" => match unsafe { pg_getarg::<f32>(fcinfo, index) } {
            None => Value::Null,
            Some(f) => Decimal::try_from(f).map(Value::Number).map_err(|_| format!("pletamil: the number {f} cannot be used"))?,
        },
        "text" | "character varying" => unsafe { pg_getarg::<String>(fcinfo, index) }.map_or(Value::Null, Value::String),
        "boolean" => unsafe { pg_getarg::<bool>(fcinfo, index) }.map_or(Value::Null, Value::Boolean),
        other => return Err(format!("pletamil: argument {} has type {other}, which is not supported", index + 1)),
    })
}

/// A whole number that fits `T`, or the error that says why not.
fn whole(value: &Decimal, ty: &str) -> Result<i64, String> {
    value
        .to_i64()
        .filter(|i| Decimal::from(*i) == *value)
        .ok_or_else(|| format!("pletamil: the result {value} is not a whole number that fits {ty}"))
}

/// The function's value as a PostgreSQL datum of the declared return type.
fn result(ty: &str, value: Value) -> Result<Reply, String> {
    let value = match value {
        Value::Ok(inner) => *inner,
        Value::Err(inner) => return Err(format!("pletamil: the function returned a தவறு: {inner}")),
        other => other,
    };
    if matches!(value, Value::Null) {
        return Ok(Reply(None));
    }

    let datum = match (ty, &value) {
        ("numeric", Value::Number(d)) => AnyNumeric::from_str(&d.normalize().to_string())
            .map_err(|e| format!("pletamil: the result {d} is not a number PostgreSQL accepts: {e}"))?
            .into_datum(),
        ("integer", Value::Number(d)) => i32::try_from(whole(d, ty)?).map_err(|_| format!("pletamil: the result {d} is not a whole number that fits {ty}"))?.into_datum(),
        ("bigint", Value::Number(d)) => whole(d, ty)?.into_datum(),
        ("smallint", Value::Number(d)) => i16::try_from(whole(d, ty)?).map_err(|_| format!("pletamil: the result {d} is not a whole number that fits {ty}"))?.into_datum(),
        ("double precision", Value::Number(d)) => d.to_f64().ok_or_else(|| format!("pletamil: the result {d} does not fit {ty}"))?.into_datum(),
        ("real", Value::Number(d)) => d.to_f32().ok_or_else(|| format!("pletamil: the result {d} does not fit {ty}"))?.into_datum(),
        ("text" | "character varying", Value::String(s)) => s.clone().into_datum(),
        ("boolean", Value::Boolean(b)) => (*b).into_datum(),
        ("numeric" | "integer" | "bigint" | "smallint" | "double precision" | "real" | "text" | "character varying" | "boolean", other) => {
            return Err(format!("pletamil: the function returns {ty}, but its body returned {}", describe(other)));
        }
        (other, _) => return Err(format!("pletamil: the return type {other} is not supported")),
    };
    Ok(Reply(datum))
}

fn describe(value: &Value) -> &'static str {
    match value {
        Value::Number(_) => "a number",
        Value::String(_) => "text",
        Value::Boolean(_) => "a boolean",
        Value::Array(_) => "an array",
        Value::Map(_) => "a record",
        Value::Function(_) => "a function",
        _ => "a value of another kind",
    }
}
