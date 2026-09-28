// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! What an eTamil program is, as firmware: the type of every value, what runs
//! once, what runs on a timer, which functions are reached — and everything
//! that artino cannot build yet, named.
//!
//! A pure function of the AST, like `codegen::refusals`, so `--artino-gaps`
//! answers on any machine, LLVM or not.
//!
//! The program shape (docs/artino.md, "Program layout"):
//!
//! - statements at the top of the file run once, at power-on;
//! - `இடைவெளி N { … }` runs every N seconds, for ever; `இடைவெளி 0` every time
//!   round the loop;
//! - `செயல் சுழற்சி()`, if written, is called every time round the loop.
//!
//! State lives at the top level. As on the VM, a name assigned inside a
//! `செயல்` is that function's own local: a function cannot change the
//! program's variables, it returns a value that the top level or an
//! `இடைவெளி` block stores. The blocks are not functions, so they can.
//!
//! Every value has a size known now. Text holds up to `TEXT_BYTES - 1` bytes of
//! UTF-8; an array's length is part of its type, taken from its literal or from
//! `அணி_நிரப்பு`, so a board never allocates after power-on.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use super::manifest::{Extern, Kind};
use crate::parser::{DeclaredType, Expr, Stmt};

/// Bytes a text value occupies: 48 of UTF-8 and the NUL after them. The
/// runtime's `ARTINO_TEXT_BYTES` must agree.
pub const TEXT_BYTES: u32 = 49;

/// Bytes a single letter's buffer occupies — what indexing text gives. The
/// runtime's `ARTINO_LETTER_BYTES` must agree.
pub const LETTER_BYTES: u32 = 16;

/// The longest array: its length is a byte in the runtime's bounds checks.
pub const MAX_ARRAY: u16 = 255;

/// What an array holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elem {
    Num,
    Bool,
    Text,
    /// Records of one வடிவம், by its index in `Program::shapes`.
    Shape(u16),
}

impl Elem {
    pub fn ty(self) -> Ty {
        match self {
            Elem::Num => Ty::Num,
            Elem::Bool => Ty::Bool,
            Elem::Text => Ty::Text,
            Elem::Shape(id) => Ty::Shape(id),
        }
    }

    fn plural(self) -> String {
        match self {
            Elem::Num => "numbers".to_string(),
            Elem::Bool => "booleans".to_string(),
            Elem::Text => "texts".to_string(),
            Elem::Shape(id) => format!("{} records", shape_name(id)),
        }
    }
}

thread_local! {
    /// The program's வடிவம் names, for messages about them.
    static SHAPE_NAMES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// The C++ functions the program's artino.toml declares.
    static EXTERNS: RefCell<Vec<Extern>> = const { RefCell::new(Vec::new()) };
}

/// A function the manifest declares, by its eTamil name.
fn manifest_function(name: &str) -> Option<Extern> {
    EXTERNS.with(|externs| externs.borrow().iter().find(|e| e.etamil == name).cloned())
}

fn shape_name(id: u16) -> String {
    SHAPE_NAMES.with(|names| {
        names
            .borrow()
            .get(id as usize)
            .cloned()
            .unwrap_or_else(|| "?".to_string())
    })
}

/// What a result holds when it is சரி.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inner {
    Num,
    Bool,
    Text,
    /// சரி(இன்மை): what closing a serial port answers.
    Nothing,
    /// Only a தவறு has been seen; a சரி elsewhere will decide.
    Unknown,
}

impl Inner {
    pub fn ty(self) -> Option<Ty> {
        match self {
            Inner::Num => Some(Ty::Num),
            Inner::Bool => Some(Ty::Bool),
            Inner::Text => Some(Ty::Text),
            Inner::Nothing | Inner::Unknown => None,
        }
    }

    fn of(ty: Ty) -> Option<Inner> {
        match ty {
            Ty::Num => Some(Inner::Num),
            Ty::Bool => Some(Inner::Bool),
            Ty::Text => Some(Inner::Text),
            _ => None,
        }
    }
}

/// The type of a value on a board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    Num,
    Bool,
    /// UTF-8, at most `TEXT_BYTES - 1` bytes.
    Text,
    /// A fixed number of numbers, booleans, texts or records.
    Array(Elem, u16),
    /// A record of one வடிவம், by its index in `Program::shapes`.
    Shape(u16),
    /// சரி(value) or தவறு(text). A board's தவறு always carries text.
    Result(Inner),
    /// A function that returns nothing; never the type of a value.
    Void,
}

/// Two types that must be one: equal, or a result whose சரி is still unknown
/// meeting one whose is known.
pub fn unify(a: Ty, b: Ty) -> Option<Ty> {
    match (a, b) {
        _ if a == b => Some(a),
        (Ty::Result(Inner::Unknown), Ty::Result(_)) => Some(b),
        (Ty::Result(_), Ty::Result(Inner::Unknown)) => Some(a),
        _ => None,
    }
}

impl Ty {
    pub fn name(self) -> String {
        match self {
            Ty::Num => "a number".to_string(),
            Ty::Bool => "true or false".to_string(),
            Ty::Text => "text".to_string(),
            Ty::Array(elem, n) => format!("an array of {} {}", n, elem.plural()),
            Ty::Shape(id) => format!("a {} record", shape_name(id)),
            Ty::Result(Inner::Num) => "a result holding a number".to_string(),
            Ty::Result(Inner::Bool) => "a result holding true or false".to_string(),
            Ty::Result(Inner::Text) => "a result holding text".to_string(),
            Ty::Result(Inner::Nothing) => "a result holding nothing".to_string(),
            Ty::Result(Inner::Unknown) => "a result".to_string(),
            Ty::Void => "nothing".to_string(),
        }
    }

    fn elem(self) -> Option<Elem> {
        match self {
            Ty::Num => Some(Elem::Num),
            Ty::Bool => Some(Elem::Bool),
            Ty::Text => Some(Elem::Text),
            Ty::Shape(id) => Some(Elem::Shape(id)),
            _ => None,
        }
    }
}

/// How an argument crosses into the shim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arg {
    /// A whole number, passed as int32: a pin.
    Int,
    /// மெய் or பொய், passed as int32 1 or 0.
    Flag,
}

/// What the shim returns, and what it becomes on the eTamil side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ret {
    Void,
    /// int32, a whole number: × 1000.
    Int,
    /// uint32 milliseconds: widened, then × 1000.
    Millis,
    /// int32 0 or 1: a boolean.
    Flag,
}

impl Ret {
    pub fn ty(self) -> Ty {
        match self {
            Ret::Void => Ty::Void,
            Ret::Int | Ret::Millis => Ty::Num,
            Ret::Flag => Ty::Bool,
        }
    }
}

/// A function of `nUlakam/vaZporuL/vaZporuL.qmz` that the board provides.
pub struct Intrinsic {
    pub name: &'static str,
    pub shim: &'static str,
    pub args: &'static [Arg],
    pub ret: Ret,
}

pub const INTRINSICS: &[Intrinsic] = &[
    Intrinsic {
        name: "முனை_வெளியீடு",
        shim: "artino_pin_output",
        args: &[Arg::Int],
        ret: Ret::Void,
    },
    Intrinsic {
        name: "முனை_உள்ளீடு",
        shim: "artino_pin_input",
        args: &[Arg::Int],
        ret: Ret::Void,
    },
    Intrinsic {
        name: "முனை_மேலிழு_உள்ளீடு",
        shim: "artino_pin_input_pullup",
        args: &[Arg::Int],
        ret: Ret::Void,
    },
    Intrinsic {
        name: "முனை_எழுது",
        shim: "artino_pin_write",
        args: &[Arg::Int, Arg::Flag],
        ret: Ret::Void,
    },
    Intrinsic {
        name: "முனை_படி",
        shim: "artino_pin_read",
        args: &[Arg::Int],
        ret: Ret::Flag,
    },
    Intrinsic {
        name: "முனை_மாற்று",
        shim: "artino_pin_toggle",
        args: &[Arg::Int],
        ret: Ret::Void,
    },
    Intrinsic {
        name: "ஒப்புமை_படி",
        shim: "artino_analog_read",
        args: &[Arg::Int],
        ret: Ret::Int,
    },
    Intrinsic {
        name: "மில்லி_நொடி",
        shim: "artino_millis",
        args: &[],
        ret: Ret::Millis,
    },
    Intrinsic {
        name: "ஒலி_எழுப்பு",
        shim: "artino_tone",
        args: &[Arg::Int, Arg::Int],
        ret: Ret::Void,
    },
    Intrinsic {
        name: "ஒலி_நிறுத்து",
        shim: "artino_no_tone",
        args: &[Arg::Int],
        ret: Ret::Void,
    },
    Intrinsic {
        name: "காவல்_தொடங்கு",
        shim: "artino_watchdog_begin",
        args: &[Arg::Int],
        ret: Ret::Void,
    },
    Intrinsic {
        name: "காவல்_புதுப்பி",
        shim: "artino_watchdog_feed",
        args: &[],
        ret: Ret::Void,
    },
];

/// vaZporuL functions artino recognises and does not build, with why.
const NOT_ON_A_BOARD: &[(&str, &str)] = &[
    (
        "காத்திரு",
        "a blocking pause (காத்திரு) — a paused board misses input; use இடைவெளி",
    ),
    (
        "போலி_முனை",
        "the simulated board (போலி_*) — it exists only under ETAMIL_BOARD=sim",
    ),
    (
        "போலி_ஒப்புமை",
        "the simulated board (போலி_*) — it exists only under ETAMIL_BOARD=sim",
    ),
    (
        "போலி_நேரம்",
        "the simulated board (போலி_*) — it exists only under ETAMIL_BOARD=sim",
    ),
    (
        "போலி_தொடர்_ஊட்டு",
        "the simulated board (போலி_*) — it exists only under ETAMIL_BOARD=sim",
    ),
    (
        "போலி_தொடர்_வெளியீடு",
        "the simulated board (போலி_*) — it exists only under ETAMIL_BOARD=sim",
    ),
];

/// The builtins artino builds itself, in every spelling the VM accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    /// நீளம் — an array's length, known when compiling.
    Length,
    /// சொல்லாக்கு — a value as text, formatted as அச்சு prints it.
    ToText,
    /// அணி_நிரப்பு(value, count) — `nUlakam/atippatY/aNi.qmz` on the VM; here the
    /// count must be written in the source, because it is the array's length.
    Fill,
    Ok,
    Err,
    IsOk,
    IsErr,
    Unwrap,
    UnwrapErr,
    UnwrapOr,
    /// எண்ணாக்கு — text to a number, as a result.
    ToNumber,
    Floor,
    Ceil,
    /// வட்டமிடு(n, places) — half away from zero; places written in the source.
    Round,
    /// பலகை() — the board's name, known when compiling.
    Board,
    SerialOpen,
    SerialReadLine,
    SerialWrite,
    SerialWriteLine,
    SerialClose,
}

pub fn builtin(name: &str) -> Option<Builtin> {
    match name {
        "நீளம்" | "nILam" | "_length" => Some(Builtin::Length),
        "சொல்லாக்கு" | "collAkku" | "_toString" => Some(Builtin::ToText),
        "அணி_நிரப்பு" => Some(Builtin::Fill),
        "சரி" | "cari" | "_ok" => Some(Builtin::Ok),
        "தவறு" | "qavaRu" | "_err" => Some(Builtin::Err),
        "சரியா" | "cariyA" | "_isOk" => Some(Builtin::IsOk),
        "தவறா" | "qavaRA" | "_isErr" => Some(Builtin::IsErr),
        "மதிப்பு" | "maqippu" | "_unwrap" => Some(Builtin::Unwrap),
        "தவறு_மதிப்பு" | "qavaRu_maqippu" | "_unwrapErr" => {
            Some(Builtin::UnwrapErr)
        }
        "இயல்பு" | "iyalpu" | "_unwrapOr" => Some(Builtin::UnwrapOr),
        "எண்ணாக்கு" | "eNNAkku" | "_toNumber" => Some(Builtin::ToNumber),
        "தரை" | "qarY" | "_floor" => Some(Builtin::Floor),
        "மேல்" | "mEl" | "_ceil" => Some(Builtin::Ceil),
        "வட்டமிடு" | "vattamitu" | "_round" => Some(Builtin::Round),
        // nUlakam/vaZporuL/vaZporuL.qmz, whose VM bodies are not compiled for a board.
        "பலகை" => Some(Builtin::Board),
        "தொடர்_திற" => Some(Builtin::SerialOpen),
        "தொடர்_வரி_படி" => Some(Builtin::SerialReadLine),
        "தொடர்_எழுது" => Some(Builtin::SerialWrite),
        "தொடர்_வரி_எழுது" => Some(Builtin::SerialWriteLine),
        "தொடர்_மூடு" => Some(Builtin::SerialClose),
        _ => None,
    }
}

/// The loop function's name, in all three spellings.
pub fn is_cycle(name: &str) -> bool {
    matches!(name, "சுழற்சி" | "cuzaRci" | "_cycle")
}

pub fn intrinsic(name: &str) -> Option<&'static Intrinsic> {
    INTRINSICS.iter().find(|i| i.name == name)
}

fn not_on_a_board(name: &str) -> Option<&'static str> {
    NOT_ON_A_BOARD
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, why)| *why)
}

/// A number literal as value × 1000, or why it cannot be one.
pub fn scaled(n: &Decimal) -> Result<i64, String> {
    let times = n * Decimal::from(1000);
    if times.fract() != Decimal::ZERO {
        return Err(format!(
            "{} has more than three decimal places; artino numbers keep three",
            n.normalize()
        ));
    }
    times
        .to_i64()
        .ok_or_else(|| format!("{} is too large for a 64-bit artino number", n.normalize()))
}

/// The count of an `அணி_நிரப்பு`: a whole number written in the source.
pub fn fill_count(expr: &Expr) -> Result<u16, String> {
    match expr {
        Expr::Number(n) if n.fract().is_zero() && !n.is_sign_negative() => match n.to_u16() {
            Some(count) if (1..=MAX_ARRAY).contains(&count) => Ok(count),
            _ => Err(format!("அணி_நிரப்பு makes 1 to {} items, not {}", MAX_ARRAY, n.normalize())),
        },
        _ => Err("அணி_நிரப்பு's count must be a whole number written in the source: it is the array's length".to_string()),
    }
}

/// வட்டமிடு's places: a whole number written in the source. A board number
/// keeps three decimals, so more than three changes nothing.
pub fn places(expr: &Expr) -> Result<u32, String> {
    match expr {
        Expr::Number(n) if n.fract().is_zero() && !n.is_sign_negative() => n
            .to_u32()
            .ok_or_else(|| format!("வட்டமிடு to {} places", n.normalize())),
        _ => Err("வட்டமிடு's places must be a whole number written in the source".to_string()),
    }
}

// --- the program, as firmware --------------------------------------------------

#[derive(Debug)]
pub struct Function {
    pub name: String,
    /// The line its name was written on, for reports from inside it.
    pub line: usize,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    /// Every name the body assigns that is not a parameter, first seen first.
    pub locals: Vec<(String, Ty)>,
    /// Every name the body writes, parameters included. A text, array or
    /// record parameter it never writes can be read where the caller holds
    /// it, rather than copied first.
    pub written: HashSet<String>,
    /// Locals that only ever hold one letter: bound by `ஒவ்வொரு` over text and
    /// assigned no other way. Each needs a letter's bytes, not a text's.
    pub letters: HashSet<String>,
    pub body: Vec<Stmt>,
}

/// The names a body writes: assigned, an element or field set, or a loop's variable.
pub(crate) fn written(body: &[Stmt], into: &mut HashSet<String>) {
    written_by(body, into, true);
}

/// The names a body writes other than as a loop's variable.
fn written_plainly(body: &[Stmt], into: &mut HashSet<String>) {
    written_by(body, into, false);
}

fn written_by(body: &[Stmt], into: &mut HashSet<String>, loops: bool) {
    for statement in body {
        match statement {
            Stmt::Assign { name, .. }
            | Stmt::SetIndex { name, .. }
            | Stmt::SetField { name, .. } => {
                into.insert(name.clone());
            }
            Stmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                written_by(then_branch, into, loops);
                if let Some(otherwise) = else_branch {
                    written_by(otherwise, into, loops);
                }
            }
            Stmt::Loop { body, .. } => written_by(body, into, loops),
            Stmt::ForEach { var, body, .. } => {
                if loops {
                    into.insert(var.clone());
                }
                written_by(body, into, loops);
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct Schedule {
    /// The period in milliseconds, which is the literal's value × 1000.
    pub ms: u32,
    /// How the period was written, for reports from inside the block.
    pub seconds: String,
    pub body: Vec<Stmt>,
}

/// A வடிவம், with every field's type settled.
#[derive(Debug, Clone)]
pub struct ShapeInfo {
    pub name: String,
    /// In the order they were declared.
    pub fields: Vec<(String, Ty)>,
}

impl ShapeInfo {
    pub fn field(&self, name: &str) -> Option<(usize, Ty)> {
        self.fields
            .iter()
            .position(|(n, _)| n == name)
            .map(|i| (i, self.fields[i].1))
    }
}

#[derive(Debug)]
pub struct Program {
    /// The program's வடிவங்கள், indexed by `Ty::Shape`.
    pub shapes: Vec<ShapeInfo>,
    /// Which serial ports the program can open: the runtime links only these,
    /// because naming a port the board has makes the linker keep its buffers.
    pub ports: [bool; 4],
    /// Top-level names, with their types, first seen first.
    pub globals: Vec<(String, Ty)>,
    /// The functions something reaches.
    pub functions: Vec<Function>,
    /// What runs once, at power-on.
    pub setup: Vec<Stmt>,
    pub schedules: Vec<Schedule>,
    /// The name of `சுழற்சி`, when the program defines it.
    pub cycle: Option<String>,
    /// The manifest's C++ functions, in its order: the shim for the n-th is
    /// `artino_x_<n>`.
    pub externs: Vec<Extern>,
}

impl Program {
    pub fn global(&self, name: &str) -> Option<Ty> {
        self.globals
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| *t)
    }

    pub fn function(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)
    }
}

// --- refusals: what artino does not build ---------------------------------------------

/// Everything in the reachable program that artino refuses, one label each,
/// in source order. Empty means `analyse` will not refuse on construct grounds;
/// it may still report a type error.
pub fn refusals(statements: &[Stmt]) -> Vec<String> {
    let reachable = reachable_functions(statements);
    let mut found = Vec::new();
    for statement in statements {
        match statement {
            Stmt::FunctionDef { name, body, .. } => {
                if reachable.contains(name.as_str()) {
                    refuse_block(body, true, &mut found);
                }
            }
            Stmt::ShapeDef { name, methods, .. } => {
                for method in methods {
                    if reachable.contains(&crate::vm::shape::method_function(name, &method.name)) {
                        refuse_block(&method.body, true, &mut found);
                    }
                }
            }
            other => refuse_stmt(other, false, &mut found),
        }
    }
    found
}

fn refuse_block(body: &[Stmt], in_function: bool, found: &mut Vec<String>) {
    for statement in body {
        refuse_stmt(statement, in_function, found);
    }
}

fn refuse_stmt(statement: &Stmt, in_function: bool, found: &mut Vec<String>) {
    match statement {
        Stmt::Assign { value, .. } => refuse_expr(value, found),
        Stmt::Expression(expr) | Stmt::Print(expr) => refuse_expr(expr, found),
        Stmt::Return(value) => {
            if !in_function {
                found.push("திரும்பு outside a செயல்".to_string());
            }
            if let Some(value) = value {
                refuse_expr(value, found);
            }
        }
        Stmt::If {
            condition,
            then_branch,
            else_branch,
        } => {
            refuse_expr(condition, found);
            refuse_block(then_branch, in_function, found);
            if let Some(branch) = else_branch {
                refuse_block(branch, in_function, found);
            }
        }
        Stmt::Loop { condition, body } => {
            refuse_expr(condition, found);
            refuse_block(body, in_function, found);
        }
        Stmt::ForEach {
            collection, body, ..
        } => {
            refuse_expr(collection, found);
            refuse_block(body, in_function, found);
        }
        Stmt::SetIndex { index, value, .. } => {
            refuse_expr(index, found);
            refuse_expr(value, found);
        }
        Stmt::Schedule { seconds, body } => {
            if in_function {
                found.push("இடைவெளி inside a செயல் — write it at the top level".to_string());
            }
            match seconds {
                Expr::Number(n) if !n.is_sign_negative() => {
                    if let Err(why) = scaled(n) {
                        found.push(format!("இடைவெளி period: {}", why));
                    }
                }
                _ => found.push(
                    "இடைவெளி with a period that is not a number written in the source".to_string(),
                ),
            }
            refuse_block(body, in_function, found);
        }
        // Resolved into the program before this runs; nothing is left to build.
        Stmt::Import(_) => {}
        Stmt::FunctionDef { .. } => found.push("a செயல் defined inside a செயல் or block".to_string()),
        Stmt::ShapeDef { .. } => found.push("a வடிவம் defined inside a செயல் or block".to_string()),
        Stmt::SetField { value, .. } => refuse_expr(value, found),
        Stmt::Input(_) => {
            found.push("உள்ளிடு (keyboard input) — read a serial port instead".to_string())
        }
        other => found.push(format!(
            "{} — not available on a board",
            crate::codegen::stmt_label(other)
        )),
    }
}

fn refuse_expr(expr: &Expr, found: &mut Vec<String>) {
    match expr {
        Expr::Number(n) => {
            if let Err(why) = scaled(n) {
                found.push(why);
            }
        }
        Expr::Boolean(_) | Expr::Variable(_) | Expr::String(_) => {}
        Expr::BinaryOp { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Logical { left, right, .. }
        | Expr::Concat { left, right }
        | Expr::Index { base: left, index: right } => {
            refuse_expr(left, found);
            refuse_expr(right, found);
        }
        Expr::Not(inner) => refuse_expr(inner, found),
        Expr::ArrayLiteral(items) => items.iter().for_each(|item| refuse_expr(item, found)),
        Expr::Call { name, args } => {
            if let Some(why) = not_on_a_board(name) {
                found.push(why.to_string());
            }
            for arg in args {
                refuse_expr(arg, found);
            }
        }
        Expr::Null => found.push("இன்மை (nil)".to_string()),
        Expr::RecordLiteral(_) => found.push(
            "a record without a வடிவம் — a board needs its fields known when compiling; declare a வடிவம்".to_string(),
        ),
        Expr::Field { base, .. } => refuse_expr(base, found),
        Expr::ShapeLiteral { fields, base, .. } => {
            fields.iter().for_each(|(_, value)| refuse_expr(value, found));
            if let Some(base) = base {
                refuse_expr(base, found);
            }
        }
        Expr::MethodCall { receiver, args, .. } => {
            refuse_expr(receiver, found);
            args.iter().for_each(|arg| refuse_expr(arg, found));
        }
        Expr::Try(inner) => refuse_expr(inner, found),
        Expr::Lambda { .. } | Expr::CallValue { .. } => found.push("functions as values".to_string()),
    }
}

// --- reachability ---------------------------------------------------------------------

/// The user's functions that the top level, an இடைவெளி block or சுழற்சி can
/// reach. An imported module's other functions are not firmware and are not
/// judged — `nUlakam/vaZporuL/vaZporuL.qmz` itself defines போலி_* over builtins no
/// board has, and `nUlakam/atippatY/aNi.qmz` is full of functions as values.
fn reachable_functions(statements: &[Stmt]) -> HashSet<String> {
    let mut defined: HashMap<String, &Vec<Stmt>> = statements
        .iter()
        .filter_map(|s| match s {
            Stmt::FunctionDef { name, body, .. } if !is_built_in(name) => {
                Some((name.clone(), body))
            }
            _ => None,
        })
        .collect();
    // A method is `Shape.method`. A call `r.m(…)` does not say which shape r
    // is until it is typed, so it reaches every shape's `m`.
    let mut methods: HashMap<String, Vec<String>> = HashMap::new();
    for statement in statements {
        if let Stmt::ShapeDef {
            name,
            methods: list,
            ..
        } = statement
        {
            for method in list {
                let full = crate::vm::shape::method_function(name, &method.name);
                methods
                    .entry(method.name.clone())
                    .or_default()
                    .push(full.clone());
                defined.insert(full, &method.body);
            }
        }
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut pending: Vec<String> = Vec::new();
    for statement in statements {
        match statement {
            Stmt::FunctionDef { name, .. } if is_cycle(name) => pending.push(name.clone()),
            Stmt::FunctionDef { .. } => {}
            other => calls_in_stmt(other, &mut pending),
        }
    }
    while let Some(name) = pending.pop() {
        if let Some(method) = name.strip_prefix(METHOD_MARK) {
            pending.extend(methods.get(method).cloned().unwrap_or_default());
            continue;
        }
        if !defined.contains_key(name.as_str()) || !seen.insert(name.clone()) {
            continue;
        }
        for statement in defined[name.as_str()] {
            calls_in_stmt(statement, &mut pending);
        }
    }
    seen
}

/// In the pending list, a method called by name, shape not yet known.
const METHOD_MARK: &str = "\u{1}method:";

/// A name the board or artino provides, whose eTamil definition — in
/// vaZporuL.qmz or aNi.qmz — is never compiled.
fn is_built_in(name: &str) -> bool {
    intrinsic(name).is_some()
        || not_on_a_board(name).is_some()
        || builtin(name).is_some()
        || manifest_function(name).is_some()
}

fn calls_in_stmt(statement: &Stmt, into: &mut Vec<String>) {
    match statement {
        Stmt::Assign { value, .. } | Stmt::Expression(value) | Stmt::Print(value) => {
            calls_in_expr(value, into)
        }
        Stmt::Return(Some(value)) => calls_in_expr(value, into),
        Stmt::If {
            condition,
            then_branch,
            else_branch,
        } => {
            calls_in_expr(condition, into);
            then_branch.iter().for_each(|s| calls_in_stmt(s, into));
            if let Some(branch) = else_branch {
                branch.iter().for_each(|s| calls_in_stmt(s, into));
            }
        }
        Stmt::Loop { condition, body } => {
            calls_in_expr(condition, into);
            body.iter().for_each(|s| calls_in_stmt(s, into));
        }
        Stmt::ForEach {
            collection, body, ..
        } => {
            calls_in_expr(collection, into);
            body.iter().for_each(|s| calls_in_stmt(s, into));
        }
        Stmt::SetIndex { index, value, .. } => {
            calls_in_expr(index, into);
            calls_in_expr(value, into);
        }
        Stmt::SetField { value, .. } => calls_in_expr(value, into),
        Stmt::Schedule { body, .. } => body.iter().for_each(|s| calls_in_stmt(s, into)),
        _ => {}
    }
}

fn calls_in_expr(expr: &Expr, into: &mut Vec<String>) {
    match expr {
        Expr::Call { name, args } => {
            into.push(name.clone());
            args.iter().for_each(|a| calls_in_expr(a, into));
        }
        Expr::BinaryOp { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Logical { left, right, .. }
        | Expr::Concat { left, right }
        | Expr::Index {
            base: left,
            index: right,
        } => {
            calls_in_expr(left, into);
            calls_in_expr(right, into);
        }
        Expr::Not(inner) | Expr::Try(inner) => calls_in_expr(inner, into),
        Expr::ArrayLiteral(items) => items.iter().for_each(|i| calls_in_expr(i, into)),
        Expr::Field { base, .. } => calls_in_expr(base, into),
        Expr::ShapeLiteral { fields, base, .. } => {
            fields
                .iter()
                .for_each(|(_, value)| calls_in_expr(value, into));
            if let Some(base) = base {
                calls_in_expr(base, into);
            }
        }
        Expr::MethodCall {
            receiver,
            name,
            args,
            ..
        } => {
            into.push(format!("{}{}", METHOD_MARK, name));
            calls_in_expr(receiver, into);
            args.iter().for_each(|a| calls_in_expr(a, into));
        }
        _ => {}
    }
}

/// Does `expr` read the variable `name` anywhere.
#[cfg(feature = "llvm")]
pub(crate) fn mentions(expr: &Expr, name: &str) -> bool {
    match expr {
        Expr::Variable(n) => n == name,
        Expr::Call { args, .. } => args.iter().any(|a| mentions(a, name)),
        Expr::BinaryOp { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Logical { left, right, .. }
        | Expr::Concat { left, right }
        | Expr::Index {
            base: left,
            index: right,
        } => mentions(left, name) || mentions(right, name),
        Expr::Not(inner) | Expr::Try(inner) => mentions(inner, name),
        Expr::ArrayLiteral(items) => items.iter().any(|i| mentions(i, name)),
        Expr::Field { base, .. } => mentions(base, name),
        Expr::ShapeLiteral { fields, base, .. } => {
            fields.iter().any(|(_, value)| mentions(value, name))
                || base.as_ref().is_some_and(|b| mentions(b, name))
        }
        Expr::MethodCall { receiver, args, .. } => {
            mentions(receiver, name) || args.iter().any(|a| mentions(a, name))
        }
        _ => false,
    }
}

// --- types ------------------------------------------------------------------------------

/// A parameter's type, from its declaration; `None` when undeclared, and then
/// the type its callers pass decides — `nUlakam/atippatY/col.qmz` declares none.
fn param_type(
    declared: &Option<DeclaredType>,
    name: &str,
    function: &str,
    shapes: &[String],
    errors: &mut Vec<String>,
) -> Option<Ty> {
    match declared {
        Some(DeclaredType::Boolean) => Some(Ty::Bool),
        Some(DeclaredType::Text) => Some(Ty::Text),
        Some(DeclaredType::Number) => Some(Ty::Num),
        Some(DeclaredType::Shape(shape)) => match shapes.iter().position(|s| s == shape) {
            Some(id) => Some(Ty::Shape(id as u16)),
            None => {
                errors.push(format!(
                    "{}: {} is declared {}, which is not a வடிவம் in this program",
                    function, name, shape
                ));
                None
            }
        },
        // An array's length is part of its type: the callers' arrays decide it.
        _ => None,
    }
}

/// The signature key a வடிவம்'s fields are learned under, beside the functions'.
fn shape_key(shape: &str) -> String {
    format!("\u{1}shape:{}", shape)
}

/// What callers have passed each parameter, gathered while typing.
type Observed = RefCell<HashMap<String, Vec<Option<Ty>>>>;

/// A வடிவம் as declared: its name, and each field with the type written, if any.
type ShapeDecl = (String, Vec<(String, Option<DeclaredType>)>);

/// A செயல் by name: its parameters, typed where declared, and its body.
type Defs = BTreeMap<String, (Vec<(String, Option<Ty>)>, Vec<Stmt>)>;

/// Build the firmware view of a program, or every reason it cannot be built.
pub fn analyse(statements: &[Stmt]) -> Result<Program, Vec<String>> {
    analyse_with(statements, &[])
}

/// As `analyse`, with the C++ functions a manifest declares callable.
pub fn analyse_with(statements: &[Stmt], externs: &[Extern]) -> Result<Program, Vec<String>> {
    EXTERNS.with(|e| *e.borrow_mut() = externs.to_vec());
    let refused = refusals(statements);
    if !refused.is_empty() {
        return Err(refused);
    }

    let reachable = reachable_functions(statements);

    // The வடிவங்கள் first: a parameter or a field can be declared as one.
    let mut shape_names: Vec<String> = Vec::new();
    let mut shape_decls: Vec<ShapeDecl> = Vec::new();
    for statement in statements {
        if let Stmt::ShapeDef { name, fields, .. } = statement {
            shape_names.push(name.clone());
            shape_decls.push((
                name.clone(),
                fields
                    .iter()
                    .map(|f| (f.name.clone(), f.declared.clone()))
                    .collect(),
            ));
        }
    }
    SHAPE_NAMES.with(|names| *names.borrow_mut() = shape_names.clone());

    let mut defs: Defs = BTreeMap::new();
    let mut lines: HashMap<String, usize> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut setup = Vec::new();
    let mut schedules = Vec::new();
    let mut cycle = None;
    let mut errors = Vec::new();

    for statement in statements {
        match statement {
            Stmt::FunctionDef {
                name,
                params,
                body,
                at,
                ..
            } => {
                if !reachable.contains(name.as_str()) {
                    continue;
                }
                lines.insert(name.clone(), at.line);
                if is_cycle(name) {
                    if !params.is_empty() {
                        errors.push(format!(
                            "{} is called by the board with nothing, so it takes no parameters",
                            name
                        ));
                    }
                    cycle = Some(name.clone());
                }
                let typed = params
                    .iter()
                    .map(|p| {
                        (
                            p.name.clone(),
                            param_type(&p.declared, &p.name, name, &shape_names, &mut errors),
                        )
                    })
                    .collect();
                if defs.insert(name.clone(), (typed, body.clone())).is_none() {
                    order.push(name.clone());
                }
            }
            Stmt::ShapeDef {
                name: shape,
                methods,
                at,
                ..
            } => {
                let id = shape_names.iter().position(|s| s == shape).unwrap() as u16;
                for method in methods {
                    let full = crate::vm::shape::method_function(shape, &method.name);
                    if !reachable.contains(&full) {
                        continue;
                    }
                    lines.insert(full.clone(), method.at.line.max(at.line));
                    let typed = method
                        .params
                        .iter()
                        .enumerate()
                        .map(|(i, p)| {
                            let ty = if i == 0 && method.takes_self() {
                                Some(Ty::Shape(id))
                            } else {
                                param_type(&p.declared, &p.name, &full, &shape_names, &mut errors)
                            };
                            (p.name.clone(), ty)
                        })
                        .collect();
                    if defs
                        .insert(full.clone(), (typed, method.body.clone()))
                        .is_none()
                    {
                        order.push(full);
                    }
                }
            }
            Stmt::Schedule {
                seconds: Expr::Number(n),
                body,
            } => {
                let ms = scaled(n).unwrap_or(0);
                match u32::try_from(ms) {
                    Ok(ms) => schedules.push(Schedule {
                        ms,
                        seconds: n.normalize().to_string(),
                        body: body.clone(),
                    }),
                    Err(_) => errors.push(format!(
                        "இடைவெளி {} is longer than a board's clock can count",
                        n.normalize()
                    )),
                }
            }
            Stmt::Import(_) => {}
            other => setup.push(other.clone()),
        }
    }

    // Parameter types: declared ones fixed; the rest learned from the calls.
    // A வடிவம்'s fields are learned the same way, from its literals.
    let mut signatures: HashMap<String, Vec<Option<Ty>>> = defs
        .iter()
        .map(|(name, (params, _))| (name.clone(), params.iter().map(|(_, t)| *t).collect()))
        .collect();
    let mut fixed: HashMap<String, Vec<bool>> = defs
        .iter()
        .map(|(name, (params, _))| {
            (
                name.clone(),
                params.iter().map(|(_, t)| t.is_some()).collect(),
            )
        })
        .collect();
    for (shape, fields) in &shape_decls {
        let types: Vec<Option<Ty>> = fields
            .iter()
            .map(|(field, declared)| param_type(declared, field, shape, &shape_names, &mut errors))
            .collect();
        fixed.insert(
            shape_key(shape),
            types.iter().map(Option::is_some).collect(),
        );
        signatures.insert(shape_key(shape), types);
    }
    let shape_fields: Vec<(String, Vec<String>)> = shape_decls
        .iter()
        .map(|(n, f)| (n.clone(), f.iter().map(|(x, _)| x.clone()).collect()))
        .collect();
    let typed = |signatures: &HashMap<String, Vec<Option<Ty>>>, name: &str| -> Vec<(String, Ty)> {
        defs[name]
            .0
            .iter()
            .zip(&signatures[name])
            .map(|((param, _), ty)| (param.clone(), ty.unwrap_or(Ty::Num)))
            .collect()
    };

    // Globals and return types settle together, to a fixed point: the top
    // level calls functions, and functions read the top level's names. A
    // function's type is its first typed திரும்பு; a recursive call is typed
    // once its base case is.
    let mut returns: HashMap<String, Ty> = HashMap::new();
    let mut globals: Vec<(String, Ty)> = Vec::new();
    for _ in 0..=2 * defs.len() + 2 {
        let mut noise = Vec::new();
        let observed: Observed = RefCell::new(HashMap::new());
        let mut top = Scope::top(&returns, &signatures, &observed, &shape_fields);
        for statement in &setup {
            top.stmt(statement, &mut noise);
        }
        for schedule in &schedules {
            top.block(&schedule.body, &mut noise);
        }
        let next_globals = top.names;

        let mut changed = next_globals != globals;
        globals = next_globals;
        for name in &order {
            let params = typed(&signatures, name);
            let body = &defs[name].1;
            let mut scope =
                Scope::function(&params, &returns, &signatures, &observed, &shape_fields);
            scope.globals = globals.clone();
            let found = scope.block(body, &mut noise);
            let ty = found.unwrap_or(Ty::Void);
            let next = match returns.get(name) {
                Some(seen) if found.is_some() => unify(*seen, ty).unwrap_or(ty),
                Some(seen) => *seen,
                None => ty,
            };
            if returns.get(name) != Some(&next) {
                returns.insert(name.clone(), next);
                changed = true;
            }
        }
        // An undeclared parameter takes the type its callers passed.
        for (name, seen) in observed.into_inner() {
            let Some(signature) = signatures.get_mut(&name) else {
                continue;
            };
            let declared = &fixed[&name];
            for (i, ty) in seen.into_iter().enumerate() {
                if i >= signature.len() || declared[i] {
                    continue;
                }
                if let Some(ty) = ty {
                    // Replace rather than keep: an earlier pass may have seen a
                    // number that only came from a parameter not yet known,
                    // and what this pass saw is better informed.
                    let next = Some(ty);
                    if signature[i] != next {
                        signature[i] = next;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }

    // The settled pass, whose errors are the real ones: setup first, then each
    // block, in the order they run, then every function against those globals.
    let observed: Observed = RefCell::new(HashMap::new());
    let mut top = Scope::top(&returns, &signatures, &observed, &shape_fields);
    for statement in &setup {
        top.stmt(statement, &mut errors);
    }
    for schedule in &schedules {
        top.block(&schedule.body, &mut errors);
    }
    let globals = top.names;

    let mut functions = Vec::new();
    for name in &order {
        let params = typed(&signatures, name);
        let body = &defs[name].1;
        let mut scope = Scope::function(&params, &returns, &signatures, &observed, &shape_fields);
        scope.globals = globals.clone();
        scope.block(body, &mut errors);
        let ret = returns.get(name).copied().unwrap_or(Ty::Void);
        if scope.tried && !matches!(ret, Ty::Result(_)) {
            errors.push(format!(
                "{} uses ? but does not return a result (சரி or தவறு)",
                name
            ));
        }
        if ret == Ty::Result(Inner::Unknown) {
            errors.push(format!(
                "{} only ever returns தவறு, so what its சரி holds is unknown",
                name
            ));
        }
        let locals = scope
            .names
            .into_iter()
            .filter(|(n, _)| !params.iter().any(|(p, _)| p == n))
            .collect();
        let letters = {
            let mut assigned = HashSet::new();
            written_plainly(body, &mut assigned);
            scope
                .letter_loops
                .iter()
                .filter(|name| !assigned.contains(*name) && !scope.other_loops.contains(*name))
                .filter(|name| !params.iter().any(|(p, _)| p == *name))
                .cloned()
                .collect()
        };
        functions.push(Function {
            name: name.clone(),
            line: lines.get(name).copied().unwrap_or(0),
            params,
            ret,
            locals,
            written: {
                let mut names = HashSet::new();
                written(body, &mut names);
                names
            },
            letters,
            body: body.clone(),
        });
    }

    let shapes: Vec<ShapeInfo> = shape_decls
        .iter()
        .map(|(shape, fields)| {
            let types = &signatures[&shape_key(shape)];
            ShapeInfo {
                name: shape.clone(),
                fields: fields
                    .iter()
                    .zip(types)
                    .map(|((field, _), ty)| {
                        if ty.is_none() {
                            errors.push(format!(
                                "{}.{} is never given a value, so its type is unknown — declare it",
                                shape, field
                            ));
                        }
                        (field.clone(), ty.unwrap_or(Ty::Num))
                    })
                    .collect(),
            }
        })
        .collect();
    let ports = ports_opened(statements);

    if !errors.is_empty() {
        errors.dedup();
        return Err(errors);
    }
    Ok(Program {
        shapes,
        ports,
        globals,
        functions,
        setup,
        schedules,
        cycle,
        externs: externs.to_vec(),
    })
}

/// The serial ports a program can open: a port written as a number is that
/// port; one computed at run time could be any.
fn ports_opened(statements: &[Stmt]) -> [bool; 4] {
    fn walk_expr(expr: &Expr, ports: &mut [bool; 4]) {
        match expr {
            Expr::Call { name, args } => {
                if builtin(name) == Some(Builtin::SerialOpen) {
                    match args.first() {
                        Some(Expr::Number(n)) => {
                            if let Some(port) = n.to_usize().filter(|p| *p < 4) {
                                ports[port] = true;
                            }
                        }
                        _ => *ports = [true; 4],
                    }
                }
                args.iter().for_each(|a| walk_expr(a, ports));
            }
            Expr::BinaryOp { left, right, .. }
            | Expr::Comparison { left, right, .. }
            | Expr::Logical { left, right, .. }
            | Expr::Concat { left, right }
            | Expr::Index {
                base: left,
                index: right,
            } => {
                walk_expr(left, ports);
                walk_expr(right, ports);
            }
            Expr::Not(inner) | Expr::Try(inner) => walk_expr(inner, ports),
            Expr::ArrayLiteral(items) => items.iter().for_each(|i| walk_expr(i, ports)),
            Expr::Field { base, .. } => walk_expr(base, ports),
            Expr::ShapeLiteral { fields, base, .. } => {
                fields.iter().for_each(|(_, v)| walk_expr(v, ports));
                if let Some(base) = base {
                    walk_expr(base, ports);
                }
            }
            Expr::MethodCall { receiver, args, .. } => {
                walk_expr(receiver, ports);
                args.iter().for_each(|a| walk_expr(a, ports));
            }
            _ => {}
        }
    }
    fn walk(body: &[Stmt], ports: &mut [bool; 4]) {
        for statement in body {
            match statement {
                Stmt::Assign { value, .. } | Stmt::Expression(value) | Stmt::Print(value) => {
                    walk_expr(value, ports)
                }
                Stmt::Return(Some(value)) | Stmt::SetField { value, .. } => walk_expr(value, ports),
                Stmt::SetIndex { index, value, .. } => {
                    walk_expr(index, ports);
                    walk_expr(value, ports);
                }
                Stmt::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    walk_expr(condition, ports);
                    walk(then_branch, ports);
                    if let Some(branch) = else_branch {
                        walk(branch, ports);
                    }
                }
                Stmt::Loop { condition, body } => {
                    walk_expr(condition, ports);
                    walk(body, ports);
                }
                Stmt::ForEach {
                    collection, body, ..
                } => {
                    walk_expr(collection, ports);
                    walk(body, ports);
                }
                Stmt::Schedule { body, .. } | Stmt::FunctionDef { body, .. } => walk(body, ports),
                Stmt::ShapeDef { methods, .. } => methods.iter().for_each(|m| walk(&m.body, ports)),
                _ => {}
            }
        }
    }
    let mut ports = [true, false, false, false];
    walk(statements, &mut ports);
    ports
}

/// Names in scope while typing one body.
struct Scope<'a> {
    /// Names this scope assigns (or, for a function, its parameters), first seen first.
    names: Vec<(String, Ty)>,
    /// For a function: the top level's names, readable, never assigned.
    globals: Vec<(String, Ty)>,
    returns: &'a HashMap<String, Ty>,
    /// Every reachable செயல், with its parameters' types where known.
    signatures: &'a HashMap<String, Vec<Option<Ty>>>,
    /// Where calls record what they pass.
    observed: &'a Observed,
    /// Each வடிவம்'s name and field names, in declared order.
    shapes: &'a [(String, Vec<String>)],
    in_function: bool,
    /// The body uses `?`, so the function must return a result.
    tried: bool,
    /// Loop variables of `ஒவ்வொரு` over text, and of `ஒவ்வொரு` over anything else.
    letter_loops: HashSet<String>,
    other_loops: HashSet<String>,
}

impl<'a> Scope<'a> {
    fn top(
        returns: &'a HashMap<String, Ty>,
        signatures: &'a HashMap<String, Vec<Option<Ty>>>,
        observed: &'a Observed,
        shapes: &'a [(String, Vec<String>)],
    ) -> Self {
        Scope {
            names: Vec::new(),
            globals: Vec::new(),
            returns,
            signatures,
            observed,
            shapes,
            in_function: false,
            tried: false,
            letter_loops: HashSet::new(),
            other_loops: HashSet::new(),
        }
    }

    fn function(
        params: &[(String, Ty)],
        returns: &'a HashMap<String, Ty>,
        signatures: &'a HashMap<String, Vec<Option<Ty>>>,
        observed: &'a Observed,
        shapes: &'a [(String, Vec<String>)],
    ) -> Self {
        Scope {
            names: params.to_vec(),
            globals: Vec::new(),
            returns,
            signatures,
            observed,
            shapes,
            in_function: true,
            tried: false,
            letter_loops: HashSet::new(),
            other_loops: HashSet::new(),
        }
    }

    fn shape_id(&self, name: &str) -> Option<u16> {
        self.shapes
            .iter()
            .position(|(n, _)| n == name)
            .map(|i| i as u16)
    }

    /// A field's index and its type if known yet.
    fn field(&self, id: u16, field: &str) -> Option<(usize, Option<Ty>)> {
        let (shape, fields) = &self.shapes[id as usize];
        let index = fields.iter().position(|f| f == field)?;
        Some((index, self.signatures[&shape_key(shape)][index]))
    }

    /// A value given to a field: record it, and check it against what is known.
    fn give_field(&mut self, id: u16, index: usize, ty: Ty, errors: &mut Vec<String>) {
        let (shape, fields) = &self.shapes[id as usize];
        let key = shape_key(shape);
        if let Some(Some(known)) = self.signatures[&key].get(index)
            && unify(*known, ty).is_none()
        {
            errors.push(format!(
                "{}.{} holds {}, not {}",
                shape,
                fields[index],
                known.name(),
                ty.name()
            ));
        }
        let mut observed = self.observed.borrow_mut();
        let seen = observed
            .entry(key)
            .or_insert_with(|| vec![None; fields.len()]);
        if seen[index].is_none() {
            seen[index] = Some(ty);
        }
    }

    fn lookup(&self, name: &str) -> Option<Ty> {
        self.names
            .iter()
            .chain(self.globals.iter())
            .find(|(n, _)| n == name)
            .map(|(_, t)| *t)
    }

    /// Give `name` type `ty` in this scope: its first assignment decides.
    fn bind(&mut self, name: &str, ty: Ty, errors: &mut Vec<String>) {
        match self.names.iter_mut().find(|(n, _)| n == name) {
            Some((_, existing)) => match unify(*existing, ty) {
                Some(both) => *existing = both,
                None => errors.push(format!(
                    "{} holds {} and is later given {}; on a board a name keeps one type",
                    name,
                    existing.name(),
                    ty.name()
                )),
            },
            None => self.names.push((name.to_string(), ty)),
        }
    }

    /// Type a block; what its திரும்புs return, unified, if any returns a value.
    fn block(&mut self, body: &[Stmt], errors: &mut Vec<String>) -> Option<Ty> {
        let mut found: Option<Ty> = None;
        for statement in body {
            if let Some(ty) = self.stmt(statement, errors) {
                found = Some(match found {
                    None => ty,
                    Some(seen) => match unify(seen, ty) {
                        Some(both) => both,
                        None => {
                            errors.push(format!(
                                "one திரும்பு gives {} and another {}; a செயல் returns one type",
                                seen.name(),
                                ty.name()
                            ));
                            seen
                        }
                    },
                });
            }
        }
        found
    }

    fn stmt(&mut self, statement: &Stmt, errors: &mut Vec<String>) -> Option<Ty> {
        match statement {
            Stmt::Assign {
                name,
                value,
                declared,
                ..
            } => {
                let ty = self.value(value, errors)?;
                let fits = match declared {
                    Some(DeclaredType::Boolean) => ty == Ty::Bool,
                    Some(DeclaredType::Number) => ty == Ty::Num,
                    Some(DeclaredType::Text) => ty == Ty::Text,
                    Some(DeclaredType::Array) => matches!(ty, Ty::Array(..)),
                    Some(DeclaredType::Shape(shape)) => {
                        self.shape_id(shape).map(Ty::Shape) == Some(ty)
                    }
                    _ => true,
                };
                if !fits {
                    errors.push(format!(
                        "{} is declared {} but given {}",
                        name,
                        declared.as_ref().unwrap().name(),
                        ty.name()
                    ));
                }
                self.bind(name, ty, errors);
                None
            }
            Stmt::Expression(expr) => {
                self.expr(expr, errors);
                None
            }
            Stmt::Print(expr) => {
                self.printed(expr, errors);
                None
            }
            Stmt::Return(Some(value)) => self.value(value, errors),
            Stmt::Return(None) => None,
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.condition(condition, errors);
                let a = self.block(then_branch, errors);
                let b = else_branch
                    .as_ref()
                    .and_then(|branch| self.block(branch, errors));
                match (a, b) {
                    (Some(a), Some(b)) => unify(a, b).or(Some(a)),
                    (a, b) => a.or(b),
                }
            }
            Stmt::Loop { condition, body } => {
                self.condition(condition, errors);
                self.block(body, errors)
            }
            Stmt::ForEach {
                var,
                collection,
                body,
            } => {
                match self.value(collection, errors) {
                    Some(Ty::Array(elem, _)) => {
                        self.other_loops.insert(var.clone());
                        self.bind(var, elem.ty(), errors)
                    }
                    Some(Ty::Text) => {
                        self.letter_loops.insert(var.clone());
                        self.bind(var, Ty::Text, errors)
                    }
                    Some(other) => {
                        errors.push(format!("ஒவ்வொரு needs an array, not {}", other.name()))
                    }
                    None => {}
                }
                self.block(body, errors)
            }
            Stmt::SetField {
                name, field, value, ..
            } => {
                let owned = self.names.iter().any(|(n, _)| n == name);
                let given = self.value(value, errors);
                match self.lookup(name) {
                    Some(Ty::Shape(id)) => {
                        if !owned {
                            errors.push(format!(
                                "{}.{} = … changes the program's record from inside a செயல், which it cannot",
                                name, field
                            ));
                        }
                        match self.field(id, field) {
                            Some((index, _)) => {
                                if let Some(ty) = given {
                                    self.give_field(id, index, ty, errors);
                                }
                            }
                            None => {
                                errors.push(format!("{} has no field {}", shape_name(id), field))
                            }
                        }
                    }
                    Some(other) => errors.push(format!(
                        "{}.{} = … needs a record, and {} is {}",
                        name,
                        field,
                        name,
                        other.name()
                    )),
                    None => errors.push(format!(
                        "{} is used before anything is assigned to it",
                        name
                    )),
                }
                None
            }
            Stmt::SetIndex {
                name, index, value, ..
            } => {
                let owned = self.names.iter().any(|(n, _)| n == name);
                match self.lookup(name) {
                    Some(Ty::Array(elem, _)) => {
                        if !owned {
                            errors.push(format!(
                                "{}[…] = … changes the program's array from inside a செயல், which it cannot",
                                name
                            ));
                        }
                        self.number(index, "an index", errors);
                        if let Some(ty) = self.value(value, errors)
                            && ty != elem.ty()
                        {
                            errors.push(format!(
                                "{} holds {}, not {}",
                                name,
                                elem.ty().name(),
                                ty.name()
                            ));
                        }
                    }
                    Some(other) => errors.push(format!(
                        "{}[…] = … needs an array, and {} is {}",
                        name,
                        name,
                        other.name()
                    )),
                    None => errors.push(format!(
                        "{} is used before anything is assigned to it",
                        name
                    )),
                }
                None
            }
            _ => None,
        }
    }

    /// What அச்சு may print: any value, and & chains of text, numbers and booleans.
    fn printed(&mut self, expr: &Expr, errors: &mut Vec<String>) {
        self.value(expr, errors);
    }

    fn condition(&mut self, expr: &Expr, errors: &mut Vec<String>) {
        if let Some(ty) = self.value(expr, errors)
            && ty != Ty::Bool
        {
            errors.push(format!(
                "a condition must be true or false, not {}",
                ty.name()
            ));
        }
    }

    fn number(&mut self, expr: &Expr, what: &str, errors: &mut Vec<String>) {
        if let Some(ty) = self.value(expr, errors)
            && ty != Ty::Num
        {
            errors.push(format!("{} must be a number, not {}", what, ty.name()));
        }
    }

    /// The type of an expression used as a value: not nothing.
    fn value(&mut self, expr: &Expr, errors: &mut Vec<String>) -> Option<Ty> {
        match self.expr(expr, errors) {
            Some(Ty::Void) => {
                if let Expr::Call { name, .. } = expr {
                    errors.push(format!(
                        "{}(…) returns nothing, so it cannot be used as a value",
                        name
                    ));
                }
                None
            }
            other => other,
        }
    }

    fn expr(&mut self, expr: &Expr, errors: &mut Vec<String>) -> Option<Ty> {
        match expr {
            Expr::Number(_) => Some(Ty::Num),
            Expr::Boolean(_) => Some(Ty::Bool),
            Expr::String(_) => Some(Ty::Text),
            Expr::Variable(name) => match self.lookup(name) {
                Some(ty) => Some(ty),
                None => {
                    errors.push(format!(
                        "{} is used before anything is assigned to it",
                        name
                    ));
                    None
                }
            },
            Expr::BinaryOp { op, left, right } => {
                let a = self.value(left, errors);
                let b = self.value(right, errors);
                for ty in [a, b].into_iter().flatten() {
                    if ty != Ty::Num {
                        errors.push(format!(
                            "{} needs numbers on both sides, not {}",
                            op,
                            ty.name()
                        ));
                    }
                }
                Some(Ty::Num)
            }
            Expr::Concat { left, right } => {
                for side in [left, right] {
                    if let Some(ty) = self.value(side, errors)
                        && let Ty::Array(..) | Ty::Result(_) | Ty::Shape(_) = ty
                    {
                        errors.push(format!(
                            "{} joined into text with & — print it on its own",
                            ty.name()
                        ));
                    }
                }
                Some(Ty::Text)
            }
            Expr::Comparison { op, left, right } => {
                let a = self.value(left, errors);
                let b = self.value(right, errors);
                if let (Some(a), Some(b)) = (a, b) {
                    let ordering = op != "==" && op != "!=";
                    if unify(a, b).is_none() {
                        errors.push(format!("{} compares {} with {}", op, a.name(), b.name()));
                    } else if let Ty::Array(..) | Ty::Result(_) | Ty::Shape(_) = a {
                        errors.push(format!("{} on {} — compare what they hold", op, a.name()));
                    } else if ordering && a != Ty::Num {
                        errors.push(format!(
                            "{} orders numbers; {} has no order here",
                            op,
                            a.name()
                        ));
                    }
                }
                Some(Ty::Bool)
            }
            Expr::Logical { left, right, .. } => {
                self.condition(left, errors);
                self.condition(right, errors);
                Some(Ty::Bool)
            }
            Expr::Not(inner) => {
                self.condition(inner, errors);
                Some(Ty::Bool)
            }
            Expr::ArrayLiteral(items) => {
                if items.is_empty() {
                    errors.push(
                        "[] has no length a board can use — use அணி_நிரப்பு(value, count)"
                            .to_string(),
                    );
                    return None;
                }
                if items.len() > MAX_ARRAY as usize {
                    errors.push(format!(
                        "an array of {} items; a board array holds up to {}",
                        items.len(),
                        MAX_ARRAY
                    ));
                    return None;
                }
                let first = self.value(&items[0], errors)?;
                let Some(elem) = first.elem() else {
                    errors.push(format!(
                        "an array of {} — a board's arrays hold numbers, booleans, texts or records",
                        first.name()
                    ));
                    return None;
                };
                for item in &items[1..] {
                    if let Some(ty) = self.value(item, errors)
                        && ty != first
                    {
                        errors.push(format!("an array mixes {} and {}", first.name(), ty.name()));
                    }
                }
                Some(Ty::Array(elem, items.len() as u16))
            }
            Expr::Index { base, index } => {
                let base_ty = self.value(base, errors);
                self.number(index, "an index", errors);
                match base_ty {
                    Some(Ty::Array(elem, _)) => Some(elem.ty()),
                    // A letter, as the VM counts letters.
                    Some(Ty::Text) => Some(Ty::Text),
                    Some(other) => {
                        errors.push(format!("[…] needs an array, not {}", other.name()));
                        None
                    }
                    None => None,
                }
            }
            Expr::Call { name, args } => self.call(name, args, errors),
            Expr::ShapeLiteral {
                shape,
                fields,
                base,
                ..
            } => {
                let Some(id) = self.shape_id(shape) else {
                    errors.push(format!("{} is not a வடிவம் in this program", shape));
                    return None;
                };
                let mut given: Vec<String> = Vec::new();
                for (field, value) in fields {
                    let ty = self.value(value, errors);
                    match self.field(id, field) {
                        Some((index, _)) => {
                            if let Some(ty) = ty {
                                self.give_field(id, index, ty, errors);
                            }
                            given.push(field.clone());
                        }
                        None => errors.push(format!(
                            "{} has no field {}; its fields are {}",
                            shape,
                            field,
                            self.shapes[id as usize].1.join(", ")
                        )),
                    }
                }
                match base {
                    Some(base) => {
                        if let Some(ty) = self.value(base, errors)
                            && ty != Ty::Shape(id)
                        {
                            errors.push(format!(
                                "..{} needs a {} record, not {}",
                                shape,
                                shape,
                                ty.name()
                            ));
                        }
                    }
                    None => {
                        let missing: Vec<&String> = self.shapes[id as usize]
                            .1
                            .iter()
                            .filter(|f| !given.contains(f))
                            .collect();
                        if !missing.is_empty() {
                            errors.push(format!(
                                "a {} needs every one of its fields; missing: {}",
                                shape,
                                missing
                                    .iter()
                                    .map(|f| f.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                        }
                    }
                }
                Some(Ty::Shape(id))
            }
            Expr::Field { base, name, .. } => match self.value(base, errors)? {
                Ty::Shape(id) => match self.field(id, name) {
                    Some((_, ty)) => ty,
                    None => {
                        errors.push(format!("{} has no field {}", shape_name(id), name));
                        None
                    }
                },
                other => {
                    errors.push(format!(".{} needs a record, not {}", name, other.name()));
                    None
                }
            },
            Expr::MethodCall {
                receiver,
                name,
                args,
                ..
            } => {
                // கடன்.புதிது(…): the shape's own function, when கடன் is a
                // வடிவம் and not a variable.
                if let Expr::Variable(shape) = receiver.as_ref()
                    && self.lookup(shape).is_none()
                    && self.shape_id(shape).is_some()
                {
                    let types: Vec<Option<Ty>> =
                        args.iter().map(|a| self.value(a, errors)).collect();
                    let full = crate::vm::shape::method_function(shape, name);
                    return self.call_typed(&full, types, errors);
                }
                let receiver_ty = self.value(receiver, errors)?;
                let Ty::Shape(id) = receiver_ty else {
                    errors.push(format!(
                        ".{}(…) needs a record, not {}",
                        name,
                        receiver_ty.name()
                    ));
                    return None;
                };
                let full = crate::vm::shape::method_function(&self.shapes[id as usize].0, name);
                let mut types: Vec<Option<Ty>> = vec![Some(receiver_ty)];
                types.extend(args.iter().map(|a| self.value(a, errors)));
                self.call_typed(&full, types, errors)
            }
            Expr::Try(inner) => {
                if !self.in_function {
                    errors.push(
                        "? outside a செயல் — there is no caller to hand the தவறு to".to_string(),
                    );
                }
                self.tried = true;
                match self.value(inner, errors)? {
                    Ty::Result(Inner::Nothing) => Some(Ty::Void),
                    Ty::Result(held) => match held.ty() {
                        Some(ty) => Some(ty),
                        None => {
                            errors.push("? on a result whose சரி is unknown".to_string());
                            None
                        }
                    },
                    other => {
                        errors.push(format!("? needs a result, not {}", other.name()));
                        None
                    }
                }
            }
            _ => None,
        }
    }

    fn call(&mut self, name: &str, args: &[Expr], errors: &mut Vec<String>) -> Option<Ty> {
        if let Some(builtin) = builtin(name) {
            return self.builtin(builtin, name, args, errors);
        }
        let types: Vec<Option<Ty>> = args.iter().map(|a| self.value(a, errors)).collect();
        if let Some(board) = intrinsic(name) {
            if board.args.len() != args.len() {
                errors.push(format!(
                    "{} takes {} argument(s), given {}",
                    name,
                    board.args.len(),
                    args.len()
                ));
            }
            for (arg, ty) in board.args.iter().zip(&types) {
                let wanted = match arg {
                    Arg::Int => Ty::Num,
                    Arg::Flag => Ty::Bool,
                };
                if let Some(ty) = ty
                    && *ty != wanted
                {
                    errors.push(format!(
                        "{} wants {}, given {}",
                        name,
                        wanted.name(),
                        ty.name()
                    ));
                }
            }
            return Some(board.ret.ty());
        }
        if let Some(function) = manifest_function(name) {
            if function.args.len() != args.len() {
                errors.push(format!(
                    "{} takes {} argument(s), given {}",
                    name,
                    function.args.len(),
                    args.len()
                ));
            }
            for (i, ((kind, ty), arg)) in function.args.iter().zip(&types).zip(args).enumerate() {
                let wanted = match kind {
                    Kind::Bool => Ty::Bool,
                    Kind::Text => Ty::Text,
                    _ => Ty::Num,
                };
                if let Some(ty) = ty
                    && *ty != wanted
                {
                    errors.push(format!(
                        "{}: argument {} goes to C++ as {}, given {}",
                        name,
                        i + 1,
                        wanted.name(),
                        ty.name()
                    ));
                }
                // A whole number C++ wants, written as a fraction, is wrong now.
                if let (Kind::Int, Expr::Number(n)) = (kind, arg)
                    && !n.fract().is_zero()
                {
                    errors.push(format!(
                        "{}: argument {} goes to C++ as a whole number, and {} is not one",
                        name,
                        i + 1,
                        n
                    ));
                }
            }
            return Some(match function.returns {
                Kind::Void => Ty::Void,
                Kind::Bool => Ty::Bool,
                Kind::Text => Ty::Text,
                Kind::Int | Kind::Num => Ty::Num,
            });
        }
        self.call_typed(name, types, errors)
    }

    /// A call to a செயல் or a method, its arguments already typed.
    fn call_typed(
        &mut self,
        name: &str,
        types: Vec<Option<Ty>>,
        errors: &mut Vec<String>,
    ) -> Option<Ty> {
        if let Some(params) = self.signatures.get(name) {
            if params.len() != types.len() {
                errors.push(format!(
                    "{} takes {} argument(s), given {}",
                    name,
                    params.len(),
                    types.len()
                ));
            }
            {
                let mut observed = self.observed.borrow_mut();
                let seen = observed
                    .entry(name.to_string())
                    .or_insert_with(|| vec![None; params.len()]);
                for (i, ty) in types.iter().enumerate().take(params.len()) {
                    if let Some(ty) = ty {
                        match seen[i] {
                            None => seen[i] = Some(*ty),
                            Some(before) if unify(before, *ty).is_none() => errors.push(format!(
                                "{}: argument {} is given {} in one call and {} in another; on a board a parameter keeps one type",
                                name,
                                i + 1,
                                before.name(),
                                ty.name()
                            )),
                            Some(_) => {}
                        }
                    }
                }
            }
            for (i, (wanted, ty)) in params.iter().zip(&types).enumerate() {
                let Some(wanted) = wanted else { continue };
                if let Some(ty) = ty
                    && unify(*ty, *wanted).is_none()
                {
                    errors.push(format!(
                        "{}: argument {} should be {}, given {}",
                        name,
                        i + 1,
                        wanted.name(),
                        ty.name()
                    ));
                }
            }
            // None: recursion before its base case is typed, settled on a later pass.
            return self.returns.get(name).copied();
        }
        errors.push(format!(
            "{}(…) — neither a செயல் in this program nor a board function, and not a builtin artino builds",
            name
        ));
        None
    }

    fn builtin(
        &mut self,
        builtin: Builtin,
        name: &str,
        args: &[Expr],
        errors: &mut Vec<String>,
    ) -> Option<Ty> {
        let wanted = match builtin {
            Builtin::Board => 0,
            Builtin::Fill
            | Builtin::UnwrapOr
            | Builtin::Round
            | Builtin::SerialOpen
            | Builtin::SerialReadLine
            | Builtin::SerialWrite
            | Builtin::SerialWriteLine => 2,
            _ => 1,
        };
        if args.len() != wanted {
            errors.push(format!(
                "{} takes {} argument(s), given {}",
                name,
                wanted,
                args.len()
            ));
            return None;
        }
        match builtin {
            Builtin::Length => match self.value(&args[0], errors)? {
                // Letters, as the VM counts them: வரி is 2.
                Ty::Array(..) | Ty::Text => Some(Ty::Num),
                other => {
                    errors.push(format!("நீளம் needs an array, not {}", other.name()));
                    None
                }
            },
            Builtin::ToText => match self.value(&args[0], errors)? {
                Ty::Num | Ty::Bool | Ty::Text => Some(Ty::Text),
                other => {
                    errors.push(format!(
                        "{} of {} — print it on its own",
                        name,
                        other.name()
                    ));
                    None
                }
            },
            Builtin::Fill => {
                let ty = self.value(&args[0], errors)?;
                let Some(elem) = ty.elem() else {
                    errors.push(format!(
                        "அணி_நிரப்பு of {} — a board's arrays hold numbers, booleans, texts or records",
                        ty.name()
                    ));
                    return None;
                };
                match fill_count(&args[1]) {
                    Ok(count) => Some(Ty::Array(elem, count)),
                    Err(why) => {
                        errors.push(why);
                        None
                    }
                }
            }
            Builtin::Ok => {
                let ty = self.value(&args[0], errors)?;
                match Inner::of(ty) {
                    Some(inner) => Some(Ty::Result(inner)),
                    None => {
                        errors.push(format!(
                            "சரி({}) — a board's result holds a number, true or false, or text",
                            ty.name()
                        ));
                        None
                    }
                }
            }
            Builtin::Err => {
                let ty = self.value(&args[0], errors)?;
                if ty != Ty::Text {
                    errors.push(format!(
                        "தவறு({}) — on a board, a தவறு carries text",
                        ty.name()
                    ));
                }
                Some(Ty::Result(Inner::Unknown))
            }
            Builtin::IsOk | Builtin::IsErr => {
                self.result_arg(name, &args[0], errors)?;
                Some(Ty::Bool)
            }
            Builtin::Unwrap => match self.result_arg(name, &args[0], errors)? {
                Inner::Nothing => Some(Ty::Void),
                held => match held.ty() {
                    Some(ty) => Some(ty),
                    None => {
                        errors.push(format!("{} of a result whose சரி is unknown", name));
                        None
                    }
                },
            },
            Builtin::UnwrapErr => {
                self.result_arg(name, &args[0], errors)?;
                Some(Ty::Text)
            }
            Builtin::UnwrapOr => {
                let held = self.result_arg(name, &args[0], errors)?;
                let fallback = self.value(&args[1], errors)?;
                match held.ty() {
                    Some(ty) if ty != fallback => {
                        errors.push(format!(
                            "{}: the result holds {} but the fallback is {}",
                            name,
                            ty.name(),
                            fallback.name()
                        ));
                        None
                    }
                    _ => Some(fallback),
                }
            }
            Builtin::ToNumber => {
                let ty = self.value(&args[0], errors)?;
                if ty != Ty::Text {
                    errors.push(format!("{} reads text, not {}", name, ty.name()));
                }
                Some(Ty::Result(Inner::Num))
            }
            Builtin::Floor | Builtin::Ceil => {
                self.number(&args[0], name, errors);
                Some(Ty::Num)
            }
            Builtin::Round => {
                self.number(&args[0], name, errors);
                if let Err(why) = places(&args[1]) {
                    errors.push(why);
                }
                Some(Ty::Num)
            }
            Builtin::Board => Some(Ty::Text),
            Builtin::SerialOpen => {
                self.number(&args[0], "a serial port", errors);
                self.number(&args[1], "a baud rate", errors);
                Some(Ty::Result(Inner::Num))
            }
            Builtin::SerialReadLine => {
                self.number(&args[0], "a serial port", errors);
                self.number(&args[1], "a wait in milliseconds", errors);
                Some(Ty::Result(Inner::Text))
            }
            Builtin::SerialWrite | Builtin::SerialWriteLine => {
                self.number(&args[0], "a serial port", errors);
                if let Some(ty) = self.value(&args[1], errors)
                    && ty != Ty::Text
                {
                    errors.push(format!("{} writes text, not {}", name, ty.name()));
                }
                Some(Ty::Result(Inner::Num))
            }
            Builtin::SerialClose => {
                self.number(&args[0], "a serial port", errors);
                Some(Ty::Result(Inner::Nothing))
            }
        }
    }

    fn result_arg(&mut self, name: &str, arg: &Expr, errors: &mut Vec<String>) -> Option<Inner> {
        match self.value(arg, errors)? {
            Ty::Result(inner) => Some(inner),
            other => {
                errors.push(format!("{} needs a result, not {}", name, other.name()));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(source: &str) -> Vec<Stmt> {
        crate::module::load_source(source, std::path::Path::new(".")).expect("parses")
    }

    fn errors(source: &str) -> Vec<String> {
        analyse(&load(source)).err().unwrap_or_default()
    }

    #[test]
    fn a_blink_is_setup_one_schedule_and_one_global() {
        let program = analyse(&load(
            "இறக்கு \"nUlakam/vaZporuL/vaZporuL.qmz\";\n\
             நிலை எண் விளக்கு = 13;\n\
             முனை_வெளியீடு(விளக்கு);\n\
             இடைவெளி 0.5 { முனை_மாற்று(விளக்கு); }",
        ))
        .expect("builds");
        assert_eq!(program.schedules.len(), 1);
        assert_eq!(program.schedules[0].ms, 500);
        assert_eq!(program.global("விளக்கு"), Some(Ty::Num));
        // vaZporuL's own definitions are the board's, not compiled.
        assert!(program.functions.is_empty());
    }

    #[test]
    fn names_assigned_in_a_block_are_the_programs() {
        let program = analyse(&load("இடைவெளி 1 { எண்ணி = 0; தீ = பொய்; }")).expect("builds");
        assert_eq!(program.global("எண்ணி"), Some(Ty::Num));
        assert_eq!(program.global("தீ"), Some(Ty::Bool));
    }

    #[test]
    fn return_types_are_inferred_through_recursion() {
        let program = analyse(&load(
            "செயல் காரணீயம்(ந) { (ந <= 1) எனில் { திரும்பு 1; } திரும்பு ந * காரணீயம்(ந - 1); }\n\
             செயல் பெரியதா(ந) { திரும்பு ந > 100; }\n\
             அ = காரணீயம்(5); ஆ = பெரியதா(அ);",
        ))
        .expect("builds");
        assert_eq!(program.function("காரணீயம்").unwrap().ret, Ty::Num);
        assert_eq!(program.function("பெரியதா").unwrap().ret, Ty::Bool);
        assert_eq!(program.global("ஆ"), Some(Ty::Bool));
    }

    #[test]
    fn a_function_reads_a_global_it_cannot_change() {
        let program = analyse(&load(
            "எல்லை = 150;\nசெயல் மேலா(அ) { எல்லை = 0; திரும்பு அ > எல்லை; }\nஇ = மேலா(200);",
        ))
        .expect("builds");
        // The assignment inside the function made a local, as on the VM.
        let function = program.function("மேலா").unwrap();
        assert!(function.locals.iter().any(|(n, _)| n == "எல்லை"));
        assert_eq!(function.ret, Ty::Bool);
    }

    #[test]
    fn what_a_function_writes_and_what_holds_a_letter() {
        let program = analyse(&load(
            "செயல் எண்ணு(சரம், தேடல்) {
                 கண்டது = 0;
                 ஒவ்வொரு எ இல் சரம் { (எ == தேடல்) எனில் { கண்டது = கண்டது + 1; } }
                 ஒவ்வொரு ஏ இல் சரம் { ஏ = ஏ & \"!\"; }
                 திரும்பு கண்டது;
             }
             அ = எண்ணு(\"அஆஅ\", \"அ\");",
        ))
        .expect("builds");
        let function = program
            .functions
            .iter()
            .find(|f| f.name == "எண்ணு")
            .expect("reached");
        // Neither parameter is written, so both can be read where the caller holds them.
        assert!(!function.written.contains("சரம்") && !function.written.contains("தேடல்"));
        assert!(function.written.contains("கண்டது") && function.written.contains("எ"));
        // எ only ever holds a letter; ஏ is also given longer text.
        assert!(function.letters.contains("எ"), "{:?}", function.letters);
        assert!(!function.letters.contains("ஏ"), "{:?}", function.letters);
    }

    #[test]
    fn a_manifest_s_functions_are_typed_at_the_boundary() {
        use crate::artino::manifest::{Extern, Kind};
        let externs = [Extern {
            etamil: "கதவு".into(),
            cpp: "servo.write".into(),
            args: vec![Kind::Int],
            returns: Kind::Int,
        }];
        let program = analyse_with(&load("அ = கதவு(90);"), &externs).expect("builds");
        assert_eq!(program.global("அ"), Some(Ty::Num));
        let found =
            analyse_with(&load("அ = கதவு(1.5);\nஆ = கதவு(\"x\");"), &externs).expect_err("refused");
        assert!(
            found.iter().any(|f| f.contains("1.5 is not one")),
            "{:?}",
            found
        );
        assert!(
            found
                .iter()
                .any(|f| f.contains("argument 1 goes to C++ as a number, given text")),
            "{:?}",
            found
        );
        // Without the manifest it is an unknown function.
        assert!(analyse(&load("அ = கதவு(90);")).is_err());
    }

    #[test]
    fn unreached_functions_are_not_judged() {
        // Helpers using records are fine as long as nothing calls them.
        assert!(analyse(&load("செயல் பதிவு() { திரும்பு {அ: 1}; }\nஅ = 1;")).is_ok());
    }

    #[test]
    fn what_b3_1_refuses_is_named() {
        let found = refusals(&load("அ = {பெயர்: 1};\nஆ = இன்மை;"));
        assert!(
            found.iter().any(|f| f.contains("record without a வடிவம்")),
            "{:?}",
            found
        );
        assert!(found.iter().any(|f| f.contains("இன்மை")), "{:?}", found);
    }

    #[test]
    fn a_board_never_pauses() {
        let found = refusals(&load(
            "இறக்கு \"nUlakam/vaZporuL/vaZporuL.qmz\";\nகாத்திரு(100);",
        ));
        assert!(found.iter().any(|f| f.contains("காத்திரு")), "{:?}", found);
    }

    #[test]
    fn four_decimals_are_refused_three_are_not() {
        assert!(refusals(&load("அ = 1.25;\nஆ = 18%;")).is_empty());
        let found = refusals(&load("அ = 1.2345;"));
        assert!(
            found.iter().any(|f| f.contains("three decimal")),
            "{:?}",
            found
        );
    }

    #[test]
    fn type_mistakes_are_reported() {
        let found = errors("அ = 1;\nஅ = பொய்;");
        assert!(
            found.iter().any(|e| e.contains("keeps one type")),
            "{:?}",
            found
        );
        let found = errors("(5) எனில் { அ = 1; }");
        assert!(
            found
                .iter()
                .any(|e| e.contains("condition must be true or false")),
            "{:?}",
            found
        );
        let found = errors("அ = 1 + மெய்;");
        assert!(
            found.iter().any(|e| e.contains("numbers on both sides")),
            "{:?}",
            found
        );
        let found = errors("இறக்கு \"nUlakam/vaZporuL/vaZporuL.qmz\";\nமுனை_எழுது(13, 1);");
        assert!(
            found.iter().any(|e| e.contains("wants true or false")),
            "{:?}",
            found
        );
        let found = errors("அ = இல்லாதது(1);");
        assert!(
            found.iter().any(|e| e.contains("neither a செயல்")),
            "{:?}",
            found
        );
    }

    #[test]
    fn literals_scale_by_a_thousand() {
        assert_eq!(scaled(&Decimal::new(15, 1)), Ok(1500));
        assert_eq!(scaled(&Decimal::new(18, 2)), Ok(180));
        assert_eq!(scaled(&Decimal::new(-3, 0)), Ok(-3000));
        assert!(scaled(&Decimal::new(12345, 4)).is_err());
    }

    #[test]
    fn arrays_take_their_length_from_the_source() {
        let program = analyse(&load(
            "இறக்கு \"nUlakam/atippatY/aNi.qmz\";\n\
             அ = [1, 2, 3];\nஆ = அணி_நிரப்பு(பொய், 5);\nஇ = அ[0] + நீளம்(ஆ);\n\
             ஒவ்வொரு உ இல் அ { ஊ = உ; }\nஅ[1] = 9;",
        ))
        .expect("builds");
        assert_eq!(program.global("அ"), Some(Ty::Array(Elem::Num, 3)));
        assert_eq!(program.global("ஆ"), Some(Ty::Array(Elem::Bool, 5)));
        assert_eq!(program.global("உ"), Some(Ty::Num));
        // aNi.qmz's own definitions are not compiled: அணி_நிரப்பு is artino's.
        assert!(program.functions.is_empty());
    }

    #[test]
    fn array_mistakes_are_reported() {
        let found = errors("அ = [];");
        assert!(
            found.iter().any(|e| e.contains("அணி_நிரப்பு")),
            "{:?}",
            found
        );
        let found = errors("அ = [1, மெய்];");
        assert!(found.iter().any(|e| e.contains("mixes")), "{:?}", found);
        let found = errors("அ = [1, 2];\nஅ = [1, 2, 3];");
        assert!(
            found.iter().any(|e| e.contains("keeps one type")),
            "{:?}",
            found
        );
        let found = errors("இறக்கு \"nUlakam/atippatY/aNi.qmz\";\nந = 3;\nஅ = அணி_நிரப்பு(0, ந);");
        assert!(
            found.iter().any(|e| e.contains("written in the source")),
            "{:?}",
            found
        );
        let found = errors("அ = [1];\nசெயல் மாற்று() { அ[0] = 2; திரும்பு 0; }\nஆ = மாற்று();");
        assert!(found.iter().any(|e| e.contains("cannot")), "{:?}", found);
    }

    #[test]
    fn text_is_a_value() {
        let program = analyse(&load(
            "செயல் வணக்கம்(சொல் பெயர்) { திரும்பு \"வணக்கம் \" & பெயர்; }\n\
             அ = வணக்கம்(\"உலகம்\");\nஆ = அ == \"x\";\nஇ = சொல்லாக்கு(1.5);",
        ))
        .expect("builds");
        assert_eq!(program.global("அ"), Some(Ty::Text));
        assert_eq!(program.global("ஆ"), Some(Ty::Bool));
        assert_eq!(
            program.function("வணக்கம்").unwrap().params,
            vec![("பெயர்".to_string(), Ty::Text)]
        );
        // Letters, as the VM counts them: நீளம் of text is a number now.
        let program = analyse(&load("அ = நீளம்(\"வரி\");")).expect("builds");
        assert_eq!(program.global("அ"), Some(Ty::Num));
    }
}
