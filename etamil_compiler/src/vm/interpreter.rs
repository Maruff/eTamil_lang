// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
// eTamil Virtual Machine Interpreter
// Executes bytecode independently without compilation

use std::collections::HashMap;
// Direct filesystem access survives only in the package and archive helpers,
// which hand a File to the zip crate and are gated out of a wasm build.
// Everything else goes through vm::host, so the VM can run in a browser and be
// tested without touching a disk.
#[cfg(not(target_family = "wasm"))]
use std::fs;
// Only reached from package_copy, which a wasm build gates out.
use crate::vm::bytecode::FunctionInfo;
use crate::vm::host;
use crate::vm::value::FunctionValue;
use crate::vm::{Bytecode, Instruction, Value};
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
#[cfg(not(target_family = "wasm"))]
use std::io::Write as IoWrite;
use std::str::FromStr;
use unicode_segmentation::UnicodeSegmentation;

/// Split text the way a reader would: by written letter. A Tamil letter is
/// frequently several code points (consonant + vowel sign, or + pulli), so
/// counting chars would make நீளம்("வணக்கம்") 7 instead of 5.
fn letters(text: &str) -> Vec<&str> {
    text.graphemes(true).collect()
}

/// Byte offsets where a written letter begins, plus the end of the text. A
/// separator counts as found only when both its ends land on one of these:
/// otherwise பிரி("கா", "ா") would cut a single letter in half, a position
/// the language does not admit anywhere else.
fn cluster_edges(text: &str) -> std::collections::BTreeSet<usize> {
    text.grapheme_indices(true)
        .map(|(at, _)| at)
        .chain(std::iter::once(text.len()))
        .collect()
}

/// Byte offsets of every non-overlapping `needle` that begins and ends on a
/// letter boundary. One segmentation pass, then a byte search — the pair that
/// lets மாற்று and பிரி run over a whole document instead of re-segmenting
/// the string once per position, as reading it letter by letter did.
fn cluster_matches(haystack: &str, needle: &str) -> Vec<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return Vec::new();
    }
    let edges = cluster_edges(haystack);
    haystack
        .match_indices(needle)
        .map(|(at, _)| at)
        .filter(|at| edges.contains(at) && edges.contains(&(at + needle.len())))
        .collect()
}

/// A file that arrived in a multipart request.
///
/// The bytes stay here rather than becoming a சரம். A handler is told what
/// was uploaded and decides where to put it; பதிவேற்றம்_சேமி does the writing.
/// Nothing that is not text ever becomes a value the language can hold, which
/// is the same rule the ODF package builtins follow.
#[derive(Debug, Clone)]
pub struct Upload {
    pub name: String,
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

/// One active function call: where to resume, and that call's local names.
#[derive(Debug)]
pub struct Frame {
    pub return_ip: usize,
    pub locals: HashMap<String, Value>,
    /// Stack depth when the call began. Returning truncates back to this, so
    /// a half-evaluated expression abandoned by `?` cannot leave residue.
    pub base_len: usize,
}

/// Guards against runaway recursion before the host stack is exhausted.
const MAX_CALL_DEPTH: usize = 256;

/// Database connections this VM has borrowed, keyed by the type name written
/// in source. Wrapped so the VM can still derive Debug — a driver handle
/// cannot.
///
/// These are *leases*, not owned connections: dropping one hands it back to
/// the process-wide idle cache instead of closing it, so the next request does
/// not pay for a connect, a TLS handshake and an authentication round trip.
/// A lease is exclusive for as long as it is held, which is what keeps one
/// request's transaction out of another's — see db::pool.
/// One open database: the handle, and which database it is.
///
/// The connection string is kept so that connecting again can tell "the same
/// database" from "a different one through the same driver". Without it the
/// second case was invisible — see `Connections::insert`.
pub struct Open {
    connection: String,
    lease: crate::db::pool::Lease,
}

#[derive(Default)]
pub struct Connections(HashMap<String, Open>);

impl std::fmt::Debug for Connections {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Connections({} open)", self.0.len())
    }
}

impl Connections {
    pub fn insert(&mut self, name: String, connection: String, lease: crate::db::pool::Lease) {
        self.0.insert(name, Open { connection, lease });
    }

    /// Which database is open through this driver, if one is.
    pub fn connection_of(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(|open| open.connection.as_str())
    }

    pub fn remove(&mut self, name: &str) -> Option<crate::db::pool::Lease> {
        self.0.remove(name).map(|open| open.lease)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The handles that are open, for an error that has to say which.
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.0.keys().map(|name| name.as_str()).collect();
        names.sort();
        names
    }
}

#[derive(Debug)]
pub struct VM {
    pub stack: Vec<Value>,
    pub variables: HashMap<String, Value>,
    pub instruction_pointer: usize,
    /// Mode ("read" / "write" / "append") recorded by கோப்பு_திற per file.
    pub file_modes: HashMap<String, String>,
    /// Active call frames; empty means we are at global scope.
    pub frames: Vec<Frame>,
    /// Open database connections.
    pub connections: Connections,
    /// Files that came with this request, in the order they were sent.
    /// Empty except while a multipart request is being handled.
    pub uploads: Vec<Upload>,
    /// An open MongoDB connection, if the program asked for one.
    ///
    /// Behind the feature, so a default build carries neither the field nor
    /// the seventy crates the driver brings.
    #[cfg(feature = "mongodb")]
    pub documents: Option<crate::mongo::Connection>,
    /// An open Redis connection, if the program asked for one.
    ///
    /// Not pooled, and deliberately. Redis keeps state on a connection —
    /// MULTI, WATCH, SUBSCRIBE — so two requests sharing one would interleave
    /// a transaction the way two requests sharing a SQL connection do. The fix
    /// is an exclusive lease, which the SQL side has and this does not yet.
    pub cache: Option<crate::redis::Connection>,
    /// The broker, held for the life of the program for the reason `cache`
    /// is: a channel carries per-channel state, so sharing one between two
    /// requests has the hazard of sharing a transaction.
    #[cfg(not(target_family = "wasm"))]
    pub broker: Option<crate::amqp::Connection>,
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            variables: HashMap::new(),
            instruction_pointer: 0,
            file_modes: HashMap::new(),
            uploads: Vec::new(),
            #[cfg(feature = "mongodb")]
            documents: None,
            cache: None,
            #[cfg(not(target_family = "wasm"))]
            broker: None,
            frames: Vec::new(),
            connections: Connections::default(),
        }
    }

    /// The connection to use for a query. There is one per database type, and
    /// with a single type open the choice is unambiguous.
    /// The connection a statement means.
    ///
    /// Named, and it is that one. Unnamed, and it is the only open one — which
    /// is every program written before handles existed. Unnamed with several
    /// open is refused: guessing which of two databases a query meant is the
    /// kind of wrong answer this project does not give.
    /// Public for the same reason `invoke_builtin` and `index_of` are: the
    /// LLVM backend's runtime reaches the same object rather than keeping a
    /// second registry that would agree until somebody edited one of them.
    pub fn connection_for(
        &mut self,
        handle: Option<&str>,
    ) -> Result<&mut dyn crate::db::Database, String> {
        if let Some(handle) = handle {
            return match self.connections.0.get_mut(handle) {
                Some(open) => Ok(open.lease.connection()),
                None => Err(format!(
                    "'{}' என்ற இணைப்பு இல்லை  (there is no connection named '{}'): \
                     open one with தளம்_இணை … , {}",
                    handle, handle, handle
                )),
            };
        }

        if self.connections.0.len() == 1 {
            return Ok(self
                .connections
                .0
                .values_mut()
                .next()
                .expect("checked")
                .lease
                .connection());
        }
        if self.connections.is_empty() {
            return Err("தரவுத்தளம் இணைக்கப்படவில்லை  (not connected to a database): \
                 use தளம்_இணை first"
                .to_string());
        }
        Err(format!(
            "பல இணைப்புகள் திறந்துள்ளன  (several connections are open: {}); \
             name the one you mean as a last argument",
            self.connections.names().join(", ")
        ))
    }

    /// The only open connection, for the statements that take no handle yet.
    fn connection_mut(&mut self) -> Result<&mut dyn crate::db::Database, String> {
        self.connection_for(None)
    }

    /// Read a name: the current call's locals shadow globals.
    fn get_var(&self, name: &str) -> Option<Value> {
        if let Some(frame) = self.frames.last()
            && let Some(value) = frame.locals.get(name)
        {
            return Some(value.clone());
        }
        self.variables.get(name).cloned()
    }

    /// Look at a name's value without copying it.
    fn peek_var(&self, name: &str) -> Option<&Value> {
        if let Some(frame) = self.frames.last()
            && let Some(value) = frame.locals.get(name)
        {
            return Some(value);
        }
        self.variables.get(name)
    }

    /// The last `count` values on the stack, in the order they were pushed.
    fn pop_args(&mut self, count: usize) -> Result<Vec<Value>, String> {
        if self.stack.len() < count {
            return Err("Stack underflow".to_string());
        }
        Ok(self.stack.split_off(self.stack.len() - count))
    }

    /// Enter a function's body: bind its captures and parameters in a new
    /// frame and move to its first instruction.
    fn enter(
        &mut self,
        info: &FunctionInfo,
        name: &str,
        captured: Vec<Value>,
        args: Vec<Value>,
    ) -> Result<(), String> {
        if info.params.len() != args.len() {
            let shown = FunctionValue::shown(name);
            return Err(format!(
                "செயல் {} {} அளவுருக்களை எதிர்பார்க்கிறது, {} வழங்கப்பட்டது  \
                 (function {} expects {} argument(s), got {})",
                shown,
                info.params.len(),
                args.len(),
                shown,
                info.params.len(),
                args.len()
            ));
        }
        if self.frames.len() >= MAX_CALL_DEPTH {
            return Err(format!(
                "செயல் அழைப்பு ஆழம் மிகுதி ({})  (call depth exceeded — infinite recursion?)",
                MAX_CALL_DEPTH
            ));
        }

        let mut locals = HashMap::with_capacity(info.captures.len() + info.params.len());
        let mut carried = captured.into_iter();
        for capture in &info.captures {
            locals.insert(capture.clone(), carried.next().unwrap_or(Value::Null));
        }
        for (param, value) in info.params.iter().zip(args) {
            locals.insert(param.clone(), value);
        }

        self.frames.push(Frame {
            return_ip: self.instruction_pointer + 1,
            locals,
            base_len: self.stack.len(),
        });
        self.instruction_pointer = info.start;
        Ok(())
    }

    /// Call a function value.
    ///
    /// Answers true when a body was entered — a frame pushed and the
    /// instruction pointer moved — and false for a builtin, whose answer has
    /// already been pushed.
    fn call_function(
        &mut self,
        bytecode: &Bytecode,
        function: FunctionValue,
        args: Vec<Value>,
    ) -> Result<bool, String> {
        if let Some(info) = bytecode.functions.get(function.name.as_str()) {
            self.enter(info, &function.name, function.captured, args)?;
            return Ok(true);
        }
        if crate::vm::is_builtin(&function.name) {
            let answer = self.invoke_builtin(&function.name, args)?;
            self.stack.push(answer);
            return Ok(false);
        }
        Err(format!(
            "அறியப்படாத செயல் {}  (unknown function {})",
            FunctionValue::shown(&function.name),
            FunctionValue::shown(&function.name)
        ))
    }

    /// Write a name. Inside a function this always creates or updates a
    /// local, so a function cannot silently clobber a global — assigning to
    /// an outer name shadows it for the duration of the call.
    ///
    /// It takes `&str` rather than `String` because only a *new* binding has to
    /// own its key. Rebinding an existing one does not, and rebinding is what a
    /// loop does on every pass: `i = i + 1` was allocating a fresh copy of
    /// `"i"` per iteration and dropping the old one.
    fn set_var(&mut self, name: &str, value: Value) {
        let scope = match self.frames.last_mut() {
            Some(frame) => &mut frame.locals,
            None => &mut self.variables,
        };
        match scope.get_mut(name) {
            Some(slot) => *slot = value,
            None => {
                scope.insert(name.to_string(), value);
            }
        }
    }

    /// Pop a value, or report a stack underflow.
    fn pop(&mut self) -> Result<Value, String> {
        self.stack
            .pop()
            .ok_or_else(|| "Stack underflow".to_string())
    }

    /// Human-readable type name, for error messages. Public so the LLVM
    /// backend's runtime words the same failure the same way.
    pub fn type_name(value: &Value) -> &'static str {
        match value {
            Value::Number(_) => "a number",
            Value::String(_) => "a string",
            Value::Boolean(_) => "a boolean",
            Value::Array(_) => "an array",
            Value::Map(_) => "a record",
            Value::Ok(_) => "a result",
            Value::Err(_) => "a result",
            Value::Function(_) => "a function",
            Value::Null => "nil",
        }
    }

    /// Turn an index value into a valid array position, or explain why not.
    fn array_index(len: usize, index: &Value) -> Result<usize, String> {
        let raw = index.to_number();
        if raw.fract() != Decimal::ZERO {
            return Err(format!(
                "அட்டவணை முழு எண்ணாக இருக்க வேண்டும்  (array index must be a whole number, got {})",
                raw
            ));
        }
        let i = rust_decimal::prelude::ToPrimitive::to_i64(&raw).unwrap_or(-1);
        if i < 0 || i as usize >= len {
            return Err(format!(
                "அட்டவணை {} வரம்பிற்கு வெளியே (நீளம் {})  (index {} out of bounds, length {})",
                raw, len, raw, len
            ));
        }
        Ok(i as usize)
    }

    /// `base[index]` for arrays (by position) and records (by key).
    /// How many times `ஒவ்வொரு` goes round. An array's items, a record's
    /// fields, or a string's *letters* — Tamil letters, which is not the same
    /// count as chars, because a letter is a cluster.
    ///
    /// Public for the same reason `index_of` is: the LLVM backend's emitted IR
    /// calls this rather than counting for itself.
    pub fn length_of(value: &Value) -> Result<usize, String> {
        match value {
            Value::Array(items) => Ok(items.len()),
            Value::Map(fields) => Ok(fields.len()),
            Value::String(s) => Ok(letters(s).len()),
            other => Err(format!(
                "இதை சுற்ற முடியாது  (cannot iterate over {})",
                Self::type_name(other)
            )),
        }
    }

    /// What `ஒவ்வொரு` binds on each turn.
    ///
    /// An array gives its item. A record gives its **key**, not its value, and
    /// the keys are sorted so that iteration order is the same run to run. A
    /// string gives one letter.
    pub fn nth_or_key(base: &Value, index: &Value) -> Result<Value, String> {
        match base {
            Value::Array(items) => {
                let i = Self::array_index(items.len(), index)?;
                Ok(items[i].clone())
            }
            Value::Map(fields) => {
                // Sorted so iteration order is stable run to run.
                let mut keys: Vec<&String> = fields.keys().collect();
                keys.sort();
                let i = Self::array_index(keys.len(), index)?;
                Ok(Value::String(keys[i].clone()))
            }
            Value::String(s) => {
                let parts = letters(s);
                let i = Self::array_index(parts.len(), index)?;
                Ok(Value::String(parts[i].to_string()))
            }
            other => Err(format!(
                "இதை சுற்ற முடியாது  (cannot iterate over {})",
                Self::type_name(other)
            )),
        }
    }

    /// Public because `crate::runtime` reaches for it: the LLVM backend's
    /// emitted IR indexes through the same function the VM uses, rather than
    /// through a second implementation that would drift from it.
    /// Indexed assignment, shared with the LLVM backend's runtime.
    ///
    /// `index_of` below has always been shared, so reads could not drift apart.
    /// This one was not, and it drifted: the runtime handled `Value::Array` and
    /// nothing else, so `record[key] = value` aborted a compiled program with
    /// "expected an array" while the VM inserted the field. Every program that
    /// built a record in a loop was affected, and the parity job found it in
    /// upi/vilAcam.qmz reading a upi:// link back into a record.
    ///
    /// `name` is only for the message. The VM has it from the instruction and
    /// the backend passes it in, so both say the same thing when the base turns
    /// out to be something that cannot be indexed at all.
    ///
    /// A record made as a வடிவம் takes only the fields its shape has, which
    /// is why the shapes come too.
    pub fn index_assign(
        base: &mut Value,
        index: &Value,
        value: Value,
        name: &str,
        shapes: &crate::vm::shape::Shapes,
    ) -> Result<(), String> {
        match base {
            Value::Array(items) => {
                let i = Self::array_index(items.len(), index)?;
                items[i] = value;
                Ok(())
            }
            Value::Map(fields) => {
                let key = index.to_string();
                crate::vm::shape::check_set(shapes, fields, &key, &value)?;
                fields.insert(key, value);
                Ok(())
            }
            other => Err(format!(
                "'{}' ஐ அட்டவணைப்படுத்த முடியாது  (cannot index into {})",
                name,
                Self::type_name(other)
            )),
        }
    }

    pub fn index_of(base: &Value, index: &Value) -> Result<Value, String> {
        match base {
            Value::Array(items) => {
                let i = Self::array_index(items.len(), index)?;
                Ok(items[i].clone())
            }
            Value::Map(fields) => {
                let key = index.to_string();
                fields.get(&key).cloned().ok_or_else(|| {
                    format!("புலம் '{}' இல்லை  (no field '{}' on this record)", key, key)
                })
            }
            Value::String(s) => {
                let parts = letters(s);
                let i = Self::array_index(parts.len(), index)?;
                Ok(Value::String(parts[i].to_string()))
            }
            other => Err(format!(
                "அட்டவணைப்படுத்த முடியாது  (cannot index into {})",
                Self::type_name(other)
            )),
        }
    }

    /// One builtin, by name, with its arguments already in hand.
    ///
    /// `call_builtin` below takes them off the stack because that is what the
    /// bytecode gives it. The LLVM backend has them as values, and calls this:
    /// the point is that both backends reach the *same* builtins, so a compiled
    /// program cannot answer differently from an interpreted one because someone
    /// reimplemented நீளம் slightly differently.
    pub fn invoke_builtin(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        let argc = args.len();
        for argument in args {
            self.stack.push(argument);
        }
        self.call_builtin(name, argc)
    }

    /// Builtins, callable under Tamil, romanized or English names. This is
    /// the extension point the tax and accounting builtins will plug into.
    // rustfmt is off here on purpose: scripts/generate_editor_support.py reads
    // these arms a line at a time, and wrapping one across lines makes it
    // invisible to the generator — which then writes editor data missing
    // whatever it could not see, and CI gates on that file.
    #[rustfmt::skip]
    fn call_builtin(&mut self, name: &str, argc: usize) -> Result<Value, String> {
        let mut args = Vec::with_capacity(argc);
        for _ in 0..argc {
            args.push(self.pop()?);
        }
        args.reverse();

        match name {
            // --- the board: nUlakam/vaZporuL/vaZporuL.qmz calls these, by their
            // English names; src/vm/board.rs is the board itself. Pins and time
            // give results a wrapper unwraps; serial ports give results the
            // program handles.
            // வன்_பலகை() — "sim", "pi" or "host"
            "வன்_பலகை" | "vaZ_palakY" | "_boardName" => {
                Self::expect_args(name, &args, 0)?;
                Ok(Value::String(super::board::name().to_string()))
            }
            // வன்_முனை_வகை(முனை, "out" | "in" | "in_pullup")
            "வன்_முனை_வகை" | "vaZ_muZY_vakY" | "_pinMode" => {
                Self::expect_args(name, &args, 2)?;
                let pin = Self::whole(&args[0], "முனை (pin)")?;
                Ok(Self::board_result(super::board::pin_mode(pin, &args[1].to_string()).map(|_| Value::Null)))
            }
            // வன்_முனை_எழுது(முனை, 0 | 1)
            "வன்_முனை_எழுது" | "vaZ_muZY_ezuqu" | "_pinWrite" => {
                Self::expect_args(name, &args, 2)?;
                let pin = Self::whole(&args[0], "முனை (pin)")?;
                let high = Self::whole(&args[1], "மட்டம் (level)")? != 0;
                Ok(Self::board_result(super::board::pin_write(pin, high).map(|_| Value::Null)))
            }
            // வன்_முனை_படி(முனை) — சரி(0 | 1)
            "வன்_முனை_படி" | "vaZ_muZY_pati" | "_pinRead" => {
                Self::expect_args(name, &args, 1)?;
                let pin = Self::whole(&args[0], "முனை (pin)")?;
                Ok(Self::board_result(super::board::pin_read(pin).map(|h| Value::Number(Decimal::from(u8::from(h))))))
            }
            // வன்_ஒப்புமை_படி(முனை) — சரி(0 to 1023)
            "வன்_ஒப்புமை_படி" | "vaZ_oppumY_pati" | "_analogRead" => {
                Self::expect_args(name, &args, 1)?;
                let pin = Self::whole(&args[0], "முனை (pin)")?;
                Ok(Self::board_result(super::board::analog_read(pin).map(|v| Value::Number(Decimal::from(v)))))
            }
            // வன்_மில்லி() — milliseconds since the program began
            "வன்_மில்லி" | "vaZ_milli" | "_millis" => {
                Self::expect_args(name, &args, 0)?;
                Ok(Value::Number(Decimal::from(super::board::millis())))
            }
            // வன்_காத்திரு(மில்லி) — wait; on the simulated board, move its clock
            "வன்_காத்திரு" | "vaZ_kAqqiru" | "_sleepMs" => {
                Self::expect_args(name, &args, 1)?;
                let ms = Self::whole(&args[0], "மில்லி (milliseconds)")?.max(0) as u64;
                super::board::sleep_ms(ms);
                Ok(Value::Ok(Box::new(Value::Null)))
            }
            // வன்_ஒலி(முனை, அதிர்வெண்) — a tone; only the simulated board has one
            "வன்_ஒலி" | "vaZ_oli" | "_tone" => {
                Self::expect_args(name, &args, 2)?;
                let pin = Self::whole(&args[0], "முனை (pin)")?;
                let hz = Self::whole(&args[1], "அதிர்வெண் (frequency)")?;
                Ok(Self::board_result(super::board::tone(pin, Some(hz)).map(|_| Value::Null)))
            }
            // வன்_ஒலி_நிறுத்து(முனை)
            "வன்_ஒலி_நிறுத்து" | "vaZ_oli_niRuqqu" | "_noTone" => {
                Self::expect_args(name, &args, 1)?;
                let pin = Self::whole(&args[0], "முனை (pin)")?;
                Ok(Self::board_result(super::board::tone(pin, None).map(|_| Value::Null)))
            }
            // வன்_காவல்(மில்லி) — a board's watchdog; nothing on the VM
            "வன்_காவல்" | "vaZ_kAval" | "_watchdogBegin" => {
                Self::expect_args(name, &args, 1)?;
                Self::whole(&args[0], "மில்லி (milliseconds)")?;
                Ok(Value::Ok(Box::new(Value::Null)))
            }
            // வன்_காவல்_புதுப்பி()
            "வன்_காவல்_புதுப்பி" | "vaZ_kAval_puquppi" | "_watchdogFeed" => {
                Self::expect_args(name, &args, 0)?;
                Ok(Value::Ok(Box::new(Value::Null)))
            }
            // வன்_தொடர்_திற(சாதனம், வேகம்) — சரி(port) or தவறு(why)
            "வன்_தொடர்_திற" | "vaZ_qotar_qiRa" | "_serialOpen" => {
                Self::expect_args(name, &args, 2)?;
                let baud = Self::whole(&args[1], "வேகம் (baud)")?;
                let opened = super::board::serial_open(&args[0].to_string(), baud.clamp(0, u32::MAX as i64) as u32);
                Ok(Self::board_result(opened.map(|port| Value::Number(Decimal::from(port)))))
            }
            // வன்_தொடர்_படி(துறை, காலம்) — சரி(line), சரி(இன்மை) if none yet, or தவறு(why)
            "வன்_தொடர்_படி" | "vaZ_qotar_pati" | "_serialReadLine" => {
                Self::expect_args(name, &args, 2)?;
                let port = Self::whole(&args[0], "துறை (port)")?;
                let wait = Self::whole(&args[1], "காலம் (wait)")?.max(0) as u64;
                let line = super::board::serial_read_line(port, wait);
                Ok(Self::board_result(line.map(|l| l.map(Value::String).unwrap_or(Value::Null))))
            }
            // வன்_தொடர்_எழுது(துறை, செய்தி) — சரி(bytes written)
            "வன்_தொடர்_எழுது" | "vaZ_qotar_ezuqu" | "_serialWrite" => {
                Self::expect_args(name, &args, 2)?;
                let port = Self::whole(&args[0], "துறை (port)")?;
                let written = super::board::serial_write(port, &args[1].to_string());
                Ok(Self::board_result(written.map(|n| Value::Number(Decimal::from(n)))))
            }
            // வன்_தொடர்_மூடு(துறை)
            "வன்_தொடர்_மூடு" | "vaZ_qotar_mUtu" | "_serialClose" => {
                Self::expect_args(name, &args, 1)?;
                let port = Self::whole(&args[0], "துறை (port)")?;
                Ok(Self::board_result(super::board::serial_close(port).map(|_| Value::Null)))
            }
            // போலி_முனை_அமை(முனை, 0 | 1) — what an input reads, on the simulated board
            "போலி_முனை_அமை" | "pOli_muZY_amY" | "_simSetPin" => {
                Self::expect_args(name, &args, 2)?;
                let pin = Self::whole(&args[0], "முனை (pin)")?;
                let high = Self::whole(&args[1], "மட்டம் (level)")? != 0;
                Ok(Self::board_result(super::board::sim_set_pin(pin, high).map(|_| Value::Null)))
            }
            // போலி_ஒப்புமை_அமை(முனை, அளவீடு)
            "போலி_ஒப்புமை_அமை" | "pOli_oppumY_amY" | "_simSetAnalog" => {
                Self::expect_args(name, &args, 2)?;
                let pin = Self::whole(&args[0], "முனை (pin)")?;
                let value = Self::whole(&args[1], "அளவீடு (reading)")?;
                Ok(Self::board_result(super::board::sim_set_analog(pin, value).map(|_| Value::Null)))
            }
            // போலி_நேரம்_நகர்(மில்லி) — move the simulated clock
            "போலி_நேரம்_நகர்" | "pOli_nEram_nakar" | "_simAdvanceMs" => {
                Self::expect_args(name, &args, 1)?;
                let ms = Self::whole(&args[0], "மில்லி (milliseconds)")?.max(0) as u64;
                Ok(Self::board_result(super::board::sim_advance_ms(ms).map(|_| Value::Null)))
            }
            // போலி_வரி_ஊட்டு(சாதனம், வரி) — a line arrives on a simulated port
            "போலி_வரி_ஊட்டு" | "pOli_vari_Uttu" | "_simFeedSerial" => {
                Self::expect_args(name, &args, 2)?;
                let fed = super::board::sim_feed(&args[0].to_string(), &args[1].to_string());
                Ok(Self::board_result(fed.map(|_| Value::Null)))
            }
            // போலி_வரி_வெளியீடு(சாதனம்) — the lines written to a simulated port since last asked
            "போலி_வரி_வெளியீடு" | "pOli_vari_veLiyItu" | "_simSerialOutput" => {
                Self::expect_args(name, &args, 1)?;
                let lines = super::board::sim_output(&args[0].to_string())?;
                Ok(Value::Array(lines.into_iter().map(Value::String).collect()))
            }
            // நீளம் — length of an array, record or string
            "நீளம்" | "nILam" | "_length" => {
                Self::expect_args(name, &args, 1)?;
                let n = match &args[0] {
                    Value::Array(items) => items.len(),
                    Value::Map(fields) => fields.len(),
                    Value::String(s) => letters(s).len(),
                    other => {
                        return Err(format!(
                            "நீளம் ஒரு அணி/பொருள்/சொல் தேவை  (length needs an array, record or string, got {})",
                            Self::type_name(other)
                        ));
                    }
                };
                Ok(Value::Number(Decimal::from(n)))
            }
            // இணை — append to an array, returning the extended array.
            // (சேர் / cEr is already the SQL JOIN keyword.)
            // புலம்_உள்ளதா — does this record carry this field?
            //
            // A primitive because the eTamil version could not be one. poruL.qmz
            // implemented it by walking the field names, which is the only thing
            // available from inside the language, and that made a record unusable
            // as a map: building a term-frequency map over a corpus of 2383 chunks
            // never finished. Measured on the eTamil version, n lookups over a
            // record of n keys took 0.12s at n=100 and 61.6s at n=800.
            //
            // Here it is one hash lookup. The reason it has to exist at all is
            // that reading an absent field is a hard error, so every read of a
            // maybe-absent key must be guarded by this call.
            "புலம்_உள்ளதா" | "pulam_uLLaqA" | "_hasField" => {
                Self::expect_args(name, &args, 2)?;
                let fields = match &args[0] {
                    Value::Map(record) => &record.fields,
                    other => {
                        return Err(format!(
                            "புலம்_உள்ளதா ஒரு பொருள் தேவை  (hasField needs a record, got {})",
                            Self::type_name(other)
                        ));
                    }
                };
                Ok(Value::Boolean(fields.contains_key(&args[1].to_string())))
            }

            // புலம்_அல்லது — the field, or a fallback when it is not there
            //
            // The pair to புலம்_உள்ளதா, and the reason that one is rarely needed
            // directly: query parameters, headers and a decoded JSON body are all
            // records whose contents depend on what someone sent, and the caller
            // almost always has a sensible default rather than a use for a
            // boolean.
            //
            // Written as one builtin rather than left to the caller so that the
            // lookup happens once. In eTamil it was necessarily two walks of the
            // record — one to ask, one to read.
            "புலம்_அல்லது" | "pulam_allaqu" | "_fieldOr" => {
                Self::expect_args(name, &args, 3)?;
                let fields = match &args[0] {
                    Value::Map(record) => &record.fields,
                    other => {
                        return Err(format!(
                            "புலம்_அல்லது ஒரு பொருள் தேவை  (fieldOr needs a record, got {})",
                            Self::type_name(other)
                        ));
                    }
                };
                Ok(match fields.get(&args[1].to_string()) {
                    Some(found) => found.clone(),
                    None => args[2].clone(),
                })
            }

            // ஜேசான்_படி — JSON text into eTamil values
            //
            // A host builtin because the eTamil version could not keep up. It
            // is in nUlakam/vativam/jEcAZ.qmz, it is correct, and it is a recursive
            // descent parser written in the language — which means it pays the
            // language's string and record costs on every character. A 624 KB
            // corpus did not finish parsing in ten minutes.
            //
            // Returns சரி or தவறு, the same shape the eTamil version returned,
            // and refuses trailing text for the same reason it did: accepting
            // it would quietly read half a request body as though it were the
            // whole one.
            "ஜேசான்_படி" | "jEcAZ_pati" | "_jsonParse" => {
                Self::expect_args(name, &args, 1)?;
                let source = args[0].to_string();
                Ok(match serde_json::from_str::<serde_json::Value>(&source) {
                    Ok(parsed) => match Self::json_to_value(&parsed) {
                        Ok(value) => Value::Ok(Box::new(value)),
                        Err(message) => Value::Err(Box::new(Value::String(message))),
                    },
                    Err(problem) => Value::Err(Box::new(Value::String(format!(
                        "செல்லாத ஜேசான்: {}  (invalid JSON)",
                        problem
                    )))),
                })
            }

            // ஜேசான்_ஆக்கு — an eTamil value as JSON text
            //
            // The pair to ஜேசான்_படி, and replacing an eTamil version for the
            // same reason: it built its result with & one piece at a time,
            // which copies the whole string on every append.
            //
            // Returns a plain string, not a சரி, because the eTamil version
            // did and every caller in nUlakam expects that.
            "ஜேசான்_ஆக்கு" | "jEcAZ_Akku" | "_jsonStringify" => {
                Self::expect_args(name, &args, 1)?;
                Ok(Value::String(Self::value_to_json(&args[0])))
            }

            // வர்க்கமூலம் — the square root
            //
            // Was Newton's method in kaNiqam.qmz, written there because the
            // host had no square root and the arithmetic to build one was all
            // the language offered. Forty iterations per call, and every
            // vector magnitude in nuNNaRivu paid them.
            //
            // A negative input is a தவறு rather than nil or zero, exactly as
            // the eTamil version had it: the root of a negative variance is a
            // sign error upstream, and answering zero hides it inside a
            // confidence figure that looks perfectly reasonable.
            "வர்க்கமூலம்" | "varkkamUlam" | "_sqrt" => {
                Self::expect_args(name, &args, 1)?;
                let value = args[0].to_number();
                Ok(match value.sqrt() {
                    Some(root) => Value::Ok(Box::new(Value::Number(root))),
                    None => Value::Err(Box::new(Value::String(
                        "எதிர்மறை எண்ணின் வர்க்கமூலம் இல்லை  \
                         (no square root of a negative number)"
                            .to_string(),
                    ))),
                })
            }

            // இயற்கை_மடக்கை — the natural logarithm, base e
            //
            // Was an artanh series in nUlakam/nuNNaRivu/matakkY.qmz, which
            // existed only because relevance scoring needs a logarithm and the
            // host had none. Computed on Decimal here, not f64: the series it
            // replaces was accurate to about 28 digits, and routing it through
            // a binary float would have been a regression dressed as a
            // speed-up.
            //
            // Zero and below are a தவறு. The logarithm of zero is not a large
            // negative number, it is undefined, and a scoring function that
            // quietly accepted it produces a ranking nobody can account for.
            "இயற்கை_மடக்கை" | "iyaRkY_matakkY" | "_ln" => {
                Self::expect_args(name, &args, 1)?;
                let value = args[0].to_number();
                Ok(match value.checked_ln() {
                    Some(result) => Value::Ok(Box::new(Value::Number(result))),
                    None => Value::Err(Box::new(Value::String(
                        "பூஜ்ஜியம் அல்லது எதிர்மறை எண்ணின் மடக்கை இல்லை  \
                         (no logarithm of zero or a negative number)"
                            .to_string(),
                    ))),
                })
            }

            // இயற்கை_அடுக்கு — e raised to a power, the inverse of the above
            "இயற்கை_அடுக்கு" | "iyaRkY_atukku" | "_exp" => {
                Self::expect_args(name, &args, 1)?;
                let value = args[0].to_number();
                Ok(match value.checked_exp() {
                    Some(result) => Value::Ok(Box::new(Value::Number(result))),
                    None => Value::Err(Box::new(Value::String(
                        "அடுக்கு ஒரு தசமத்தில் அடங்கவில்லை  \
                         (the result does not fit a decimal)"
                            .to_string(),
                    ))),
                })
            }

            // அடுக்கேற்று(அடிப்படை, அடுக்கு) — one number raised to another
            //
            // Named அடுக்கேற்று because அடுக்கு alone is a reserved word (it is
            // the stack keyword) and cannot be a function name.
            "அடுக்கேற்று" | "atukkERRu" | "_pow" => {
                Self::expect_args(name, &args, 2)?;
                let base = args[0].to_number();
                let exponent = args[1].to_number();
                Ok(match base.checked_powd(exponent) {
                    Some(result) => Value::Ok(Box::new(Value::Number(result))),
                    None => Value::Err(Box::new(Value::String(
                        "அடுக்கேற்றம் ஒரு தசமத்தில் அடங்கவில்லை  \
                         (the result does not fit a decimal)"
                            .to_string(),
                    ))),
                })
            }

            // பத்தின்_மடக்கை — base 10
            //
            // Its own builtin rather than ln(x)/ln(10), because that division
            // is not exact: computed that way log10(1000) came back as
            // 2.9999999999999999999999999998. A base-10 logarithm is mostly
            // asked for by somebody reading the answer, and an order of
            // magnitude that reads as 2.999… is a bug report.
            "பத்தின்_மடக்கை" | "paqqiZ_matakkY" | "_log10" => {
                Self::expect_args(name, &args, 1)?;
                let value = args[0].to_number();
                Ok(match value.checked_log10() {
                    Some(result) => Value::Ok(Box::new(Value::Number(result))),
                    None => Value::Err(Box::new(Value::String(
                        "பூஜ்ஜியம் அல்லது எதிர்மறை எண்ணின் மடக்கை இல்லை  \
                         (no logarithm of zero or a negative number)"
                            .to_string(),
                    ))),
                })
            }

            // வரிசையாக்கு(அணி) — an array in order, smallest first
            //
            // Here because every library that needed ordering wrote its own.
            // qaravaricY.qmz used an insertion sort, which is the right shape
            // for ranking tens of results and the wrong one for anything
            // larger, and it is not the only place.
            //
            // Numbers sort numerically and everything else by its text, so a
            // mixed array still has a defined order rather than an error: the
            // caller asked for an ordering, and refusing one because a null
            // crept into the data is rarely what is wanted. Sorting is stable,
            // so equal elements keep the order they arrived in.
            "வரிசையாக்கு" | "varicYyAkku" | "_sort" => {
                Self::expect_args(name, &args, 1)?;
                let items = match &args[0] {
                    Value::Array(items) => items.clone(),
                    other => {
                        return Err(format!(
                            "வரிசையாக்கு ஒரு அணி தேவை  (sort needs an array, got {})",
                            Self::type_name(other)
                        ));
                    }
                };
                let all_numbers = items.iter().all(|v| matches!(v, Value::Number(_)));
                let mut sorted = items;
                if all_numbers {
                    sorted.sort_by(|a, b| {
                        a.to_number()
                            .partial_cmp(&b.to_number())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                } else {
                    sorted.sort_by_key(|value| value.to_string());
                }
                Ok(Value::Array(sorted))
            }

            // புலத்தால்_வரிசையாக்கு(அணி, புலம், இறங்குமா) — records by one field
            //
            // The form that ranking actually wants: a list of records ordered
            // by a score they carry. Written as a builtin rather than left to
            // a comparator because eTamil has no way to hand a function to the
            // host, and because this is the case that appears every time.
            //
            // A record without the field sorts as zero rather than failing.
            // One malformed row out of a retriever should cost that row its
            // place, not cost the caller the whole ranking.
            "புலத்தால்_வரிசையாக்கு" | "pulaqqAl_varicYyAkku" | "_sortByField" => {
                Self::expect_args(name, &args, 3)?;
                let items = match &args[0] {
                    Value::Array(items) => items.clone(),
                    other => {
                        return Err(format!(
                            "புலத்தால்_வரிசையாக்கு ஒரு அணி தேவை  \
                             (sortByField needs an array, got {})",
                            Self::type_name(other)
                        ));
                    }
                };
                let field = args[1].to_string();
                let descending = args[2].is_truthy();

                let key_of = |value: &Value| -> Decimal {
                    match value {
                        Value::Map(record) => match record.fields.get(&field) {
                            Some(found) => found.to_number(),
                            None => Decimal::ZERO,
                        },
                        _ => Decimal::ZERO,
                    }
                };

                let mut sorted = items;
                sorted.sort_by(|a, b| {
                    let order = key_of(a)
                        .partial_cmp(&key_of(b))
                        .unwrap_or(std::cmp::Ordering::Equal);
                    if descending {
                        order.reverse()
                    } else {
                        order
                    }
                });
                Ok(Value::Array(sorted))
            }

            "இணை" | "iNY" | "_append" => {
                Self::expect_args(name, &args, 2)?;
                match &args[0] {
                    Value::Array(items) => {
                        let mut items = items.clone();
                        items.push(args[1].clone());
                        Ok(Value::Array(items))
                    }
                    other => Err(format!(
                        "சேர் ஒரு அணி தேவை  (append needs an array, got {})",
                        Self::type_name(other)
                    )),
                }
            }
            // --- Dates ---
            // ISO-8601 text throughout, which sorts chronologically, so
            // comparison needs no primitive. Arithmetic does: the calendar
            // cannot be derived from string operations.
            // இன்று() — today, in UTC
            "இன்று" | "iZRu" | "_today" => {
                Self::expect_args(name, &args, 0)?;
                let seconds = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| "கடிகாரம் படிக்க முடியவில்லை  (cannot read the clock)")?
                    .as_secs() as i64;
                Ok(Value::String(Self::format_date(seconds / 86_400)))
            }
            // இப்போதைய_நொடி() — whole seconds since 1970-01-01 UTC
            //
            // For timing, not for dates: a retry's next attempt, a token's age.
            // Before this the only clock finer than a day was வன்_மில்லி, which
            // counts from the program's start, so anything a program stored and
            // read back after a restart — an outbox row's backoff — meant
            // nothing. A number rather than text, because the uses are
            // arithmetic; இன்று remains the way to ask for a date.
            "இப்போதைய_நொடி" | "ippOqYya_noti" | "_nowSeconds" => {
                Self::expect_args(name, &args, 0)?;
                let seconds = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| "கடிகாரம் படிக்க முடியவில்லை  (cannot read the clock)")?
                    .as_secs() as i64;
                Ok(Value::Number(Decimal::from(seconds)))
            }
            // நாள்_வேறுபாடு(a, b) — whole days from a to b, negative if b is earlier
            "நாள்_வேறுபாடு" | "nAL_vERupAtu" | "_daysBetween" => {
                Self::expect_args(name, &args, 2)?;
                let from = Self::parse_date(&args[0])?;
                let to = Self::parse_date(&args[1])?;
                Ok(Value::Number(Decimal::from(to - from)))
            }
            // நாள்_கூட்டு(நாள், நாட்கள்)
            "நாள்_கூட்டு" | "nAL_kUttu" | "_addDays" => {
                Self::expect_args(name, &args, 2)?;
                let date = Self::parse_date(&args[0])?;
                let days = rust_decimal::prelude::ToPrimitive::to_i64(&args[1].to_number())
                    .ok_or("நாட்கள் ஒரு முழு எண்ணாக இருக்க வேண்டும்  (days must be a whole number)")?;
                Ok(Value::String(Self::format_date(date + days)))
            }
            // வகை — the type of a value, as a string
            "வகை" | "vakY" | "_typeof" => {
                Self::expect_args(name, &args, 1)?;
                // A shaped record is the shape it was made as.
                if let Value::Map(record) = &args[0]
                    && let Some(shape) = &record.shape
                {
                    return Ok(Value::String(shape.clone()));
                }
                Ok(Value::String(Self::type_name(&args[0]).to_string()))
            }
            // --- Results, following Rust ---
            // சரி(v) — Ok
            "சரி" | "cari" | "_ok" => {
                Self::expect_args(name, &args, 1)?;
                Ok(Value::Ok(Box::new(args[0].clone())))
            }
            // தவறு(e) — Err
            "தவறு" | "qavaRu" | "_err" => {
                Self::expect_args(name, &args, 1)?;
                Ok(Value::Err(Box::new(args[0].clone())))
            }
            // சரியா(r) — is_ok
            "சரியா" | "cariyA" | "_isOk" => {
                Self::expect_args(name, &args, 1)?;
                Ok(Value::Boolean(matches!(args[0], Value::Ok(_))))
            }
            // தவறா(r) — is_err
            "தவறா" | "qavaRA" | "_isErr" => {
                Self::expect_args(name, &args, 1)?;
                Ok(Value::Boolean(matches!(args[0], Value::Err(_))))
            }
            // மதிப்பு(r) — unwrap; a தவறு here is a runtime error, as in Rust
            "மதிப்பு" | "maqippu" | "_unwrap" => {
                Self::expect_args(name, &args, 1)?;
                match &args[0] {
                    Value::Ok(inner) => Ok((**inner).clone()),
                    Value::Err(error) => Err(format!(
                        "தவறான முடிவை விரித்தது: {}  (unwrap on an error: {})",
                        error,
                        error
                    )),
                    other => Err(format!(
                        "மதிப்பு க்கு ஒரு முடிவு தேவை  (unwrap needs a result, got {})",
                        Self::type_name(other)
                    )),
                }
            }
            // தவறு_மதிப்பு(r) — what the failure carried
            //
            // மதிப்பு opens a சரி and nothing opened the other one, so a
            // program could tell that something failed and never read why. It
            // could construct a தவறு holding a record and then not get the
            // record back. That is half a result type: handling a failure
            // usually means looking at it — deciding whether this is the one
            // error worth retrying, or the ninety-nine that are not.
            //
            // A சரி here is a runtime error, exactly as a தவறு is to மதிப்பு.
            // Answering nil instead would make "it succeeded" and "it failed
            // with nothing in it" the same answer.
            "தவறு_மதிப்பு" | "qavaRu_maqippu" | "_unwrapErr" => {
                Self::expect_args(name, &args, 1)?;
                match &args[0] {
                    Value::Err(error) => Ok((**error).clone()),
                    Value::Ok(inner) => Err(format!(
                        "வெற்றியான முடிவில் பிழை இல்லை: {}  \
                         (there is no error in a successful result: {})",
                        inner,
                        inner
                    )),
                    other => Err(format!(
                        "தவறு_மதிப்பு க்கு ஒரு முடிவு தேவை  (it needs a result, got {})",
                        Self::type_name(other)
                    )),
                }
            }
            // இயல்பு(r, d) — unwrap_or
            "இயல்பு" | "iyalpu" | "_unwrapOr" => {
                Self::expect_args(name, &args, 2)?;
                match &args[0] {
                    Value::Ok(inner) => Ok((**inner).clone()),
                    Value::Err(_) => Ok(args[1].clone()),
                    other => Err(format!(
                        "இயல்பு க்கு ஒரு முடிவு தேவை  (unwrapOr needs a result, got {})",
                        Self::type_name(other)
                    )),
                }
            }
            // --- Numeric primitives ---
            // These need the decimal type itself, so they cannot be written
            // in eTamil. Everything derivable from them lives in nUlakam/.
            // வட்டமிடு(n, இடங்கள்) — round to n decimal places, half away
            // from zero, which is what Indian tax rules expect.
            "வட்டமிடு" | "vattamitu" | "_round" => {
                Self::expect_args(name, &args, 2)?;
                let places = rust_decimal::prelude::ToPrimitive::to_u32(&args[1].to_number())
                    .ok_or("வட்டமிடு: இடங்கள் ஒரு முழு எண்  (round: places must be a whole number)")?;
                Ok(Value::Number(args[0].to_number().round_dp_with_strategy(
                    places,
                    rust_decimal::RoundingStrategy::MidpointAwayFromZero,
                )))
            }
            // தரை(n) — floor
            "தரை" | "qarY" | "_floor" => {
                Self::expect_args(name, &args, 1)?;
                Ok(Value::Number(args[0].to_number().floor()))
            }
            // மேல்(n) — ceiling
            "மேல்" | "mEl" | "_ceil" => {
                Self::expect_args(name, &args, 1)?;
                Ok(Value::Number(args[0].to_number().ceil()))
            }
            // சொல்லாக்கு(v) — render any value as text
            "சொல்லாக்கு" | "collAkku" | "_toString" => {
                Self::expect_args(name, &args, 1)?;
                Ok(Value::String(args[0].to_string()))
            }
            // எண்ணாக்கு(s) — parse text as a number, as a result
            "எண்ணாக்கு" | "eNNAkku" | "_toNumber" => {
                Self::expect_args(name, &args, 1)?;
                let text = args[0].to_string();
                match Decimal::from_str(text.trim()) {
                    Ok(number) => Ok(Value::Ok(Box::new(Value::Number(number)))),
                    Err(_) => Ok(Value::Err(Box::new(Value::String(format!(
                        "'{}' ஒரு எண் அல்ல  ('{}' is not a number)",
                        text, text
                    ))))),
                }
            }
            // Case folding matters for PAN/GSTIN style codes; Tamil has no
            // case, so these only affect the Latin characters.
            "மேல்_எழுத்து" | "mEl_ezuqqu" | "_upper" => {
                Self::expect_args(name, &args, 1)?;
                Ok(Value::String(args[0].to_string().to_uppercase()))
            }
            "கீழ்_எழுத்து" | "kIz_ezuqqu" | "_lower" => {
                Self::expect_args(name, &args, 1)?;
                Ok(Value::String(args[0].to_string().to_lowercase()))
            }
            // --- Text, over a whole string ---------------------------------
            // மாற்று, பிரி and ஒன்றிணை were nUlakam/atippatY/col.qmz functions until
            // now. They read one letter at a time, and every read re-segmented
            // the entire string, so a single search cost O(n²) segmentations:
            // measured at 14 seconds over 8 KB, and a 400 KB document never
            // finished. The letter-boundary rule in cluster_matches is the
            // same rule those versions enforced by comparing letter by letter,
            // so what a program computes does not change — only what it costs.

            // மாற்று(சரம், பழையது, புதியது) — every occurrence replaced
            "மாற்று" | "mARRu" | "_replace" => {
                Self::expect_args(name, &args, 3)?;
                let text = args[0].to_string();
                let from = args[1].to_string();
                let to = args[2].to_string();
                // Nothing to look for means nothing to change, rather than a
                // copy of `to` wedged between every letter.
                if from.is_empty() {
                    return Ok(Value::String(text));
                }
                let mut out = String::with_capacity(text.len());
                let mut cursor = 0;
                for at in cluster_matches(&text, &from) {
                    out.push_str(&text[cursor..at]);
                    out.push_str(&to);
                    cursor = at + from.len();
                }
                out.push_str(&text[cursor..]);
                Ok(Value::String(out))
            }
            // பிரி(சரம், பிரிப்பான்) — split into an array of pieces
            "பிரி" | "piri" | "_split" => {
                Self::expect_args(name, &args, 2)?;
                let text = args[0].to_string();
                let separator = args[1].to_string();
                // An empty separator has no pieces to find, so the whole
                // string is the single piece — what col.qmz answered too.
                if separator.is_empty() {
                    return Ok(Value::Array(vec![Value::String(text)]));
                }
                let mut pieces = Vec::new();
                let mut cursor = 0;
                for at in cluster_matches(&text, &separator) {
                    pieces.push(Value::String(text[cursor..at].to_string()));
                    cursor = at + separator.len();
                }
                // The tail always closes the list, so "அ," splits into two
                // pieces and the second one is empty.
                pieces.push(Value::String(text[cursor..].to_string()));
                Ok(Value::Array(pieces))
            }
            // ஒன்றிணை(பட்டியல், இணைப்பான்) — join an array into one string
            "ஒன்றிணை" | "oZRiNY" | "_join" => {
                Self::expect_args(name, &args, 2)?;
                let separator = args[1].to_string();
                match &args[0] {
                    Value::Array(items) => Ok(Value::String(
                        items
                            .iter()
                            .map(|item| item.to_string())
                            .collect::<Vec<_>>()
                            .join(&separator),
                    )),
                    other => Err(format!(
                        "ஒன்றிணை ஒரு அணி தேவை  (join needs an array, got {})",
                        Self::type_name(other)
                    )),
                }
            }

            // --- Files ------------------------------------------------------
            // கோப்பு_சேமி(கோப்பு, உள்ளடக்கம்) — write the whole file, exactly
            //
            // கோப்பு_எழுது appends a line, which is right for a CSV row and
            // wrong for a document: it cannot produce a file whose bytes are
            // exactly the string that was built, and it adds a newline that
            // some formats count. This writes the string and nothing else,
            // and answers with the number of bytes written.
            "கோப்பு_சேமி" | "kOppu_cEmi" | "_fileSave" => {
                Self::expect_args(name, &args, 2)?;
                let filename = args[0].to_string();
                let content = args[1].to_string();
                match host::write(&filename, content.as_bytes()) {
                    Ok(()) => Ok(Value::Ok(Box::new(Value::Number(Decimal::from(
                        content.len(),
                    ))))),
                    Err(e) => Ok(Value::Err(Box::new(Value::String(format!(
                        "கோப்பு '{}' எழுத முடியவில்லை  (cannot write '{}'): {}",
                        filename, filename, e
                    ))))),
                }
            }

            // --- The file system ---------------------------------------------
            //
            // Reading and writing a *named* file has always been possible;
            // finding out what files there are had not. Without it a program
            // that works over a tree — collecting a corpus, walking a folder
            // of statements to import — had to shell out to `find` through
            // கட்டளை_ஓட்டு, which is not portable: `find` on Windows is a
            // text-search tool, so the same program did something else
            // entirely there.
            //
            // Three primitives, because three are what the language cannot
            // express for itself. Everything above them — joining paths,
            // taking a base name or an extension, walking a tree — is
            // ordinary eTamil and lives in nUlakam/kOppumuRY/kOppumuRY.qmz.

            // கோப்பகம்_படி(பாதை) — the entries of a directory, sorted, as bare
            // names rather than paths. Joining is the caller's business:
            // deciding the separator here would put a backslash in the middle
            // of a path an author wrote with slashes.
            "கோப்பகம்_படி" | "kOppakam_pati" | "_readDir" => {
                Self::expect_args(name, &args, 1)?;
                let path = args[0].to_string();
                match host::read_dir(&path) {
                    Ok(names) => Ok(Value::Ok(Box::new(Value::Array(
                        names.into_iter().map(Value::String).collect(),
                    )))),
                    Err(e) => Ok(Value::Err(Box::new(Value::String(format!(
                        "கோப்பகம் '{}' படிக்க முடியவில்லை  (cannot read directory '{}'): {}",
                        path, path, e
                    ))))),
                }
            }

            // கோப்பு_உள்ளதா(பாதை) — a plain ஈர்மம், not a முடிவு. "No" is an
            // answer rather than a failure, and there is nothing a caller
            // could do differently for a path that is absent versus one it may
            // not look at.
            "கோப்பு_உள்ளதா" | "kOppu_uLLaqA" | "_fileExists" => {
                Self::expect_args(name, &args, 1)?;
                Ok(Value::Boolean(host::exists(&args[0].to_string())))
            }

            // கோப்பு_விவரம்(பாதை) — what a path is:
            //   {வகை: "கோப்பகம்" | "கோப்பு", அளவு: எண், மாற்றம்: எண் | இன்மை}
            //
            // மாற்றம் is seconds since 1970 because that is the form
            // arithmetic works on; நாளாக() in nUlakam turns it into a date. A
            // file system that cannot report one yields இன்மை rather than 0,
            // since 1970 is a real date and a program comparing against it
            // would quietly believe the file was ancient.
            "கோப்பு_விவரம்" | "kOppu_vivaram" | "_fileInfo" => {
                Self::expect_args(name, &args, 1)?;
                let path = args[0].to_string();
                match host::metadata(&path) {
                    Ok((is_directory, size, modified)) => {
                        let mut fields = HashMap::new();
                        fields.insert(
                            "வகை".to_string(),
                            Value::String(
                                if is_directory { "கோப்பகம்" } else { "கோப்பு" }.to_string(),
                            ),
                        );
                        fields.insert("அளவு".to_string(), Value::Number(Decimal::from(size)));
                        fields.insert(
                            "மாற்றம்".to_string(),
                            match modified {
                                Some(seconds) => Value::Number(Decimal::from(seconds)),
                                None => Value::Null,
                            },
                        );
                        Ok(Value::Ok(Box::new(Value::Map(fields.into()))))
                    }
                    Err(e) => Ok(Value::Err(Box::new(Value::String(format!(
                        "கோப்பு '{}' பற்றி அறிய முடியவில்லை  (cannot stat '{}'): {}",
                        path, path, e
                    ))))),
                }
            }

            // --- ODF packages -----------------------------------------------
            // An .odt or .ods is a zip: content.xml and styles.xml hold the
            // text, and beside them sit pictures, thumbnails, and a mimetype
            // entry that the format requires to come first and uncompressed.
            //
            // So a template is not read and rewritten wholesale. It is copied
            // entry by entry with named entries swapped out, which leaves every
            // picture byte-for-byte intact and keeps the container legal. Bytes
            // that are not text never enter the language, where they could not
            // survive being a சரம் anyway.

            // பொதி_படி(கோப்பு, உறுப்பு) — one entry of a package, as text
            "பொதி_படி" | "poqi_pati" | "_packageRead" => {
                Self::expect_args(name, &args, 2)?;
                let path = args[0].to_string();
                let entry = args[1].to_string();
                match Self::package_entry(&path, &entry) {
                    Ok(text) => Ok(Value::Ok(Box::new(Value::String(text)))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // பொதி_மாற்று(மூலம், விளைவு, மாற்றங்கள்) — copy a package with the
            // named entries replaced. மாற்றங்கள் is a record keyed by entry
            // name, so {"content.xml": ஆவணம்} rewrites just that one.
            "பொதி_மாற்று" | "poqi_mARRu" | "_packageWrite" => {
                Self::expect_args(name, &args, 3)?;
                let source = args[0].to_string();
                let target = args[1].to_string();
                let changes = match &args[2] {
                    Value::Map(fields) => fields
                        .iter()
                        .map(|(key, value)| (key.clone(), value.to_string()))
                        .collect::<HashMap<String, String>>(),
                    other => {
                        return Err(format!(
                            "பொதி_மாற்று ஒரு பொருள் தேவை  (package changes must be a record, got {})",
                            Self::type_name(other)
                        ));
                    }
                };
                match Self::package_copy(&source, &target, &changes) {
                    Ok(count) => Ok(Value::Ok(Box::new(Value::Number(Decimal::from(count))))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }

            // --- Running another program ------------------------------------
            // A PDF comes out of LibreOffice, not out of this language, so
            // something has to be able to start it. That is a sharp tool, so
            // it is blunted in three ways: nothing runs unless it is named in
            // ETAMIL_EXEC_ALLOW, arguments are passed as a list and never
            // through a shell, and a program that will not finish is killed.

            // கட்டளை_ஓட்டு(நிரல், அளபுருக்கள், வினாடிகள்) — run a program and
            // wait. Answers with {நிலை, வெளியீடு, பிழை}: a non-zero exit is a
            // value to test, not a failure, because a converter that refuses
            // one document is ordinary. Only being unable to run it is a தவறு.
            "கட்டளை_ஓட்டு" | "kattaLY_Ottu" | "_run" => {
                Self::expect_args(name, &args, 3)?;
                let program = args[0].to_string();
                let parameters = match &args[1] {
                    Value::Array(items) => items.iter().map(|i| i.to_string()).collect::<Vec<_>>(),
                    other => {
                        return Err(format!(
                            "கட்டளை_ஓட்டு அளபுருக்கள் ஒரு அணி தேவை  (arguments must be an array, got {})",
                            Self::type_name(other)
                        ));
                    }
                };
                let seconds = rust_decimal::prelude::ToPrimitive::to_u64(&args[2].to_number())
                    .unwrap_or(0);
                match Self::run_program(&program, &parameters, seconds) {
                    Ok((code, out, err)) => {
                        let mut answer = HashMap::new();
                        answer.insert("நிலை".to_string(), Value::Number(Decimal::from(code)));
                        answer.insert("வெளியீடு".to_string(), Value::String(out));
                        answer.insert("பிழை".to_string(), Value::String(err));
                        Ok(Value::Ok(Box::new(Value::Map(answer.into()))))
                    }
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }

            // --- Answering with a file --------------------------------------
            // பதில்_கோப்பு(நிலை, கோப்பு, தலைப்புகள்) — send a file as the body
            //
            // The same shape as பதில், and for the same reason as பொதி_மாற்று:
            // a PDF or an .odt is not a சரம், so the language names the file
            // and the server reads it. Nothing else can send one — a body
            // built as text would lose every byte that is not valid UTF-8.
            //
            // Content-Type defaults to application/octet-stream. Set it in
            // தலைப்புகள் when the browser should know better than that.
            "பதில்_கோப்பு" | "paDil_kOppu" | "_respondFile" => {
                Self::expect_args(name, &args, 3)?;
                let status = args[0].to_number();
                let path = args[1].to_string();
                if !std::path::Path::new(&path).is_file() {
                    return Ok(Value::Err(Box::new(Value::String(format!(
                        "கோப்பு '{}' இல்லை  (no such file '{}')",
                        path, path
                    )))));
                }
                self.variables
                    .insert("response_status".to_string(), Value::Number(status));
                self.variables
                    .insert("response_file".to_string(), Value::String(path));
                self.variables
                    .insert("response_headers".to_string(), args[2].clone());
                Ok(Value::Ok(Box::new(Value::Null)))
            }

            // --- Uploads ----------------------------------------------------
            // பதிவேற்றம்_சேமி(குறியீடு, கோப்பு) — write an uploaded file out
            //
            // request_files says what arrived and in what order; this writes
            // one of them where the handler wants it. The bytes never become
            // a value, so an upload cannot be corrupted by being looked at,
            // and nothing is spooled to a temporary file that someone then
            // has to remember to delete.
            "பதிவேற்றம்_சேமி" | "paqivERRam_cEmi" | "_saveUpload" => {
                Self::expect_args(name, &args, 2)?;
                let wanted = rust_decimal::prelude::ToPrimitive::to_usize(&args[0].to_number());
                let path = args[1].to_string();
                let found = wanted.and_then(|index| self.uploads.get(index));
                match found {
                    Some(upload) => match host::write(&path, &upload.data) {
                        Ok(()) => Ok(Value::Ok(Box::new(Value::Number(Decimal::from(
                            upload.data.len(),
                        ))))),
                        Err(e) => Ok(Value::Err(Box::new(Value::String(format!(
                            "கோப்பு '{}' எழுத முடியவில்லை  (cannot write '{}'): {}",
                            path, path, e
                        ))))),
                    },
                    None => Ok(Value::Err(Box::new(Value::String(format!(
                        "பதிவேற்றம் {} இல்லை  (no upload at {})",
                        args[0].to_number(),
                        args[0].to_number()
                    ))))),
                }
            }

            // --- Single sign-on ---------------------------------------------
            // An identity provider signs with RS256 and publishes its public
            // keys as a JWKS document. Fetching that document, picking the key
            // and caching it are ordinary work the language can do with
            // வலை_பெறு and nUlakam/vativam/jEcAZ.qmz. Only the two things it cannot
            // do live here: reading a token's header, and checking a signature
            // against an RSA key.

            // சீட்டு_தலைப்பு(சீட்டு) — {kid, alg}, read but not trusted
            "சீட்டு_தலைப்பு" | "cIttu_qalYppu" | "_tokenHeader" => {
                Self::expect_args(name, &args, 1)?;
                match crate::http::auth::token_header(&args[0].to_string()) {
                    Ok((kid, algorithm)) => {
                        let mut described = HashMap::new();
                        described.insert("kid".to_string(), Value::String(kid));
                        described.insert("alg".to_string(), Value::String(algorithm));
                        Ok(Value::Ok(Box::new(Value::Map(described.into()))))
                    }
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // சீட்டு_பொதுச்_சரிபார்(சீட்டு, n, e, வழங்குநர், பார்வையாளர்)
            //   — verify against a public key from a JWKS
            //
            // The issuer and audience are arguments and not optional: a token
            // an identity provider really signed, for somebody else's
            // application, is a real token and must still be refused.
            "சீட்டு_பொதுச்_சரிபார்" | "cIttu_poquc_caripAr" | "_verifyTokenRSA" => {
                Self::expect_args(name, &args, 5)?;
                match crate::http::auth::verify_rsa_token(
                    &args[0].to_string(),
                    &args[1].to_string(),
                    &args[2].to_string(),
                    &args[3].to_string(),
                    &args[4].to_string(),
                ) {
                    Ok(claims) => Ok(Value::Ok(Box::new(Value::String(claims)))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }

            // --- Stopping ---------------------------------------------------
            // வெளியேறு(நிலை) — stop the program with an exit status
            //
            // A test run that reports failures and then exits 0 has told the
            // truth to a reader and a lie to everything else — CI, make, a
            // shell script. This is how nUlakam/cOqaZY/cOqaZY.qmz makes a failing
            // suite fail the process it is running in.
            //
            // Nothing is returned, because nothing continues.
            "வெளியேறு" | "veLiyERu" | "_exit" => {
                Self::expect_args(name, &args, 1)?;
                let status = rust_decimal::prelude::ToPrimitive::to_i32(&args[0].to_number())
                    .unwrap_or(1);
                // Exiting does not unwind, so anything still buffered would be
                // lost — including the summary line that explains the status.
                host::exit(status)?;
                // Native `exit` never returns; the browser host has no process
                // to end, so a zero status falls through and carries on.
                Ok(Value::Ok(Box::new(Value::Number(Decimal::from(status)))))
            }

            // --- Signing with a key only one side holds ---------------------
            // கையொப்பம் is HMAC: it proves a message came from someone holding
            // the same secret you do. Both sides can forge each other's
            // messages, so neither can prove to a third party which of them
            // sent one. That is enough for a webhook and not enough for a
            // ledger entry or a payment instruction.
            //
            // These are ECDSA over P-256, signed with a private key and checked
            // with a public one — what Hyperledger Fabric requires of an MSP
            // identity, and what a bank requires of a request that moves money.
            //
            // Keys and signatures cross as lowercase hex, as HMAC's already do.

            // வளைவு_சாவிகள்() — a new key pair, {தனி, பொது}
            "வளைவு_சாவிகள்" | "vaLYvu_cAvikaL" | "_keyPair" => {
                Self::expect_args(name, &args, 0)?;
                let (private, public) = crate::signing::generate();
                let mut pair = HashMap::new();
                pair.insert("தனி".to_string(), Value::String(private));
                pair.insert("பொது".to_string(), Value::String(public));
                Ok(Value::Map(pair.into()))
            }
            // வளைவு_பொதுச்சாவி(தனிச்சாவி) — the public half of a private key
            "வளைவு_பொதுச்சாவி" | "vaLYvu_poquccAvi" | "_publicKey" => {
                Self::expect_args(name, &args, 1)?;
                match crate::signing::public_of(&args[0].to_string()) {
                    Ok(public) => Ok(Value::Ok(Box::new(Value::String(public)))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // வளைவு_கையொப்பம்(செய்தி, தனிச்சாவி) — sign, as DER hex
            "வளைவு_கையொப்பம்" | "vaLYvu_kYyoppam" | "_ecSign" => {
                Self::expect_args(name, &args, 2)?;
                match crate::signing::sign(&args[0].to_string(), &args[1].to_string()) {
                    Ok(signature) => Ok(Value::Ok(Box::new(Value::String(signature)))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // வளைவு_சரிபார்(செய்தி, கையொப்பம், பொதுச்சாவி) — does it hold?
            //
            // A signature that simply does not verify answers false: that is an
            // ordinary outcome a program has to handle, not a fault. A key that
            // is not a key at all is a fault, and says so.
            "வளைவு_சரிபார்" | "vaLYvu_caripAr" | "_ecVerify" => {
                Self::expect_args(name, &args, 3)?;
                match crate::signing::verify(
                    &args[0].to_string(),
                    &args[1].to_string(),
                    &args[2].to_string(),
                ) {
                    Ok(held) => Ok(Value::Ok(Box::new(Value::Boolean(held)))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }

            // --- The environment a program runs in --------------------------
            // சூழல்(பெயர், இயல்பு_மதிப்பு) — an environment variable, or a
            // fallback when it is not set.
            //
            // The host already reads several of these for itself —
            // ETAMIL_TLS_CERT, ETAMIL_EXEC_ALLOW, ETAMIL_JWT_SECRET — but a
            // program has deployment settings of its own: which gateway to
            // call, which key to send, which database to open. Those belong
            // outside the source for the same reason: a URL that differs
            // between test and production is not something to edit code for,
            // and a credential written into a program is a credential in the
            // repository.
            //
            // A fallback rather than a result, because "not set" is the
            // ordinary case for an optional setting and forcing every caller
            // to unwrap would make the common path the loud one.
            "சூழல்" | "cUzal" | "_env" => {
                Self::expect_args(name, &args, 2)?;
                let wanted = args[0].to_string();
                match std::env::var(&wanted) {
                    Ok(found) => Ok(Value::String(found)),
                    Err(_) => Ok(args[1].clone()),
                }
            }

            // --- Redis ------------------------------------------------------
            // One command, generically, because that is the shape of Redis:
            // a command name and its arguments. Every command works, including
            // ones added after this was written, and convenience for the common
            // ones belongs in nUlakam rather than in a builtin each.
            //
            // The roadmap said Redis needed a design first, because it does not
            // fit a trait shaped as execute(sql)/query(sql). It does not, and
            // this is the design: it is not a query language.

            // ரெடிஸ்_இணை(முகவரி) — connect, as host:port
            "ரெடிஸ்_இணை" | "retis_iNY" | "_redisConnect" => {
                Self::expect_args(name, &args, 1)?;
                let address = args[0].to_string();
                match crate::redis::Connection::open(&address) {
                    Ok(connection) => {
                        self.cache = Some(connection);
                        Ok(Value::Ok(Box::new(Value::String(address))))
                    }
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // ரெடிஸ்_கட்டளை(கட்டளை, அளபுருக்கள்) — send one, read the reply
            //
            // A server that answers -ERR is answering, so that comes back as a
            // தவறு carrying what it said. Being unable to reach it at all is
            // also a தவறு, and the message says which happened.
            "ரெடிஸ்_கட்டளை" | "retis_kattaLY" | "_redisCommand" => {
                Self::expect_args(name, &args, 2)?;
                let command = args[0].to_string();
                let arguments = match &args[1] {
                    Value::Array(items) => {
                        items.iter().map(|item| item.to_string()).collect::<Vec<_>>()
                    }
                    other => {
                        return Err(format!(
                            "ரெடிஸ்_கட்டளை அளபுருக்கள் ஒரு அணி தேவை  \
                             (the arguments must be an array, got {})",
                            Self::type_name(other)
                        ));
                    }
                };

                let connection = match self.cache.as_mut() {
                    Some(connection) => connection,
                    None => {
                        return Ok(Value::Err(Box::new(Value::String(
                            "ரெடிஸ் இணைக்கப்படவில்லை  (not connected to Redis): \
                             use ரெடிஸ்_இணை first"
                                .to_string(),
                        ))));
                    }
                };

                match connection.command(&command, &arguments) {
                    Ok(crate::redis::Reply::Error(said)) => {
                        Ok(Value::Err(Box::new(Value::String(said))))
                    }
                    Ok(reply) => Ok(Value::Ok(Box::new(reply.to_value()))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // ரெடிஸ்_பிரி() — done with it
            "ரெடிஸ்_பிரி" | "retis_piri" | "_redisClose" => {
                Self::expect_args(name, &args, 0)?;
                let was = self.cache.is_some();
                self.cache = None;
                Ok(Value::Boolean(was))
            }

            // --- MongoDB ----------------------------------------------------
            // A document is a பொருள் and a collection of them is an array of
            // records, so the mapping needed no invention — the value model was
            // already document-shaped. What did need care is numbers: see
            // src/mongo.rs. Nothing written from here is a double.
            //
            // Behind --features mongodb. Without it these say so rather than
            // not existing, because "no such function" sends someone hunting
            // for a typo.
            #[cfg(feature = "mongodb")]
            "மொங்கோ_இணை" | "mowkO_iNY" | "_mongoConnect" => {
                Self::expect_args(name, &args, 2)?;
                let uri = args[0].to_string();
                let database = args[1].to_string();
                match crate::mongo::Connection::open(&uri, &database) {
                    Ok(connection) => {
                        self.documents = Some(connection);
                        Ok(Value::Ok(Box::new(Value::String(database))))
                    }
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // மொங்கோ_கட்டளை(கட்டளைப்_பொருள்) — runCommand, the generic door
            #[cfg(feature = "mongodb")]
            "மொங்கோ_கட்டளை" | "mowkO_kattaLY" | "_mongoCommand" => {
                Self::expect_args(name, &args, 1)?;
                let command = match crate::mongo::to_document(&args[0]) {
                    Ok(document) => document,
                    Err(why) => return Ok(Value::Err(Box::new(Value::String(why)))),
                };
                match Self::mongo_of(&self.documents) {
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    Ok(connection) => match connection.command(command) {
                        Ok(reply) => Ok(Value::Ok(Box::new(reply))),
                        Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    },
                }
            }
            // மொங்கோ_செருகு(தொகுப்பு, ஆவணம்) — insert one, answering its id
            #[cfg(feature = "mongodb")]
            "மொங்கோ_செருகு" | "mowkO_ceruku" | "_mongoInsert" => {
                Self::expect_args(name, &args, 2)?;
                let collection = args[0].to_string();
                let document = match crate::mongo::to_document(&args[1]) {
                    Ok(document) => document,
                    Err(why) => return Ok(Value::Err(Box::new(Value::String(why)))),
                };
                match Self::mongo_of(&self.documents) {
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    Ok(connection) => match connection.insert(&collection, document) {
                        Ok(id) => Ok(Value::Ok(Box::new(id))),
                        Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    },
                }
            }
            // மொங்கோ_கண்டுபிடி(தொகுப்பு, வடிகட்டி) — every match, as an array
            #[cfg(feature = "mongodb")]
            "மொங்கோ_கண்டுபிடி" | "mowkO_kaNtupiti" | "_mongoFind" => {
                Self::expect_args(name, &args, 2)?;
                let collection = args[0].to_string();
                let filter = match crate::mongo::to_document(&args[1]) {
                    Ok(document) => document,
                    Err(why) => return Ok(Value::Err(Box::new(Value::String(why)))),
                };
                match Self::mongo_of(&self.documents) {
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    Ok(connection) => match connection.find(&collection, filter) {
                        Ok(found) => Ok(Value::Ok(Box::new(found))),
                        Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    },
                }
            }
            // மொங்கோ_புதுப்பி(தொகுப்பு, வடிகட்டி, மாற்றம், அனைத்துமா)
            #[cfg(feature = "mongodb")]
            "மொங்கோ_புதுப்பி" | "mowkO_puquppi" | "_mongoUpdate" => {
                Self::expect_args(name, &args, 4)?;
                let collection = args[0].to_string();
                let filter = match crate::mongo::to_document(&args[1]) {
                    Ok(document) => document,
                    Err(why) => return Ok(Value::Err(Box::new(Value::String(why)))),
                };
                let change = match crate::mongo::to_document(&args[2]) {
                    Ok(document) => document,
                    Err(why) => return Ok(Value::Err(Box::new(Value::String(why)))),
                };
                let many = args[3].is_truthy();
                match Self::mongo_of(&self.documents) {
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    Ok(connection) => {
                        match connection.update(&collection, filter, change, many) {
                            Ok(changed) => Ok(Value::Ok(Box::new(Value::Number(
                                Decimal::from(changed),
                            )))),
                            Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                        }
                    }
                }
            }
            // மொங்கோ_நீக்கு(தொகுப்பு, வடிகட்டி, அனைத்துமா)
            //
            // அனைத்துமா is not defaulted on purpose: "delete one" and "delete
            // everything that matches" are different enough that a caller
            // should have to say which, and the wrong default is unrecoverable.
            #[cfg(feature = "mongodb")]
            "மொங்கோ_நீக்கு" | "mowkO_nIkku" | "_mongoDelete" => {
                Self::expect_args(name, &args, 3)?;
                let collection = args[0].to_string();
                let filter = match crate::mongo::to_document(&args[1]) {
                    Ok(document) => document,
                    Err(why) => return Ok(Value::Err(Box::new(Value::String(why)))),
                };
                let many = args[2].is_truthy();
                match Self::mongo_of(&self.documents) {
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    Ok(connection) => match connection.delete(&collection, filter, many) {
                        Ok(gone) => {
                            Ok(Value::Ok(Box::new(Value::Number(Decimal::from(gone)))))
                        }
                        Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    },
                }
            }
            // Built without the feature: say so, rather than leaving someone to
            // hunt for a typo in a name that is spelled correctly.
            #[cfg(not(feature = "mongodb"))]
            "மொங்கோ_இணை" | "mowkO_iNY" | "_mongoConnect" => Err(
                "மொங்கோ ஆதரவு இல்லாமல் கட்டப்பட்டது  \
                 (this build has no MongoDB support): rebuild with --features mongodb"
                    .to_string(),
            ),

            // --- A database write that can fail without ending the program ---
            //
            // தளம்_செய் and தளம்_வினா are statements, and a statement has
            // nowhere to put an answer. Two things followed from that, and both
            // are real:
            //
            // The row count went nowhere. The driver returns how many rows a
            // statement touched and the VM dropped it, so a program could not
            // tell an UPDATE that matched a row from one that matched none —
            // and an UPDATE matching nothing is a silent no-op, which is
            // exactly the class of failure this language refuses elsewhere.
            //
            // A constraint violation ended the program. A duplicate key is not
            // a broken program: it is the database enforcing a rule, and the
            // ordinary answer is 409, not a crash. Under the server it took the
            // request handler with it and became a 500. The workaround —
            // SELECT first, then insert — is both slower and racy, because two
            // requests can pass the check before either writes.
            //
            // So these two attempt the statement and answer a முடிவு. The
            // statements stay: `தளம்_செய்` still insists, and insisting is
            // right when a failure really is unrecoverable.

            // தளம்_செய்_முயற்சி(வினா, அளபுருக்கள்) — attempt it; answers the
            // number of rows touched, or why not
            "தளம்_செய்_முயற்சி" | "qaLam_cey_muyaRci" | "_tryExecute" => {
                Self::expect_args(name, &args, 2)?;
                let sql = args[0].to_string();
                let params = match crate::db::params_from(&args[1]) {
                    Ok(params) => params,
                    Err(why) => return Ok(Value::Err(Box::new(Value::String(why)))),
                };
                match self.connection_mut() {
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    Ok(handle) => match handle.execute(&sql, &params) {
                        Ok(touched) => Ok(Value::Ok(Box::new(Value::Number(
                            Decimal::from(touched),
                        )))),
                        Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    },
                }
            }
            // தளம்_வினா_முயற்சி(வினா, அளபுருக்கள்) — attempt a query; answers
            // the rows, or why not
            //
            // No rows is a successful query answering nothing, and stays a
            // சரி holding an empty array. Only a query that could not run is a
            // தவறு — a missing column, a syntax error, a lost connection.
            "தளம்_வினா_முயற்சி" | "qaLam_viZA_muyaRci" | "_tryQuery" => {
                Self::expect_args(name, &args, 2)?;
                let sql = args[0].to_string();
                let params = match crate::db::params_from(&args[1]) {
                    Ok(params) => params,
                    Err(why) => return Ok(Value::Err(Box::new(Value::String(why)))),
                };
                match self.connection_mut() {
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    Ok(handle) => match handle.query(&sql, &params) {
                        Ok(rows) => Ok(Value::Ok(Box::new(Value::Array(rows)))),
                        Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    },
                }
            }

            // --- Authentication ---
            // bcrypt, HMAC-SHA256, base64 and randomness are not expressible
            // in eTamil, so they live in the host. Everything above them —
            // who a user is, which route needs which role — stays in the
            // language. A token's payload crosses as JSON text, which
            // nUlakam/vativam/jEcAZ.qmz reads and writes.
            // கடவுச்சொல்_மறை(கடவுச்சொல்) — hash a password for storage
            "கடவுச்சொல்_மறை" | "kataveuccol_maRY" | "_hashPassword" => {
                Self::expect_args(name, &args, 1)?;
                Ok(Value::String(crate::http::auth::hash_password(
                    &args[0].to_string(),
                )?))
            }
            // கடவுச்சொல்_சரியா(கடவுச்சொல், மறையீடு) — does it match?
            "கடவுச்சொல்_சரியா" | "kataveuccol_cariyA" | "_verifyPassword" => {
                Self::expect_args(name, &args, 2)?;
                Ok(Value::Boolean(crate::http::auth::verify_password(
                    &args[0].to_string(),
                    &args[1].to_string(),
                )?))
            }
            // சீட்டு_ஆக்கு(சுமை_ஜேசான், நொடிகள்) — sign a token.
            //
            // Named சீட்டு rather than the more literal குறியீடு because that
            // word is already the SQL INDEX keyword and is hard reserved: a
            // caller writing the obvious `குறியீடு = குறியீடு_ஆக்கு(...)`
            // would get a parse error on their own variable.
            "சீட்டு_ஆக்கு" | "cIttu_Akku" | "_issueToken" => {
                Self::expect_args(name, &args, 2)?;
                let seconds = rust_decimal::prelude::ToPrimitive::to_i64(&args[1].to_number())
                    .ok_or("நொடிகள் ஒரு முழு எண்  (the lifetime must be a whole number of seconds)")?;
                Ok(Value::String(crate::http::auth::issue_token(
                    &args[0].to_string(),
                    seconds,
                )?))
            }
            // சீட்டு_சரிபார்(சீட்டு) — verify, yielding the claims as JSON
            // text. A bad or expired token is a தவறு, not an error, so
            // rejecting a request is ordinary control flow.
            "சீட்டு_சரிபார்" | "cIttu_caripAr" | "_readToken" => {
                Self::expect_args(name, &args, 1)?;
                match crate::http::auth::read_token(&args[0].to_string()) {
                    Ok(claims) => Ok(Value::Ok(Box::new(Value::String(claims)))),
                    Err(message) => Ok(Value::Err(Box::new(Value::String(message)))),
                }
            }
            // --- Bytes ---
            //
            // A byte array is an ordinary array of numbers, not a new kind of
            // value. That is deliberate: a Value::Bytes variant would touch
            // every exhaustive match in the value path — the interpreter, all
            // three database drivers, the JSON library, the checker, the LLVM
            // backend — to add a type the language could otherwise represent
            // already. The cost is space, since each byte becomes a Decimal;
            // the gain is that base64 and hex are ordinary eTamil in
            // nUlakam/vativam/kuRiyAkkam.qmz rather than more host code.
            //
            // The limit worth knowing: a சொல் is valid UTF-8, so arbitrary
            // bytes can live in an array but not in a string. Encode them
            // first — which is what base64 is for.
            // பைட்டுகள்(சரம்) — the UTF-8 bytes of some text
            "பைட்டுகள்" | "pYttukaL" | "_bytes" => {
                Self::expect_args(name, &args, 1)?;
                let bytes = args[0]
                    .to_string()
                    .into_bytes()
                    .into_iter()
                    .map(|byte| Value::Number(Decimal::from(byte)))
                    .collect();
                Ok(Value::Array(bytes))
            }
            // பைட்டுச்_சரம்(அணி) — text from bytes, as a result
            //
            // A result rather than an error: bytes that are not valid UTF-8
            // are an ordinary thing to receive from outside, and a caller
            // should be able to handle it.
            "பைட்டுச்_சரம்" | "pYttuc_caram" | "_fromBytes" => {
                Self::expect_args(name, &args, 1)?;
                let items = match &args[0] {
                    Value::Array(items) => items,
                    other => {
                        return Err(format!(
                            "பைட்டுச்_சரம் ஒரு அணி தேவை  (fromBytes needs an array, got {})",
                            Self::type_name(other)
                        ));
                    }
                };

                let mut bytes = Vec::with_capacity(items.len());
                for (position, item) in items.iter().enumerate() {
                    let raw = item.to_number();
                    let byte = rust_decimal::prelude::ToPrimitive::to_u16(&raw)
                        .filter(|value| *value <= 255)
                        .filter(|_| raw.fract() == Decimal::ZERO);
                    match byte {
                        Some(byte) => bytes.push(byte as u8),
                        None => {
                            return Err(format!(
                                "இடம் {} இல் '{}' ஒரு பைட்டு அல்ல                                   (position {}: '{}' is not a byte, expected a whole number 0-255)",
                                position, raw, position, raw
                            ));
                        }
                    }
                }

                match String::from_utf8(bytes) {
                    Ok(text) => Ok(Value::Ok(Box::new(Value::String(text)))),
                    Err(_) => Ok(Value::Err(Box::new(Value::String(
                        "இந்தப் பைட்டுகள் செல்லுபடியான UTF-8 அல்ல                           (these bytes are not valid UTF-8)"
                            .to_string(),
                    )))),
                }
            }
            // --- Signing ---
            // HMAC needs bytes and a constant-time comparison, neither of
            // which the language has. What is signed, and what a signature
            // means, stays in eTamil.
            // கையொப்பம்(விசை, செய்தி) — HMAC-SHA256 as lowercase hex
            // --- Authenticated encryption ---------------------------------
            // XChaCha20-Poly1305 over an Argon2id key, in crate::crypt. These
            // three are why மறை and வெளிப்படு stopped being reserved words with
            // nothing behind them.

            // மறை(உரை, கடவுச்சொல்) — encrypt, as one base64 string
            "மறை" | "maRY" | "_encrypt" => {
                Self::expect_args(name, &args, 2)?;
                let sealed = crate::crypt::encrypt(&args[0].to_string(), &args[1].to_string())?;
                Ok(Value::String(sealed))
            }
            // வெளிப்படு(மறையீடு, கடவுச்சொல்) — decrypt, as a result
            //
            // A result rather than a runtime error: a wrong passphrase and a
            // tampered file are things a program handles, not things that
            // should stop it. `இயல்பு` and `?` work on it like any other.
            "வெளிப்படு" | "veLippatu" | "_decrypt" => {
                Self::expect_args(name, &args, 2)?;
                match crate::crypt::decrypt(&args[0].to_string(), &args[1].to_string()) {
                    Ok(text) => Ok(Value::Ok(Box::new(Value::String(text)))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // மறை_விசை() — a fresh passphrase, for a program that would rather
            // store a strong one than ask a person to invent it
            "மறை_விசை" | "maRY_vicY" | "_encryptionKey" => {
                Self::expect_args(name, &args, 0)?;
                Ok(Value::String(crate::crypt::fresh_key()?))
            }
            "கையொப்பம்" | "kYyoppam" | "_sign" => {
                Self::expect_args(name, &args, 2)?;
                Ok(Value::String(crate::net::sign(
                    &args[0].to_string(),
                    &args[1].to_string(),
                )))
            }
            // கையொப்பம்_சரியா(விசை, செய்தி, கையொப்பம்) — verify one
            //
            // Use this rather than comparing கையொப்பம்(...) with `==`: that
            // comparison stops at the first wrong character, and how long it
            // took reveals how much of the signature was right.
            "கையொப்பம்_சரியா" | "kYyoppam_cariyA" | "_verifySignature" => {
                Self::expect_args(name, &args, 3)?;
                Ok(Value::Boolean(crate::net::verify(
                    &args[0].to_string(),
                    &args[1].to_string(),
                    &args[2].to_string(),
                )))
            }
            // சுருக்கம்_256(செய்தி) — the SHA-256 digest, as bytes
            //
            // Bytes, not hex: kuRiyAkkam.qmz turns an array of bytes into
            // base64 or hex in eTamil, so the host does only the part the
            // language cannot — the digest is bit work and eTamil has no bit
            // operators.
            "சுருக்கம்_256" | "curukkam_256" | "_sha256" => {
                Self::expect_args(name, &args, 1)?;
                let digest = crate::crypt::sha256(&args[0].to_string())
                    .into_iter()
                    .map(|byte| Value::Number(Decimal::from(byte)))
                    .collect();
                Ok(Value::Array(digest))
            }
            // எழுமாற்று(எத்தனை) — that many unguessable bytes
            //
            // The clock was the only source of variety an eTamil program had,
            // and a value derived from the clock is one an attacker derives
            // too. A state, a nonce and a PKCE verifier all have to be
            // unguessable or the flow they protect is decoration.
            "எழுமாற்று" | "ezumARRu" | "_randomBytes" => {
                Self::expect_args(name, &args, 1)?;
                let count = args[0].to_number();
                let count = rust_decimal::prelude::ToPrimitive::to_usize(&count).ok_or_else(|| {
                    "எழுமாற்று ஒரு முழு எண்ணை எதிர்பார்க்கிறது                       (எழுமாற்று wants a whole number of bytes)"
                        .to_string()
                })?;
                if count == 0 || count > 1024 {
                    return Err(format!(
                        "எழுமாற்று 1 முதல் 1024 பைட்டுகள் வரை; {} கேட்கப்பட்டது                           (எழுமாற்று gives between 1 and 1024 bytes, asked for {})",
                        count, count
                    ));
                }
                let bytes = crate::crypt::random_bytes(count)?
                    .into_iter()
                    .map(|byte| Value::Number(Decimal::from(byte)))
                    .collect();
                Ok(Value::Array(bytes))
            }
            // நேர்வடிவம்(XML, அடையாளம், வடிவம்) — canonical XML
            //
            // An XML signature is not over the bytes that arrived: two
            // documents a parser calls identical can differ in attribute
            // order, in <a/> against <a></a>, in where a namespace is
            // declared. So it is over a canonical serialisation, and both
            // ends must produce the same one or nothing verifies.
            //
            // அடையாளம் is the Id the Reference names, or "" for the whole
            // document. வடிவம் is "விலக்கு" for Exclusive — what SAML and
            // Aadhaar use, and what lets a signed subtree be moved — or
            // "உள்ளடக்கு" for Inclusive.
            "நேர்வடிவம்" | "nErvativam" | "_canonicalXml" => {
                Self::expect_args(name, &args, 3)?;
                let source = args[0].to_string();
                let id = args[1].to_string();
                let form = match args[2].to_string().as_str() {
                    "விலக்கு" | "vilakku" | "exclusive" => crate::xmlsig::Form::Exclusive,
                    "உள்ளடக்கு" | "uLLatakku" | "inclusive" => crate::xmlsig::Form::Inclusive,
                    other => {
                        return Ok(Value::Err(Box::new(Value::String(format!(
                            "'{}' ஒரு நேர்வடிவ முறை அல்ல  ('{}' is not a canonicalization form)",
                            other, other
                        )))));
                    }
                };
                let wanted = if id.is_empty() { None } else { Some(id.as_str()) };
                match crate::xmlsig::canonicalize(&source, wanted, form) {
                    Ok(text) => Ok(Value::Ok(Box::new(Value::String(text)))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // உறுப்பைத்_தேடு(XML, பெயர், வடிவம்) — an element three ways
            //
            // Verifying means reading values back out: the SignedInfo the
            // signature covers, the DigestValue to compare against. Matched on
            // local name, because which prefix a document picked for the XML
            // Signature namespace is its own business.
            //
            // நேர்வடிவம் is the canonical form, உரைப்பு the text it contains,
            // and மூலம் the element's own bytes out of the source. That last one
            // is what the enveloped-signature transform needs: an enveloped
            // signature is removed from the document before the document is
            // digested, and removing this exact substring is what leaves every
            // other byte as it was when it was signed.
            "உறுப்பைத்_தேடு" | "uRuppYq_qEtu" | "_xmlElement" => {
                Self::expect_args(name, &args, 3)?;
                let form = match args[2].to_string().as_str() {
                    "விலக்கு" | "vilakku" | "exclusive" => crate::xmlsig::Form::Exclusive,
                    "உள்ளடக்கு" | "uLLatakku" | "inclusive" => crate::xmlsig::Form::Inclusive,
                    other => {
                        return Ok(Value::Err(Box::new(Value::String(format!(
                            "'{}' ஒரு நேர்வடிவ முறை அல்ல  ('{}' is not a canonicalization form)",
                            other, other
                        )))));
                    }
                };
                match crate::xmlsig::find_element(&args[0].to_string(), &args[1].to_string(), form) {
                    Ok((canonical, text, original)) => {
                        let mut record = HashMap::with_capacity(3);
                        record.insert("நேர்வடிவம்".to_string(), Value::String(canonical));
                        record.insert("உரைப்பு".to_string(), Value::String(text));
                        record.insert("மூலம்".to_string(), Value::String(original));
                        Ok(Value::Ok(Box::new(Value::Map(record.into()))))
                    }
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // வளைவு_நேர்_கையொப்பம்(செய்தி, தனிச்சாவி) — sign, as sixty-four bytes
            //
            // The same signature வளைவு_கையொப்பம் makes, in the encoding XML
            // Signature specifies: r and s side by side, each padded to the
            // curve size. Bytes rather than text, so the caller chooses base64
            // or hex — kuRiyAkkam.qmz does either.
            "வளைவு_நேர்_கையொப்பம்" | "vaLYvu_nEr_kYyoppam" | "_ecSignFixed" => {
                Self::expect_args(name, &args, 2)?;
                match crate::signing::sign_fixed(&args[0].to_string(), &args[1].to_string()) {
                    Ok(bytes) => Ok(Value::Ok(Box::new(Value::Array(
                        bytes.into_iter().map(|b| Value::Number(Decimal::from(b))).collect(),
                    )))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // வளைவு_நேர்_சரிபார்(செய்தி, பைட்டுகள், பொதுச்சாவி) — verify one
            "வளைவு_நேர்_சரிபார்" | "vaLYvu_nEr_caripAr" | "_ecVerifyFixed" => {
                Self::expect_args(name, &args, 3)?;
                let bytes: Vec<u8> = match &args[1] {
                    Value::Array(items) => items
                        .iter()
                        .map(|item| {
                            rust_decimal::prelude::ToPrimitive::to_u8(&item.to_number()).unwrap_or(0)
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                match crate::signing::verify_fixed(
                    &args[0].to_string(),
                    &bytes,
                    &args[2].to_string(),
                ) {
                    Ok(answer) => Ok(Value::Boolean(answer)),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // --- AMQP: a message broker, and what "sent" means ---
            //
            // The channel runs in confirm mode and every publish is mandatory,
            // so செய்தி_அனுப்பு answering சரி means the broker has taken
            // responsibility for the message *and* it reached a queue. Those
            // are two different failures and neither is visible from a write
            // that succeeded, which is what a publish without confirms tells
            // you. src/amqp.rs has the reasoning.
            //
            // One connection, held on the VM, as ரெடிஸ்_இணை does — a channel
            // has per-channel state and sharing one has the hazard of sharing
            // a transaction.

            // செய்தி_இணை(உரலி) — amqp://user:pass@host:port/vhost
            #[cfg(not(target_family = "wasm"))]
            "செய்தி_இணை" | "ceyqi_iNY" | "_mqConnect" => {
                Self::expect_args(name, &args, 1)?;
                let url = args[0].to_string();
                match crate::amqp::Connection::open(&url) {
                    Ok(connection) => {
                        self.broker = Some(connection);
                        Ok(Value::Ok(Box::new(Value::String(url))))
                    }
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // செய்தி_வரிசை(பெயர், நிலையானதா) — declare a queue, and say how
            // many messages are waiting in it
            #[cfg(not(target_family = "wasm"))]
            "செய்தி_வரிசை" | "ceyqi_varicY" | "_mqQueue" => {
                Self::expect_args(name, &args, 2)?;
                let (queue, durable) = (args[0].to_string(), args[1].is_truthy());
                match self.broker.as_mut() {
                    Some(connection) => match connection.declare_queue(&queue, durable) {
                        Ok(waiting) => {
                            Ok(Value::Ok(Box::new(Value::Number(Decimal::from(waiting)))))
                        }
                        Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    },
                    None => Ok(Self::no_broker()),
                }
            }
            // செய்தி_பரிமாற்றி(பெயர், வகை, நிலையானதா) — direct, topic, fanout, headers
            #[cfg(not(target_family = "wasm"))]
            "செய்தி_பரிமாற்றி" | "ceyqi_parimARRi" | "_mqExchange" => {
                Self::expect_args(name, &args, 3)?;
                let (exchange, kind, durable) =
                    (args[0].to_string(), args[1].to_string(), args[2].is_truthy());
                match self.broker.as_mut() {
                    Some(connection) => {
                        match connection.declare_exchange(&exchange, &kind, durable) {
                            Ok(()) => Ok(Value::Ok(Box::new(Value::Boolean(true)))),
                            Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                        }
                    }
                    None => Ok(Self::no_broker()),
                }
            }
            // செய்தி_பிணை(வரிசை, பரிமாற்றி, திறவுகோல்)
            #[cfg(not(target_family = "wasm"))]
            "செய்தி_பிணை" | "ceyqi_piNY" | "_mqBind" => {
                Self::expect_args(name, &args, 3)?;
                let (queue, exchange, key) =
                    (args[0].to_string(), args[1].to_string(), args[2].to_string());
                match self.broker.as_mut() {
                    Some(connection) => match connection.bind(&queue, &exchange, &key) {
                        Ok(()) => Ok(Value::Ok(Box::new(Value::Boolean(true)))),
                        Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    },
                    None => Ok(Self::no_broker()),
                }
            }
            // செய்தி_அனுப்பு(பரிமாற்றி, திறவுகோல், உடல், பண்புகள்)
            //
            // Returns சரி only when the broker has confirmed it and it routed
            // somewhere. A message an exchange accepted and dropped comes back
            // தவறு naming the routing key, because "published successfully"
            // and "nothing is listening" look identical from the socket.
            #[cfg(not(target_family = "wasm"))]
            "செய்தி_அனுப்பு" | "ceyqi_aZuppu" | "_mqPublish" => {
                Self::expect_args(name, &args, 4)?;
                let (exchange, key, body) =
                    (args[0].to_string(), args[1].to_string(), args[2].to_string());
                let (content_type, correlation, message_id) = match &args[3] {
                    Value::Map(fields) => crate::amqp::properties(fields),
                    _ => (String::new(), String::new(), String::new()),
                };
                match self.broker.as_mut() {
                    Some(connection) => match connection.publish(
                        &exchange,
                        &key,
                        body.as_bytes(),
                        &content_type,
                        &correlation,
                        &message_id,
                    ) {
                        Ok(crate::amqp::Published::Confirmed) => {
                            Ok(Value::Ok(Box::new(Value::Boolean(true))))
                        }
                        Ok(crate::amqp::Published::Nacked) => Ok(Value::Err(Box::new(
                            Value::String(
                                "தரகர் செய்தியை ஏற்கவில்லை  \
                                 (the broker refused responsibility for the message)"
                                    .to_string(),
                            ),
                        ))),
                        Ok(crate::amqp::Published::Returned { code, text }) => {
                            Ok(Value::Err(Box::new(Value::String(format!(
                                "'{}' திறவுகோலுக்கு வரிசை இல்லை ({}: {})  \
                                 (nothing is bound to route '{}', so it was discarded)",
                                key, code, text, key
                            )))))
                        }
                        Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    },
                    None => Ok(Self::no_broker()),
                }
            }
            // செய்தி_பெறு(வரிசை) — one message, or இன்மை if the queue is empty
            //
            // Not acknowledged by taking it: it stays the broker's
            // responsibility until செய்தி_ஏற்பு, so a program that reads one
            // and then fails leaves it to be delivered again.
            #[cfg(not(target_family = "wasm"))]
            "செய்தி_பெறு" | "ceyqi_peRu" | "_mqGet" => {
                Self::expect_args(name, &args, 1)?;
                let queue = args[0].to_string();
                match self.broker.as_mut() {
                    Some(connection) => match connection.get(&queue) {
                        Ok(None) => Ok(Value::Ok(Box::new(Value::Null))),
                        Ok(Some(delivery)) => {
                            Ok(Value::Ok(Box::new(Self::delivery_value(&delivery))))
                        }
                        Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    },
                    None => Ok(Self::no_broker()),
                }
            }
            // செய்தி_ஏற்பு(குறிச்சொல்) — done with it
            #[cfg(not(target_family = "wasm"))]
            "செய்தி_ஏற்பு" | "ceyqi_ERpu" | "_mqAck" => {
                Self::expect_args(name, &args, 1)?;
                let tag = rust_decimal::prelude::ToPrimitive::to_u64(&args[0].to_number())
                    .unwrap_or_default();
                match self.broker.as_mut() {
                    Some(connection) => match connection.ack(tag) {
                        Ok(()) => Ok(Value::Ok(Box::new(Value::Boolean(true)))),
                        Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    },
                    None => Ok(Self::no_broker()),
                }
            }
            // செய்தி_மறு(குறிச்சொல், மீண்டுமா) — not done with it
            //
            // மீண்டுமா puts it back for another attempt. Without it the message
            // is dead-lettered if the queue says where and dropped if it does
            // not, so rejecting from a queue with no dead-letter exchange
            // discards — worth knowing before rejecting anything.
            #[cfg(not(target_family = "wasm"))]
            "செய்தி_மறு" | "ceyqi_maRu" | "_mqNack" => {
                Self::expect_args(name, &args, 2)?;
                let tag = rust_decimal::prelude::ToPrimitive::to_u64(&args[0].to_number())
                    .unwrap_or_default();
                let requeue = args[1].is_truthy();
                match self.broker.as_mut() {
                    Some(connection) => match connection.nack(tag, requeue) {
                        Ok(()) => Ok(Value::Ok(Box::new(Value::Boolean(true)))),
                        Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                    },
                    None => Ok(Self::no_broker()),
                }
            }
            // செய்தி_பிரி() — close politely, rather than letting the socket go
            #[cfg(not(target_family = "wasm"))]
            "செய்தி_பிரி" | "ceyqi_piri" | "_mqClose" => {
                Self::expect_args(name, &args, 0)?;
                match self.broker.as_mut() {
                    Some(connection) => {
                        let answer = connection.close();
                        self.broker = None;
                        match answer {
                            Ok(()) => Ok(Value::Ok(Box::new(Value::Boolean(true)))),
                            Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                        }
                    }
                    None => Ok(Self::no_broker()),
                }
            }

            // No socket in a browser, so no broker to reach with one. Said
            // rather than reported as an unknown name, for the reason the
            // MongoDB arm below gives.
            #[cfg(target_family = "wasm")]
            "செய்தி_இணை" | "ceyqi_iNY" | "_mqConnect" | "செய்தி_வரிசை" | "ceyqi_varicY" | "_mqQueue" | "செய்தி_பரிமாற்றி" | "ceyqi_parimARRi" | "_mqExchange" | "செய்தி_பிணை" | "ceyqi_piNY" | "_mqBind" | "செய்தி_அனுப்பு" | "ceyqi_aZuppu" | "_mqPublish" | "செய்தி_பெறு" | "ceyqi_peRu" | "_mqGet" | "செய்தி_ஏற்பு" | "ceyqi_ERpu" | "_mqAck" | "செய்தி_மறு" | "ceyqi_maRu" | "_mqNack" | "செய்தி_பிரி" | "ceyqi_piri" | "_mqClose" => Err(
                "உலாவியில் தரகர் இல்லை  \
                 (a browser has no socket to reach a message broker with)"
                    .to_string(),
            ),

            // --- PKCS#11: a key that is not in this process ---
            //
            // An HSM's whole proposition is that the private key never leaves
            // it, so there is no builtin here that returns one and nowhere to
            // put it if there were. You hand the device something to sign and
            // take back the signature.
            //
            // What comes back is r and s side by side, which is what XML
            // Signature specifies — so வன்சாவி_கையொப்பம் and
            // வளைவு_நேர்_கையொப்பம் are interchangeable, and முத்திரையிடு takes
            // either. That is the point of the pair: moving a signing key into
            // an HSM should change which function is called and nothing else.
            //
            // Behind --features pkcs11. Sessions belong to the thread that
            // opened them; see src/pkcs11.rs.

            // வன்சாவி_திற(நூலகம், டோக்கன், முள்) — open a session and log in
            #[cfg(all(feature = "pkcs11", not(target_family = "wasm")))]
            "வன்சாவி_திற" | "vaZcAvi_qiRa" | "_hsmOpen" => {
                Self::expect_args(name, &args, 3)?;
                match crate::pkcs11::open(
                    &args[0].to_string(),
                    &args[1].to_string(),
                    &args[2].to_string(),
                ) {
                    Ok(handle) => Ok(Value::Ok(Box::new(Value::Number(Decimal::from(handle))))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // வன்சாவி_மூடு(அமர்வு) — log out and close
            #[cfg(all(feature = "pkcs11", not(target_family = "wasm")))]
            "வன்சாவி_மூடு" | "vaZcAvi_mUtu" | "_hsmClose" => {
                Self::expect_args(name, &args, 1)?;
                let handle =
                    rust_decimal::prelude::ToPrimitive::to_i64(&args[0].to_number()).unwrap_or(0);
                match crate::pkcs11::close(handle) {
                    Ok(()) => Ok(Value::Ok(Box::new(Value::Boolean(true)))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // வன்சாவி_சாவிகள்(அமர்வு) — the labels this session can sign with
            #[cfg(all(feature = "pkcs11", not(target_family = "wasm")))]
            "வன்சாவி_சாவிகள்" | "vaZcAvi_cAvikaL" | "_hsmKeys" => {
                Self::expect_args(name, &args, 1)?;
                let handle =
                    rust_decimal::prelude::ToPrimitive::to_i64(&args[0].to_number()).unwrap_or(0);
                match crate::pkcs11::keys(handle) {
                    Ok(labels) => Ok(Value::Ok(Box::new(Value::Array(
                        labels.into_iter().map(Value::String).collect(),
                    )))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // வன்சாவி_பொதுச்சாவி(அமர்வு, பெயர்) — the public half, as hex
            #[cfg(all(feature = "pkcs11", not(target_family = "wasm")))]
            "வன்சாவி_பொதுச்சாவி" | "vaZcAvi_poquccAvi" | "_hsmPublicKey" => {
                Self::expect_args(name, &args, 2)?;
                let handle =
                    rust_decimal::prelude::ToPrimitive::to_i64(&args[0].to_number()).unwrap_or(0);
                match crate::pkcs11::public_key(handle, &args[1].to_string()) {
                    Ok(hex) => Ok(Value::Ok(Box::new(Value::String(hex)))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // வன்சாவி_கையொப்பம்(அமர்வு, பெயர், செய்தி) — sign, as sixty-four bytes
            #[cfg(all(feature = "pkcs11", not(target_family = "wasm")))]
            "வன்சாவி_கையொப்பம்" | "vaZcAvi_kYyoppam" | "_hsmSign" => {
                Self::expect_args(name, &args, 3)?;
                let handle =
                    rust_decimal::prelude::ToPrimitive::to_i64(&args[0].to_number()).unwrap_or(0);
                match crate::pkcs11::sign(handle, &args[1].to_string(), &args[2].to_string()) {
                    Ok(bytes) => Ok(Value::Ok(Box::new(Value::Array(
                        bytes.into_iter().map(|b| Value::Number(Decimal::from(b))).collect(),
                    )))),
                    Err(why) => Ok(Value::Err(Box::new(Value::String(why)))),
                }
            }
            // Built without the feature: say so, rather than leaving someone to
            // hunt for a typo in a name that is spelled correctly.
            #[cfg(not(all(feature = "pkcs11", not(target_family = "wasm"))))]
            "வன்சாவி_திற" | "vaZcAvi_qiRa" | "_hsmOpen" | "வன்சாவி_மூடு" | "vaZcAvi_mUtu"
            | "_hsmClose" | "வன்சாவி_சாவிகள்" | "vaZcAvi_cAvikaL" | "_hsmKeys"
            | "வன்சாவி_பொதுச்சாவி" | "vaZcAvi_poquccAvi" | "_hsmPublicKey"
            | "வன்சாவி_கையொப்பம்" | "vaZcAvi_kYyoppam" | "_hsmSign" => Err(
                "PKCS#11 ஆதரவு இல்லாமல் கட்டப்பட்டது  \
                 (this build has no PKCS#11 support): rebuild with --features pkcs11"
                    .to_string(),
            ),

            // --- Outbound HTTP ---
            // வலை_பெறு(உரலி, தலைப்புகள்)
            "வலை_பெறு" | "valY_peRu" | "_httpGet" => {
                Self::expect_args(name, &args, 2)?;
                Ok(Self::http_call("GET", &args[0], None, &args[1]))
            }
            // வலை_பதி(உரலி, உடலுரை, தலைப்புகள்)
            "வலை_பதி" | "valY_paqi" | "_httpPost" => {
                Self::expect_args(name, &args, 3)?;
                Ok(Self::http_call("POST", &args[0], Some(&args[1]), &args[2]))
            }
            // வலை_அனுப்பு(முறை, உரலி, உடலுரை, தலைப்புகள்) — any method
            "வலை_அனுப்பு" | "valY_aZuppu" | "_httpRequest" => {
                Self::expect_args(name, &args, 4)?;
                let method = args[0].to_string().to_uppercase();
                let body = match &args[2] {
                    // A method with no body passes இன்மை rather than "".
                    Value::Null => None,
                    other => Some(other),
                };
                Ok(Self::http_call(&method, &args[1], body, &args[3]))
            }
            unknown => Err(format!(
                "அறியப்படாத செயல் '{}'  (unknown function '{}')",
                unknown, unknown
            )),
        }
    }

    /// Read an ISO-8601 date as a day number, counting from 1970-01-01.
    ///
    /// Done here rather than with a date crate: the conversion is a closed
    /// form (Howard Hinnant's civil-calendar algorithm), it is exact for any
    /// year, and it keeps a calendar library out of the interpreter.
    fn parse_date(value: &Value) -> Result<i64, String> {
        let text = value.to_string();
        let bad = || {
            format!(
                "'{}' ஒரு நாள் அல்ல  ('{}' is not a date; expected YYYY-MM-DD)",
                text, text
            )
        };

        let parts: Vec<&str> = text.trim().split('-').collect();
        if parts.len() != 3 {
            return Err(bad());
        }
        let year: i64 = parts[0].parse().map_err(|_| bad())?;
        let month: i64 = parts[1].parse().map_err(|_| bad())?;
        let day: i64 = parts[2].parse().map_err(|_| bad())?;

        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return Err(bad());
        }
        Ok(Self::days_from_civil(year, month, day))
    }

    /// Days from 1970-01-01 to a civil date.
    fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
        let y = if month <= 2 { year - 1 } else { year };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let mp = if month > 2 { month - 3 } else { month + 9 };
        let doy = (153 * mp + 2) / 5 + day - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe - 719468
    }

    /// The inverse: a civil date from a day number.
    fn civil_from_days(days: i64) -> (i64, i64, i64) {
        let z = days + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        (if m <= 2 { y + 1 } else { y }, m, d)
    }

    /// Format a day number back as ISO-8601.
    fn format_date(days: i64) -> String {
        let (year, month, day) = Self::civil_from_days(days);
        format!("{:04}-{:02}-{:02}", year, month, day)
    }

    /// Make one HTTP request, handing the language back a result.
    ///
    /// A non-2xx status is a *successful* call: a gateway declining a charge
    /// answers 402 with a body saying why, and reporting that as a தவறு would
    /// throw the explanation away. Only a request that never got an answer —
    /// DNS, TLS, connection, timeout — is a failure.
    fn http_call(method: &str, url: &Value, body: Option<&Value>, headers: &Value) -> Value {
        let headers: Vec<(String, String)> = match headers {
            Value::Map(fields) => fields
                .iter()
                .map(|(name, value)| (name.clone(), value.to_string()))
                .collect(),
            // இன்மை or anything else means "no headers", which is the common
            // case and not worth an error.
            _ => Vec::new(),
        };

        let body = body.map(|value| value.to_string());

        match crate::net::request(method, &url.to_string(), body.as_deref(), &headers) {
            Ok(response) => {
                let mut record = HashMap::with_capacity(3);
                record.insert(
                    "நிலைக்_குறி".to_string(),
                    Value::Number(Decimal::from(response.status)),
                );
                record.insert("உடலுரை".to_string(), Value::String(response.body));
                record.insert(
                    "தலைப்புகள்".to_string(),
                    Value::Map(
                        response
                            .headers
                            .into_iter()
                            .map(|(name, value)| (name, Value::String(value)))
                            .collect(),
                    ),
                );
                Value::Ok(Box::new(Value::Map(record.into())))
            }
            Err(message) => Value::Err(Box::new(Value::String(message))),
        }
    }

    /// JSON text -> an eTamil value.
    ///
    /// Numbers go through their decimal text rather than through f64: every
    /// number in this language is a fixed-point Decimal, and routing an amount
    /// through a binary float is exactly the bargain the language exists to
    /// refuse. That text is the source's own only because serde_json is built
    /// with `arbitrary_precision`; without it, `n.to_string()` is an f64 printed
    /// back. The source may use an exponent, which `parse` does not read.
    fn json_to_value(parsed: &serde_json::Value) -> Result<Value, String> {
        Ok(match parsed {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Boolean(*b),
            serde_json::Value::Number(n) => {
                let text = n.to_string();
                let parsed = if text.contains(['e', 'E']) {
                    Decimal::from_scientific(&text)
                } else {
                    text.parse::<Decimal>()
                };
                match parsed {
                    Ok(number) => Value::Number(number),
                    Err(_) => {
                        return Err(format!(
                            "ஒரு தசமமாக அடங்காத எண்: {}  (number does not fit a decimal)",
                            text
                        ));
                    }
                }
            }
            serde_json::Value::String(text) => Value::String(text.clone()),
            serde_json::Value::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(Self::json_to_value(item)?);
                }
                Value::Array(out)
            }
            serde_json::Value::Object(pairs) => {
                let mut fields = HashMap::with_capacity(pairs.len());
                for (key, item) in pairs {
                    fields.insert(key.clone(), Self::json_to_value(item)?);
                }
                Value::Map(fields.into())
            }
        })
    }

    /// The JSON spelling of a string, quotes included.
    fn json_string(text: &str) -> String {
        let mut out = String::with_capacity(text.len() + 2);
        out.push('"');
        for c in text.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                '\u{8}' => out.push_str("\\b"),
                '\u{c}' => out.push_str("\\f"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }

    /// An eTamil value -> JSON text.
    ///
    /// Written out rather than handed to serde so that a Decimal prints its
    /// own exact text. serde_json has no decimal number type, and going
    /// through f64 would turn 0.1 into 0.1000000000000000055511151231257827 —
    /// the bargain this language exists to refuse.
    fn value_to_json(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(text) => Self::json_string(text),
            Value::Array(items) => {
                let parts: Vec<String> = items.iter().map(Self::value_to_json).collect();
                format!("[{}]", parts.join(","))
            }
            Value::Map(record) => {
                // Sorted, so the same record always produces the same text. A
                // HashMap iterates in an arbitrary order, and a request body
                // that changes shape between two identical calls defeats any
                // cache and makes a failure hard to reproduce.
                let mut keys: Vec<&String> = record.fields.keys().collect();
                keys.sort();
                let parts: Vec<String> = keys
                    .iter()
                    .map(|key| {
                        format!(
                            "{}:{}",
                            Self::json_string(key),
                            Self::value_to_json(&record.fields[*key])
                        )
                    })
                    .collect();
                format!("{{{}}}", parts.join(","))
            }
            // சரி, தவறு and a செயல் have no JSON spelling. This writes null,
            // which is what the eTamil version in jEcAZ.qmz did. Worth noting
            // that its own comment argued for refusing instead and the code
            // did not: that contradiction is preserved here rather than
            // decided, because changing it is a decision about the library's
            // contract and this change is about speed.
            _ => "null".to_string(),
        }
    }

    /// A board call's outcome as a result value: சரி(v), or தவறு(why).
    fn board_result(outcome: Result<Value, String>) -> Value {
        match outcome {
            Ok(value) => Value::Ok(Box::new(value)),
            Err(why) => Value::Err(Box::new(Value::String(why))),
        }
    }

    /// A whole number argument: a pin, a port, milliseconds.
    fn whole(value: &Value, what: &str) -> Result<i64, String> {
        let number = match value {
            Value::Number(n) => *n,
            other => {
                return Err(format!(
                    "{} ஒரு எண்ணாக இருக்க வேண்டும்  ({} must be a number, not {})",
                    what,
                    what,
                    Self::type_name(other)
                ));
            }
        };
        if !number.fract().is_zero() {
            return Err(format!(
                "{} முழு எண்ணாக இருக்க வேண்டும்  ({} must be a whole number, not {})",
                what, what, number
            ));
        }
        rust_decimal::prelude::ToPrimitive::to_i64(&number)
            .ok_or_else(|| format!("{} மிகப் பெரியது  ({} is too large: {})", what, what, number))
    }

    /// Said rather than panicked: a program that forgot செய்தி_இணை gets the
    /// name of the thing it forgot, not a missing-value error from inside.
    #[cfg(not(target_family = "wasm"))]
    fn no_broker() -> Value {
        Value::Err(Box::new(Value::String(
            "தரகருடன் இணைப்பு இல்லை  (no broker connection): use செய்தி_இணை first".to_string(),
        )))
    }

    /// A delivery as a record. The tag is what செய்தி_ஏற்பு and செய்தி_மறு
    /// take, so it is the field a program cannot do without; the rest is what
    /// the broker said about where the message came from.
    #[cfg(not(target_family = "wasm"))]
    fn delivery_value(delivery: &crate::amqp::Delivery) -> Value {
        let mut record = HashMap::with_capacity(8);
        record.insert(
            "குறிச்சொல்".to_string(),
            Value::Number(Decimal::from(delivery.tag)),
        );
        record.insert(
            "உடல்".to_string(),
            Value::String(String::from_utf8_lossy(&delivery.body).into_owned()),
        );
        record.insert(
            "மீண்டும்_வந்ததா".to_string(),
            Value::Boolean(delivery.redelivered),
        );
        record.insert(
            "பரிமாற்றி".to_string(),
            Value::String(delivery.exchange.clone()),
        );
        record.insert(
            "திறவுகோல்".to_string(),
            Value::String(delivery.routing_key.clone()),
        );
        record.insert(
            "காத்திருப்பவை".to_string(),
            Value::Number(Decimal::from(delivery.waiting)),
        );
        record.insert(
            "வகை".to_string(),
            Value::String(delivery.content_type.clone()),
        );
        record.insert(
            "தொடர்பு".to_string(),
            Value::String(delivery.correlation_id.clone()),
        );
        record.insert(
            "அடையாளம்".to_string(),
            Value::String(delivery.message_id.clone()),
        );
        Value::Map(record.into())
    }

    fn expect_args(name: &str, args: &[Value], want: usize) -> Result<(), String> {
        if args.len() != want {
            return Err(format!(
                "செயல் '{}' {} அளவுருக்களை எதிர்பார்க்கிறது, {} வழங்கப்பட்டது  \
                 (function '{}' expects {} argument(s), got {})",
                name,
                want,
                args.len(),
                name,
                want,
                args.len()
            ));
        }
        Ok(())
    }

    /// Append one line to a file, creating it if needed.
    /// Run a program to completion, capturing what it printed.
    ///
    /// Deny by default: ETAMIL_EXEC_ALLOW lists what may run, separated the
    /// way PATH is on this platform, and an empty or absent list permits
    /// nothing. A listed entry matches either the whole program string or its
    /// file name, so both "soffice" and a full path can be allowed.
    ///
    /// The arguments are handed over as a list. No shell sees them, so a
    /// document title full of semicolons is a title and not a second command.
    #[cfg(not(target_family = "wasm"))]
    fn run_program(
        program: &str,
        parameters: &[String],
        seconds: u64,
    ) -> Result<(i64, String, String), String> {
        let allowed = std::env::var("ETAMIL_EXEC_ALLOW").unwrap_or_default();
        let wanted_name = std::path::Path::new(program).file_name();
        let permitted = std::env::split_paths(&allowed).any(|entry| {
            entry.as_os_str() == std::ffi::OsStr::new(program)
                || (wanted_name.is_some() && entry.file_name() == wanted_name)
        });
        if !permitted {
            return Err(format!(
                "'{}' ETAMIL_EXEC_ALLOW இல் இல்லை  ('{}' is not listed in ETAMIL_EXEC_ALLOW)",
                program, program
            ));
        }

        let mut child = std::process::Command::new(program)
            .args(parameters)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                format!(
                    "'{}' இயக்க முடியவில்லை  (cannot run '{}'): {}",
                    program, program, e
                )
            })?;

        // Drained on their own threads: a program that prints more than the
        // pipe holds would otherwise block forever waiting to be read, and
        // the timeout below would never be what stopped it.
        let mut out_pipe = child.stdout.take();
        let mut err_pipe = child.stderr.take();
        let out_reader = std::thread::spawn(move || {
            let mut buffer = Vec::new();
            if let Some(pipe) = out_pipe.as_mut() {
                let _ = std::io::Read::read_to_end(pipe, &mut buffer);
            }
            buffer
        });
        let err_reader = std::thread::spawn(move || {
            let mut buffer = Vec::new();
            if let Some(pipe) = err_pipe.as_mut() {
                let _ = std::io::Read::read_to_end(pipe, &mut buffer);
            }
            buffer
        });

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if std::time::Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(format!(
                            "'{}' {} வினாடிகளுக்குள் முடியவில்லை  ('{}' did not finish within {}s)",
                            program, seconds, program, seconds
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(e) => {
                    return Err(format!(
                        "'{}' முடிவுக்காகக் காத்திருக்க முடியவில்லை  (cannot wait for '{}'): {}",
                        program, program, e
                    ));
                }
            }
        };

        let printed = out_reader.join().unwrap_or_default();
        let complained = err_reader.join().unwrap_or_default();
        Ok((
            status.code().unwrap_or(-1) as i64,
            String::from_utf8_lossy(&printed).to_string(),
            String::from_utf8_lossy(&complained).to_string(),
        ))
    }

    /// The MongoDB connection, or an explanation of why there is none.
    #[cfg(feature = "mongodb")]
    fn mongo_of(
        held: &Option<crate::mongo::Connection>,
    ) -> Result<&crate::mongo::Connection, String> {
        held.as_ref().ok_or_else(|| {
            "மொங்கோ இணைக்கப்படவில்லை  (not connected to MongoDB): \
             use மொங்கோ_இணை first"
                .to_string()
        })
    }

    // --- Browser twins ------------------------------------------------------
    //
    // These three want a subprocess or a `File` handed to the zip crate, and a
    // browser has neither. Their callers are match arms scattered through
    // `execute`; giving each a same-signature twin that fails at runtime keeps
    // every one of those call sites untouched, which is the whole reason the
    // VM compiles for wasm without `#[cfg]` sprinkled through its body.

    #[cfg(target_family = "wasm")]
    fn run_program(
        program: &str,
        _parameters: &[String],
        _seconds: u64,
    ) -> Result<(i64, String, String), String> {
        Err(format!(
            "'{}' உலாவியில் இயக்க முடியாது  (cannot run '{}' in the browser)",
            program, program
        ))
    }

    #[cfg(target_family = "wasm")]
    fn package_entry(path: &str, _entry: &str) -> Result<String, String> {
        Err(format!(
            "பொதி '{}' உலாவியில் திறக்க முடியாது  \
             (packages cannot be opened in the browser: '{}')",
            path, path
        ))
    }

    #[cfg(target_family = "wasm")]
    fn package_copy(
        source: &str,
        _target: &str,
        _changes: &HashMap<String, String>,
    ) -> Result<usize, String> {
        Err(format!(
            "பொதி '{}' உலாவியில் எழுத முடியாது  \
             (packages cannot be written in the browser: '{}')",
            source, source
        ))
    }

    /// One entry of a zip package, decoded as UTF-8.
    #[cfg(not(target_family = "wasm"))]
    fn package_entry(path: &str, entry: &str) -> Result<String, String> {
        let file = fs::File::open(path).map_err(|e| {
            format!(
                "பொதி '{}' திறக்க முடியவில்லை  (cannot open package '{}'): {}",
                path, path, e
            )
        })?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| {
            format!(
                "'{}' ஒரு பொதி அல்ல  ('{}' is not a package): {}",
                path, path, e
            )
        })?;
        let mut found = archive.by_name(entry).map_err(|_| {
            format!(
                "பொதியில் '{}' இல்லை  (no entry '{}' in the package)",
                entry, entry
            )
        })?;
        let mut text = String::new();
        std::io::Read::read_to_string(&mut found, &mut text).map_err(|e| {
            format!(
                "'{}' உரையாகப் படிக்க முடியவில்லை  (cannot read '{}' as text): {}",
                entry, entry, e
            )
        })?;
        Ok(text)
    }

    /// Copy a zip package entry by entry, swapping the named ones. Answers with
    /// how many entries the new package holds.
    ///
    /// Two rules the ODF format imposes, and this keeps: `mimetype` comes first
    /// and stays uncompressed, and every entry not being replaced is copied
    /// without being decompressed, so a picture arrives unchanged.
    #[cfg(not(target_family = "wasm"))]
    fn package_copy(
        source: &str,
        target: &str,
        changes: &HashMap<String, String>,
    ) -> Result<usize, String> {
        let reader = fs::File::open(source).map_err(|e| {
            format!(
                "பொதி '{}' திறக்க முடியவில்லை  (cannot open package '{}'): {}",
                source, source, e
            )
        })?;
        let mut archive = zip::ZipArchive::new(reader).map_err(|e| {
            format!(
                "'{}' ஒரு பொதி அல்ல  ('{}' is not a package): {}",
                source, source, e
            )
        })?;

        let names: Vec<String> = (0..archive.len())
            .map(|index| {
                archive
                    .by_index(index)
                    .map(|entry| entry.name().to_string())
            })
            .collect::<Result<_, _>>()
            .map_err(|e| format!("பொதியைப் படிக்க முடியவில்லை  (cannot read the package): {}", e))?;

        // A name that matches nothing is a typo, and quietly writing an
        // unchanged document would be the worst way to report it.
        for wanted in changes.keys() {
            if !names.contains(wanted) {
                return Err(format!(
                    "பொதியில் '{}' இல்லை  (no entry '{}' to replace)",
                    wanted, wanted
                ));
            }
        }

        let writer = fs::File::create(target).map_err(|e| {
            format!(
                "பொதி '{}' எழுத முடியவில்லை  (cannot write package '{}'): {}",
                target, target, e
            )
        })?;
        let mut out = zip::ZipWriter::new(writer);

        for index in 0..archive.len() {
            let entry = archive.by_index(index).map_err(|e| {
                format!("பொதியைப் படிக்க முடியவில்லை  (cannot read the package): {}", e)
            })?;
            let entry_name = entry.name().to_string();

            match changes.get(&entry_name) {
                Some(replacement) => {
                    // mimetype stays uncompressed even when it is rewritten.
                    let method = if entry_name == "mimetype" {
                        zip::CompressionMethod::Stored
                    } else {
                        zip::CompressionMethod::Deflated
                    };
                    let options: zip::write::SimpleFileOptions =
                        zip::write::SimpleFileOptions::default().compression_method(method);
                    out.start_file(&entry_name, options).map_err(|e| {
                        format!(
                            "'{}' எழுத முடியவில்லை  (cannot write '{}'): {}",
                            entry_name, entry_name, e
                        )
                    })?;
                    IoWrite::write_all(&mut out, replacement.as_bytes()).map_err(|e| {
                        format!(
                            "'{}' எழுத முடியவில்லை  (cannot write '{}'): {}",
                            entry_name, entry_name, e
                        )
                    })?;
                }
                None => {
                    // Copied still compressed: never decoded, so never damaged.
                    out.raw_copy_file(entry).map_err(|e| {
                        format!(
                            "'{}' நகலெடுக்க முடியவில்லை  (cannot copy '{}'): {}",
                            entry_name, entry_name, e
                        )
                    })?;
                }
            }
        }

        out.finish().map_err(|e| {
            format!(
                "பொதி '{}' முடிக்க முடியவில்லை  (cannot finish package '{}'): {}",
                target, target, e
            )
        })?;
        Ok(names.len())
    }

    fn append_line(filename: &str, data: &str) -> Result<(), String> {
        host::append_line(filename, data).map_err(|e| {
            format!(
                "கோப்பு '{}' எழுத முடியவில்லை  (cannot write '{}'): {}",
                filename, filename, e
            )
        })
    }

    pub fn execute(&mut self, bytecode: Bytecode) -> Result<(), String> {
        self.run(bytecode, None)
    }

    /// Execute, but give up after `max_steps` instructions.
    ///
    /// A browser tab cannot interrupt a `சுற்று` whose condition never goes
    /// false: the page simply stops responding, with no stack to look at and
    /// nothing to click. A ceiling turns that into an error message. Native
    /// callers use `execute`, which has no ceiling -- a long-running report is
    /// a legitimate thing for a server to do.
    pub fn execute_limited(&mut self, bytecode: Bytecode, max_steps: u64) -> Result<(), String> {
        self.run(bytecode, Some(max_steps))
    }

    fn run(&mut self, bytecode: Bytecode, max_steps: Option<u64>) -> Result<(), String> {
        let mut steps: u64 = 0;
        while self.instruction_pointer < bytecode.instructions.len() {
            if let Some(limit) = max_steps {
                steps += 1;
                if steps > limit {
                    return Err(format!(
                        "நிரல் {} செயல்முறைகளுக்குப் பிறகும் முடியவில்லை — முடிவில்லாத சுற்று?  \
                         (the program was still running after {} instructions — an endless loop?)",
                        limit, limit
                    ));
                }
            }
            // Borrowed, not cloned. This used to deep-copy the instruction
            // before dispatching it, which for `LoadVar`/`StoreVar` meant a
            // heap allocation for the variable's name on every single access,
            // and for `Push` a full copy of the operand. That was most of the
            // ~75 ns per instruction the benchmarks measured — the dispatch
            // cost the README calls out, and it was not dispatch at all.
            let instruction = &bytecode.instructions[self.instruction_pointer];

            match instruction {
                Instruction::Push(value) => {
                    self.stack.push(value.clone());
                }
                Instruction::Pop => {
                    self.stack.pop();
                }
                Instruction::StoreVar(name) => {
                    if let Some(value) = self.stack.pop() {
                        self.set_var(name, value);
                    }
                }
                Instruction::AppendVar(name) => {
                    let item = self.pop()?;

                    // Inside a function, assigning to a name that is not
                    // already local creates a local — it does not write
                    // through to the global. So an array reached from an outer
                    // scope is copied once, here, and every append after that
                    // is in place on the local.
                    if let Some(frame) = self.frames.last()
                        && !frame.locals.contains_key(name.as_str())
                    {
                        let outer = self.get_var(name).ok_or_else(|| {
                            format!(
                                "அறிவிக்கப்படாத மாறி '{}'  (undefined variable '{}')",
                                name, name
                            )
                        })?;
                        self.set_var(name, outer);
                    }

                    let scope = match self.frames.last_mut() {
                        Some(frame) => &mut frame.locals,
                        None => &mut self.variables,
                    };
                    match scope.get_mut(name.as_str()) {
                        Some(Value::Array(items)) => items.push(item),
                        Some(other) => {
                            return Err(format!(
                                "சேர் ஒரு அணி தேவை  (append needs an array, got {})",
                                Self::type_name(other)
                            ));
                        }
                        None => {
                            return Err(format!(
                                "அறிவிக்கப்படாத மாறி '{}'  (undefined variable '{}')",
                                name, name
                            ));
                        }
                    }
                }
                Instruction::LoadVar(name) => {
                    // An unknown name used to silently load Null, which
                    // to_number() then turned into 0.0 — a typo became a
                    // wrong answer with no diagnostic.
                    //
                    // A name that is no variable but is a function — the
                    // program's own, or a builtin — is that function as a
                    // value: `ச = இரட்டி;`. A variable of the same name wins,
                    // as it does in a call.
                    let value = match self.get_var(name) {
                        Some(value) => value,
                        None if bytecode.functions.contains_key(name.as_str())
                            || crate::vm::is_builtin(name) =>
                        {
                            Value::Function(Box::new(FunctionValue {
                                name: name.clone(),
                                captured: Vec::new(),
                            }))
                        }
                        None => {
                            return Err(format!(
                                "அறிவிக்கப்படாத மாறி '{}'  (undefined variable '{}')",
                                name, name
                            ));
                        }
                    };
                    self.stack.push(value);
                }
                Instruction::Add => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    self.stack
                        .push(Value::Number(left.to_number() + right.to_number()));
                }
                Instruction::Subtract => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    self.stack
                        .push(Value::Number(left.to_number() - right.to_number()));
                }
                Instruction::Multiply => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    self.stack
                        .push(Value::Number(left.to_number() * right.to_number()));
                }
                Instruction::Divide => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let divisor = right.to_number();
                    if divisor == Decimal::ZERO {
                        return Err("பூஜ்ஜியத்தால் வகுத்தல்  (division by zero)".to_string());
                    }
                    // Division stays exact to the decimal type's full
                    // precision. Rounding is deliberately not applied here:
                    // Indian tax computation rounds once at the end, and
                    // rounding every intermediate would compound error in a
                    // chained calculation.
                    self.stack.push(Value::Number(left.to_number() / divisor));
                }
                Instruction::Modulo => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    self.stack
                        .push(Value::Number(left.to_number() % right.to_number()));
                }
                Instruction::Equal => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    self.stack.push(Value::Boolean(left == right));
                }
                Instruction::NotEqual => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    self.stack.push(Value::Boolean(left != right));
                }
                Instruction::LessThan => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = left
                        .partial_cmp(&right)
                        .map(|ord| ord == std::cmp::Ordering::Less)
                        .unwrap_or(false);
                    self.stack.push(Value::Boolean(result));
                }
                Instruction::LessOrEqual => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = left
                        .partial_cmp(&right)
                        .map(|ord| ord != std::cmp::Ordering::Greater)
                        .unwrap_or(false);
                    self.stack.push(Value::Boolean(result));
                }
                Instruction::GreaterThan => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = left
                        .partial_cmp(&right)
                        .map(|ord| ord == std::cmp::Ordering::Greater)
                        .unwrap_or(false);
                    self.stack.push(Value::Boolean(result));
                }
                Instruction::GreaterOrEqual => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = left
                        .partial_cmp(&right)
                        .map(|ord| ord != std::cmp::Ordering::Less)
                        .unwrap_or(false);
                    self.stack.push(Value::Boolean(result));
                }
                Instruction::ConcatVar(name) => {
                    let tail = self.pop()?;

                    // Same scoping rule as AppendVar: inside a function,
                    // assigning to a name that is not already local creates a
                    // local rather than writing through to the global, so a
                    // string reached from an outer scope is copied once here
                    // and appended in place from then on.
                    if let Some(frame) = self.frames.last()
                        && !frame.locals.contains_key(name.as_str())
                    {
                        let outer = self.get_var(name).ok_or_else(|| {
                            format!(
                                "அறிவிக்கப்படாத மாறி '{}'  (undefined variable '{}')",
                                name, name
                            )
                        })?;
                        self.set_var(name, outer);
                    }

                    let scope = match self.frames.last_mut() {
                        Some(frame) => &mut frame.locals,
                        None => &mut self.variables,
                    };
                    match scope.get_mut(name.as_str()) {
                        Some(Value::String(text)) => match &tail {
                            Value::String(more) => text.push_str(more),
                            other => text.push_str(&other.to_string()),
                        },
                        // & joins anything to anything, so a non-string left
                        // side is not an error — it just cannot be appended to
                        // in place. Fall back to building a new value.
                        Some(other) => {
                            let joined = format!("{}{}", other, tail);
                            *other = Value::String(joined);
                        }
                        None => {
                            return Err(format!(
                                "அறிவிக்கப்படாத மாறி '{}'  (undefined variable '{}')",
                                name, name
                            ));
                        }
                    }
                }
                Instruction::Concat => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = format!("{}{}", left, right);
                    self.stack.push(Value::String(result));
                }
                Instruction::Print => {
                    if let Some(value) = self.stack.pop() {
                        host::print_line(&value.to_string());
                    }
                }
                Instruction::Input => {
                    let input = host::read_line()?;
                    self.stack.push(Value::String(input.trim().to_string()));
                }
                Instruction::JumpIfFalse(target) => {
                    if let Some(value) = self.stack.pop()
                        && !value.is_truthy()
                    {
                        self.instruction_pointer = *target;
                        continue;
                    }
                }
                Instruction::Jump(target) => {
                    self.instruction_pointer = *target;
                    continue;
                }
                Instruction::And => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack
                        .push(Value::Boolean(left.is_truthy() && right.is_truthy()));
                }
                Instruction::Or => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack
                        .push(Value::Boolean(left.is_truthy() || right.is_truthy()));
                }
                Instruction::Not => {
                    let value = self.pop()?;
                    self.stack.push(Value::Boolean(!value.is_truthy()));
                }
                Instruction::FileOpen(mode) => {
                    let filename = self.pop()?.to_string();
                    // Opening for writing truncates once; later writes append,
                    // so a sequence of writes reads back in order.
                    if mode == "write" {
                        host::write(&filename, b"").map_err(|e| {
                            format!(
                                "கோப்பு '{}' திறக்க முடியவில்லை  (cannot open '{}' for writing): {}",
                                filename, filename, e
                            )
                        })?;
                    }
                    self.file_modes.insert(filename, mode.clone());
                }
                Instruction::FileClose => {
                    let filename = self.pop()?.to_string();
                    self.file_modes.remove(&filename);
                }
                Instruction::FileWrite => {
                    let data = self.pop()?.to_string();
                    let filename = self.pop()?.to_string();
                    Self::append_line(&filename, &data)?;
                }
                Instruction::FileRead => {
                    let filename = self.pop()?.to_string();
                    let contents = host::read_to_string(&filename).map_err(|e| {
                        format!(
                            "கோப்பு '{}' படிக்க முடியவில்லை  (cannot read '{}'): {}",
                            filename, filename, e
                        )
                    })?;
                    self.stack
                        .push(Value::String(contents.trim_end_matches('\n').to_string()));
                }
                Instruction::ReadCSV => {
                    let filename = self.pop()?.to_string();
                    let contents = host::read_to_string(&filename).map_err(|e| {
                        format!(
                            "கோப்பு '{}' படிக்க முடியவில்லை  (cannot read '{}'): {}",
                            filename, filename, e
                        )
                    })?;
                    // Count data rows, excluding the header line.
                    let rows = contents.lines().filter(|l| !l.trim().is_empty()).count();
                    let data_rows = if rows > 0 { rows - 1 } else { 0 };
                    self.stack.push(Value::Number(Decimal::from(data_rows)));
                }
                Instruction::WriteCSV => {
                    let row = self.pop()?.to_string();
                    let filename = self.pop()?.to_string();
                    Self::append_line(&filename, &row)?;
                }
                Instruction::Nop => {}
                Instruction::Unsupported(what) => {
                    return Err(format!(
                        "{} — இந்த VM இல் இன்னும் செயல்படுத்தப்படவில்லை  (not implemented in the VM yet)",
                        what
                    ));
                }
                Instruction::MakeArray(count) => {
                    let mut items = Vec::with_capacity(*count);
                    for _ in 0..*count {
                        items.push(self.pop()?);
                    }
                    items.reverse(); // pushed left to right
                    self.stack.push(Value::Array(items));
                }
                Instruction::MakeRecord(keys) => {
                    let mut fields = HashMap::with_capacity(keys.len());
                    for key in keys.iter().rev() {
                        let value = self.pop()?;
                        fields.insert(key.clone(), value);
                    }
                    self.stack.push(Value::Map(fields.into()));
                }
                Instruction::Index => {
                    let index = self.pop()?;
                    let base = self.pop()?;
                    self.stack.push(Self::index_of(&base, &index)?);
                }
                Instruction::Field(name) => {
                    let base = self.pop()?;
                    match base {
                        Value::Map(fields) => {
                            let value = fields.get(name.as_str()).cloned().ok_or_else(|| {
                                format!(
                                    "புலம் '{}' இல்லை  (no field '{}' on this record)",
                                    name, name
                                )
                            })?;
                            self.stack.push(value);
                        }
                        other => {
                            return Err(format!(
                                "'{}' ஒரு பொருள் அல்ல  ('.{}' needs a record, got {})",
                                name,
                                name,
                                Self::type_name(&other)
                            ));
                        }
                    }
                }
                Instruction::SetIndex(name) => {
                    let value = self.pop()?;
                    let index = self.pop()?;
                    let mut base = self.get_var(name).ok_or_else(|| {
                        format!(
                            "அறிவிக்கப்படாத மாறி '{}'  (undefined variable '{}')",
                            name, name
                        )
                    })?;
                    Self::index_assign(&mut base, &index, value, name, &bytecode.shapes)?;
                    self.set_var(name, base);
                }
                Instruction::SetField(name, field) => {
                    let value = self.pop()?;
                    let mut base = self.get_var(name).ok_or_else(|| {
                        format!(
                            "அறிவிக்கப்படாத மாறி '{}'  (undefined variable '{}')",
                            name, name
                        )
                    })?;
                    match &mut base {
                        Value::Map(fields) => {
                            crate::vm::shape::check_set(&bytecode.shapes, fields, field, &value)?;
                            fields.insert(field.clone(), value);
                        }
                        other => {
                            return Err(format!(
                                "'{}' ஒரு பொருள் அல்ல  ('{}.{}' needs a record, got {})",
                                name,
                                name,
                                field,
                                Self::type_name(other)
                            ));
                        }
                    }
                    self.set_var(name, base);
                }
                Instruction::Length => {
                    let value = self.pop()?;
                    let n = Self::length_of(&value)?;
                    self.stack.push(Value::Number(Decimal::from(n)));
                }
                Instruction::NthOrKey => {
                    let index = self.pop()?;
                    let base = self.pop()?;
                    let value = Self::nth_or_key(&base, &index)?;
                    self.stack.push(value);
                }
                Instruction::Call(name, argc) => {
                    // A variable holding a function value is called first, so
                    // a parameter can be called by its name. Any other value
                    // under that name leaves the call to the function of that
                    // name, which is what the call always meant before.
                    if let Some(Value::Function(function)) = self.peek_var(name) {
                        let function = (**function).clone();
                        let args = self.pop_args(*argc)?;
                        if !self.call_function(&bytecode, function, args)? {
                            self.instruction_pointer += 1;
                        }
                        continue;
                    }

                    // `கடன்(பதிவு)` — a shape's name called makes a record of
                    // that shape out of a plain one, as a result.
                    if !bytecode.functions.contains_key(name.as_str())
                        && bytecode.shapes.contains_key(name.as_str())
                        && *argc == 1
                    {
                        let record = self.pop()?;
                        let made = crate::vm::shape::convert(&bytecode.shapes, name, record)?;
                        self.stack.push(made);
                        self.instruction_pointer += 1;
                        continue;
                    }

                    // User-defined functions shadow builtins.
                    if !bytecode.functions.contains_key(name.as_str()) {
                        let result = self.call_builtin(name, *argc)?;
                        self.stack.push(result);
                        self.instruction_pointer += 1;
                        continue;
                    }
                    // Borrowed, not cloned. Cloning a FunctionInfo copies its
                    // parameter names — one allocation per parameter, on every
                    // single call — and nothing here needs to own them.
                    let info = bytecode.functions.get(name.as_str()).ok_or_else(|| {
                        format!("அறியப்படாத செயல் '{}'  (unknown function '{}')", name, name)
                    })?;
                    let args = self.pop_args(*argc)?;
                    self.enter(info, name, Vec::new(), args)?;
                    continue;
                }
                Instruction::CallValue(argc) => {
                    let args = self.pop_args(*argc)?;
                    let callee = self.pop()?;
                    let function = match callee {
                        Value::Function(function) => *function,
                        other => {
                            return Err(format!(
                                "இது ஒரு செயல் அல்ல  (this is not a function: it is {})",
                                Self::type_name(&other)
                            ));
                        }
                    };
                    if !self.call_function(&bytecode, function, args)? {
                        self.instruction_pointer += 1;
                    }
                    continue;
                }
                Instruction::CallMethod(name, argc) => {
                    let mut args = self.pop_args(*argc)?;
                    let receiver = self.pop()?;
                    let function =
                        match crate::vm::shape::method_target(&bytecode.shapes, &receiver, name)? {
                            crate::vm::shape::Target::Method(function) => {
                                // The record is the method's இது: its first argument.
                                args.insert(0, receiver);
                                FunctionValue {
                                    name: function,
                                    captured: Vec::new(),
                                }
                            }
                            crate::vm::shape::Target::Held(function) => function,
                        };
                    if !self.call_function(&bytecode, function, args)? {
                        self.instruction_pointer += 1;
                    }
                    continue;
                }
                Instruction::MakeShaped {
                    shape,
                    keys,
                    with_base,
                } => {
                    let values = self.pop_args(keys.len())?;
                    let base = if *with_base { Some(self.pop()?) } else { None };
                    let given = keys.iter().cloned().zip(values).collect();
                    let record = crate::vm::shape::build(&bytecode.shapes, shape, given, base)?;
                    self.stack.push(record);
                }
                Instruction::MakeFunction(name, captures) => {
                    // A local that has not been bound yet is carried as
                    // இன்மை, as the compiled program's zeroed slot is.
                    let captured = captures
                        .iter()
                        .map(|capture| {
                            self.frames
                                .last()
                                .and_then(|frame| frame.locals.get(capture.as_str()))
                                .cloned()
                                .unwrap_or(Value::Null)
                        })
                        .collect();
                    self.stack.push(Value::Function(Box::new(FunctionValue {
                        name: name.clone(),
                        captured,
                    })));
                }
                Instruction::Return => {
                    let value = self.pop()?;
                    let frame = self
                        .frames
                        .pop()
                        .ok_or("செயலுக்கு வெளியே திரும்பு  (return outside of a function)")?;
                    self.stack.truncate(frame.base_len);
                    self.instruction_pointer = frame.return_ip;
                    self.stack.push(value);
                    continue;
                }
                Instruction::TryUnwrap => {
                    let value = self.pop()?;
                    match value {
                        Value::Ok(inner) => self.stack.push(*inner),
                        Value::Err(error) => {
                            // Rust's `?`: hand the failure to the caller.
                            match self.frames.pop() {
                                Some(frame) => {
                                    self.stack.truncate(frame.base_len);
                                    self.instruction_pointer = frame.return_ip;
                                    self.stack.push(Value::Err(error));
                                    continue;
                                }
                                None => {
                                    return Err(format!(
                                        "கையாளப்படாத தவறு: {}  (unhandled error at top level: {})",
                                        error, error
                                    ));
                                }
                            }
                        }
                        other => {
                            return Err(format!(
                                "'?' க்கு ஒரு முடிவு தேவை  ('?' needs a result, got {})",
                                Self::type_name(&other)
                            ));
                        }
                    }
                }
                Instruction::DBConnect(db_type, handle) => {
                    let connection = self.pop()?.to_string();

                    // Connecting again through the same driver used to replace
                    // the open connection without saying so. The map is keyed
                    // by driver, so the second insert overwrote the first, the
                    // count stayed at one, and connection_mut — which does
                    // refuse when several are open — never saw a reason to.
                    // Every query after that went to the second database while
                    // the program still believed it was talking to the first.
                    //
                    // Two different databases through one driver is the case
                    // the language cannot express: தளம்_வினா names no handle,
                    // so there would be no way to say which one a query meant.
                    // Refusing is the honest answer until it can.
                    if let Some(already) = self.connections.connection_of(handle) {
                        if already != connection {
                            return Err(format!(
                                "'{}' ஏற்கனவே '{}' உடன் இணைக்கப்பட்டுள்ளது  \
                                 ('{}' is already connected to '{}'): \
                                 தளம்_பிரி it first, or give this one its own name",
                                handle, already, handle, already
                            ));
                        }
                        // The same database again: already connected, so there
                        // is nothing to do and no lease to take.
                        self.instruction_pointer += 1;
                        continue;
                    }

                    // Borrowed rather than opened: under --server every request
                    // runs on a fresh VM, so this statement is reached once per
                    // request and used to mean a new connection each time.
                    let lease = crate::db::pool::checkout(db_type, &connection)?;
                    self.connections.insert(handle.clone(), connection, lease);
                }
                // The operand is a handle, which for an unnamed connection is
                // its driver name — so `தளம்_பிரி SQL` still means what it did.
                Instruction::DBDisconnect(db_type) => {
                    // Returns the connection to the cache rather than closing
                    // it. தளம்_பிரி means "I am done with this", which is what
                    // a program actually wants to say; keeping the socket open
                    // for the next request is the host's business.
                    match self.connections.remove(db_type) {
                        Some(lease) => drop(lease),
                        None => {
                            return Err(format!(
                                "'{}' இணைக்கப்படவில்லை  (not connected to {})",
                                db_type, db_type
                            ));
                        }
                    }
                }
                Instruction::DBExecute(named) => {
                    let params = crate::db::params_from(&self.pop()?)?;
                    let sql = self.pop()?.to_string();
                    let connection = self.connection_for(named.as_deref())?;
                    connection.execute(&sql, &params)?;
                }
                Instruction::DBQuery(named) => {
                    let params = crate::db::params_from(&self.pop()?)?;
                    let sql = self.pop()?.to_string();
                    let connection = self.connection_for(named.as_deref())?;
                    // One record per row, so a result set is an array of
                    // records — a table in the language's own terms.
                    let rows = connection.query(&sql, &params)?;
                    self.stack.push(Value::Array(rows));
                }
                Instruction::SendResponse => {
                    let headers = self.pop()?;
                    let body = self.pop()?;
                    let status = self.pop()?;
                    // Written to globals, not the current frame: the server
                    // reads them from the VM once the handler has returned,
                    // and பதில் is often called from inside a function.
                    self.variables.insert("response_status".to_string(), status);
                    self.variables
                        .insert("response_body".to_string(), Value::String(body.to_string()));
                    self.variables
                        .insert("response_headers".to_string(), headers);
                }
                // ஜேசான்_உரை தரவு, நிலை; — பதில் with the JSON content type
                // already on it, which is the whole of what it saves you.
                //
                // The body must already be text. Rendering a record here would
                // need a second JSON encoder in Rust beside nUlakam's
                // ஜேசான்_ஆக்கு, and two encoders are two answers to the same
                // question — so this asks for the one that exists rather than
                // quietly emitting eTamil's record syntax and calling it JSON.
                Instruction::SendJSON => {
                    let data = self.pop()?;
                    let status = self.pop()?;
                    let body = match data {
                        Value::String(text) => text,
                        other => {
                            return Err(format!(
                                "ஜேசான்_உரை க்கு உரை தேவை — ஜேசான்_ஆக்கு() ஐப் பயன்படுத்துங்கள்  \
                                 (json response needs text, got {}: encode it with ஜேசான்_ஆக்கு first)",
                                Self::type_name(&other)
                            ));
                        }
                    };

                    let mut headers = HashMap::new();
                    headers.insert(
                        "Content-Type".to_string(),
                        Value::String("application/json".to_string()),
                    );

                    // The same three globals பதில் writes, for the same reason:
                    // the server reads them after the handler has returned.
                    self.variables.insert("response_status".to_string(), status);
                    self.variables
                        .insert("response_body".to_string(), Value::String(body));
                    self.variables
                        .insert("response_headers".to_string(), Value::Map(headers.into()));
                }
                Instruction::DefineRoute(_, _) | Instruction::StartServer(_, _) => {
                    return Err(
                        "வழங்கி செயல்பாடுகள் VM இல் இன்னும் இல்லை  (server operations are not implemented in the VM yet)"
                            .to_string(),
                    );
                }
                Instruction::Halt => {
                    break;
                }
            }

            self.instruction_pointer += 1;
        }

        Ok(())
    }
}
