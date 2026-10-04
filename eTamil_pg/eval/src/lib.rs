// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Run an eTamil program that may not touch the outside world.
//!
//! This is the part of PL/eTamil that needs no PostgreSQL, so it builds and is tested
//! anywhere. The extension (`../src/lib.rs`) calls it.
//!
//! "May not touch the outside world" is enforced before anything runs, by walking the
//! program: only statements that compute are allowed, and a call to a builtin is allowed
//! only if the build script (build.rs) found it on the list of pure ones. A statement kind
//! or a builtin that the compiler gains later is therefore refused until someone reads it.
//!
//! It is an allowlist over the syntax tree, not a sandbox: it does not limit memory, and
//! the step limit does not bound a single builtin that is itself expensive. See the README.

use etamil_compiler::check;
use etamil_compiler::lexer;
use etamil_compiler::parser::{Expr, Parser, Stmt};
use etamil_compiler::vm::{self, host};

include!(concat!(env!("OUT_DIR"), "/allowed_builtins.rs"));

pub mod function;
pub use etamil_compiler::vm::Value;

/// Instructions one call may retire. The browser allows ten million; a database
/// function runs inside someone's query, so it gets a tenth of that.
pub const STEP_LIMIT: u64 = 1_000_000;

/// What one run produced.
#[derive(Debug, PartialEq)]
pub struct Outcome {
    pub ok: bool,
    /// Everything `அச்சு` printed.
    pub output: String,
    /// The failure, from whichever stage produced it.
    pub error: Option<String>,
    /// "load", "pure", "type" or "run".
    pub stage: Option<&'static str>,
}

fn refused(stage: &'static str, error: String) -> Outcome {
    Outcome { ok: false, output: String::new(), error: Some(error), stage: Some(stage) }
}

/// Compile and run `source`, if it only computes.
pub fn run(source: &str) -> Outcome {
    let tokens = match lexer::tokenize(source) {
        Ok(tokens) => tokens,
        Err(errors) => {
            let first = errors.first().map(|e| e.to_string()).unwrap_or_else(|| "lexical error".into());
            return refused("load", first);
        }
    };
    let statements = match Parser::new(tokens.iter()).parse() {
        Ok(statements) => statements,
        Err(e) => return refused("load", e.to_string()),
    };
    if let Err(why) = require_pure(&statements) {
        return refused("pure", why);
    }
    if let Err(errors) = check::check(&statements) {
        let first = errors.first().map(|e| e.to_string()).unwrap_or_else(|| "type error".into());
        return refused("type", first);
    }

    let bytecode = vm::compile_to_bytecode(statements);
    let mut machine = vm::VM::new();

    // Capture on before the first `அச்சு`, and off again on every path out: the buffer
    // is per thread, and leaving it on would swallow the next caller's output.
    host::begin_capture();
    let result = machine.execute_limited(bytecode, STEP_LIMIT);
    let output = host::end_capture();

    match result {
        Ok(()) => Outcome { ok: true, output, error: None, stage: None },
        Err(e) => Outcome { ok: false, output, error: Some(e), stage: Some("run") },
    }
}

/// The text of `expression`, as a database function would return it.
pub fn eval_expression(expression: &str) -> Result<String, String> {
    let outcome = run(&format!("அச்சு({});", expression.trim().trim_end_matches(';')));
    if outcome.ok { Ok(outcome.output.trim_end_matches('\n').to_string()) } else { Err(outcome.error.unwrap_or_default()) }
}

// --- The allowlist ---------------------------------------------------------------

/// Is `name` a builtin that is not on the pure list?
fn forbidden_builtin(name: &str) -> bool {
    vm::is_builtin(name) && ALLOWED_BUILTINS.binary_search(&name).is_err()
}

/// Walk the program; the first thing that could reach outside it is the error.
pub fn require_pure(statements: &[Stmt]) -> Result<(), String> {
    statements.iter().try_for_each(stmt)
}

fn block(statements: &[Stmt]) -> Result<(), String> {
    statements.iter().try_for_each(stmt)
}

fn stmt(s: &Stmt) -> Result<(), String> {
    match s {
        Stmt::Assign { value, .. } => expr(value),
        Stmt::FunctionDef { body, .. } => block(body),
        Stmt::ShapeDef { methods, .. } => methods.iter().try_for_each(|m| block(&m.body)),
        Stmt::Return(value) => value.as_ref().map_or(Ok(()), expr),
        Stmt::SetIndex { index, value, .. } => expr(index).and_then(|_| expr(value)),
        Stmt::SetField { value, .. } => expr(value),
        Stmt::Expression(e) | Stmt::Print(e) => expr(e),
        Stmt::If { condition, then_branch, else_branch } => {
            expr(condition)?;
            block(then_branch)?;
            else_branch.as_deref().map_or(Ok(()), block)
        }
        Stmt::Loop { condition, body } => expr(condition).and_then(|_| block(body)),
        Stmt::ForEach { collection, body, .. } => expr(collection).and_then(|_| block(body)),
        // Everything else: imports, input, files, databases, routes and servers, and any
        // statement kind added later. Refused by default.
        other => Err(format!("a database function may not use this statement ({})", kind(other))),
    }
}

/// The variant's name, for the message.
fn kind(s: &Stmt) -> String {
    let text = format!("{s:?}");
    text.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("statement").to_string()
}

fn exprs(list: &[Expr]) -> Result<(), String> {
    list.iter().try_for_each(expr)
}

fn expr(e: &Expr) -> Result<(), String> {
    match e {
        Expr::Number(_) | Expr::String(_) | Expr::Boolean(_) | Expr::Null => Ok(()),
        // A builtin used as a value could be called later (`f = _env; f("X")`), so
        // naming a forbidden one at all is refused, not only calling it.
        Expr::Variable(name) => not_forbidden(name),
        Expr::BinaryOp { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Concat { left, right }
        | Expr::Logical { left, right, .. } => expr(left).and_then(|_| expr(right)),
        Expr::Not(inner) | Expr::Try(inner) => expr(inner),
        Expr::Call { name, args } => not_forbidden(name).and_then(|_| exprs(args)),
        Expr::ArrayLiteral(items) => exprs(items),
        Expr::RecordLiteral(fields) => fields.iter().try_for_each(|(_, v)| expr(v)),
        Expr::Index { base, index } => expr(base).and_then(|_| expr(index)),
        Expr::Field { base, .. } => expr(base),
        Expr::MethodCall { receiver, args, .. } => expr(receiver).and_then(|_| exprs(args)),
        Expr::ShapeLiteral { fields, base, .. } => {
            fields.iter().try_for_each(|(_, v)| expr(v))?;
            base.as_deref().map_or(Ok(()), expr)
        }
        Expr::Lambda { body, .. } => block(body),
        Expr::CallValue { callee, args } => expr(callee).and_then(|_| exprs(args)),
    }
}

fn not_forbidden(name: &str) -> Result<(), String> {
    if forbidden_builtin(name) {
        Err(format!("a database function may not use `{name}`: it reaches outside the database"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
