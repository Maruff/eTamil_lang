// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! The runtime the LLVM backend's emitted IR calls into.
//!
//! ## Why this exists
//!
//! The backend used to compute in `f64`, then in `i64`. Both are the same
//! bargain in different clothes: the IR holds the value directly, so the
//! language shrinks to whatever fits in a register. Numbers fit. Strings,
//! arrays, records, results and `இன்மை` do not, and neither does a decimal —
//! `1 / 3` is `0.3333333333333333333333333333` on the VM and there is no
//! register that holds that.
//!
//! So the IR stops holding values. Every eTamil value is an `i64` **handle**
//! into an arena that lives here, and every operation on one is a call into
//! this module. That is the ordinary way to compile a dynamically typed
//! language, and it buys three things that matter more than the call overhead:
//!
//! 1. **Decimals are exact, because this is the same `rust_decimal` the VM
//!    uses.** Not a fixed two-place approximation — the actual type. `0.1 + 0.2`
//!    is `0.3` and `1 / 3` prints all twenty-eight digits, in both backends,
//!    for the same reason rather than by coincidence.
//! 2. **Formatting cannot drift**, because printing calls `Value::to_string`,
//!    the function the VM prints through. Trailing zeros are trimmed in one
//!    place, not two.
//! 3. **Every builtin works at once**, because `etamil_call` goes
//!    through `VM::invoke_builtin` — the dispatch the bytecode interpreter
//!    uses. `நீளம்` is not reimplemented here, so it cannot disagree.
//!
//! ## The cost
//!
//! `output.ll` is no longer self-contained. It has to be linked against this:
//!
//! ```text
//! clang output.ll -o prog -L target/release -letamil_compiler \
//!       -Wl,-rpath,target/release
//! ```
//!
//! `Cargo.toml` already builds the `cdylib`, so nothing new is produced; these
//! functions are simply exported from it.
//!
//! ## Handles
//!
//! A handle is an index into a thread-local arena. Handle 0 is always `இன்மை`.
//! Nothing is ever freed: a compiled program runs and exits, and an arena that
//! only grows cannot dangle, cannot double-free, and needs no reference counts
//! in the emitted IR. A long-running program compiled this way would grow
//! without bound, which is the honest limitation of the approach and the reason
//! the VM remains the way to run a server.
//!
//! Every function here is `extern "C"` and takes and returns handles or plain
//! integers, so `codegen.rs` needs no knowledge of `Value`'s layout.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::vm::VM;
use crate::vm::shape::{self, Shapes, Target};
use crate::vm::value::{FunctionValue, Value};

/// How a compiled function is entered when it is called as a value: its
/// arguments as one array of handles, captures first. `codegen.rs` emits one of
/// these beside every function, because a call through a value cannot know the
/// function's arity at compile time and the C ABI has no other way to say
/// "some number of handles".
pub type Entry = unsafe extern "C" fn(*const i64) -> i64;

/// A compiled function, as the program registered it at startup.
#[derive(Clone, Copy)]
struct Compiled {
    entry: Entry,
    /// Handles taken from the function value before the arguments.
    captures: usize,
    /// Arguments the caller must supply.
    params: usize,
}

thread_local! {
    /// Index 0 is `இன்மை`, so a zeroed handle is a valid value rather than a
    /// wild index.
    static ARENA: RefCell<Vec<Value>> = RefCell::new(vec![Value::Null]);

    /// Kept only for its builtin dispatch and its stack. Nothing here executes
    /// bytecode; the compiled program is the program.
    static HOST: RefCell<VM> = RefCell::new(VM::new());

    /// Every function in the program, by the name a function value carries.
    /// The VM's equivalent is `Bytecode::functions`.
    static FUNCTIONS: RefCell<HashMap<String, Compiled>> = RefCell::new(HashMap::new());

    /// Every வடிவம் the program declares, registered at startup. The VM's
    /// equivalent is `Bytecode::shapes`, and both are read by `vm::shape`.
    static SHAPES: RefCell<Shapes> = RefCell::new(Shapes::new());
}

/// A runtime error, reported the way the VM reports one and with the same
/// exit status, because the two backends failing differently on the same
/// program is its own kind of disagreement.
fn fail(message: &str) -> ! {
    eprintln!("✗ Runtime error: {}", message);
    std::process::exit(1);
}

fn put(value: Value) -> i64 {
    ARENA.with(|arena| {
        let mut arena = arena.borrow_mut();
        arena.push(value);
        (arena.len() - 1) as i64
    })
}

/// Cloned out rather than borrowed, so that a caller can hold two values at
/// once without holding two borrows of the arena.
fn get(handle: i64) -> Value {
    ARENA.with(|arena| {
        let arena = arena.borrow();
        match arena.get(handle.max(0) as usize) {
            Some(value) => value.clone(),
            None => fail("இயக்க நேரப் பிழை: செல்லாத மதிப்பு  (invalid runtime handle)"),
        }
    })
}

/// # Safety
///
/// `text` must be a NUL-terminated string the emitted IR keeps alive, which a
/// global string constant is.
unsafe fn borrow_str(text: *const c_char) -> String {
    if text.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(text) }
        .to_string_lossy()
        .into_owned()
}

// --- Constructors ---------------------------------------------------------

/// A number, from the decimal's own text. Going through the string is what
/// makes it exact: `codegen.rs` has a `Decimal` at compile time and writes it
/// out, and `Decimal::from_str` reads back the identical value. Passing a
/// double here would lose the thing this whole module exists to keep.
///
/// # Safety
///
/// `text` must be a valid, NUL-terminated C string that stays alive for
/// the call. The generated code passes a pointer to a constant in its own
/// module, which satisfies both.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_number(text: *const c_char) -> i64 {
    let text = unsafe { borrow_str(text) };
    match text.parse::<rust_decimal::Decimal>() {
        Ok(number) => put(Value::Number(number)),
        Err(_) => fail(&format!(
            "'{}' ஒரு எண் அல்ல  ('{}' is not a number)",
            text, text
        )),
    }
}

/// # Safety
///
/// `text` must be a valid, NUL-terminated C string that stays alive for
/// the call. The generated code passes a pointer to a constant in its own
/// module, which satisfies both.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_text(text: *const c_char) -> i64 {
    let text = unsafe { borrow_str(text) };
    put(Value::String(text))
}

#[unsafe(no_mangle)]
pub extern "C" fn etamil_boolean(flag: i32) -> i64 {
    put(Value::Boolean(flag != 0))
}

#[unsafe(no_mangle)]
pub extern "C" fn etamil_nil() -> i64 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn etamil_array() -> i64 {
    put(Value::Array(Vec::new()))
}

/// Grows an array in place. Used to build a literal, where the array is fresh
/// and nothing else refers to it yet.
#[unsafe(no_mangle)]
pub extern "C" fn etamil_array_push(array: i64, value: i64) {
    let value = get(value);
    ARENA.with(|arena| {
        let mut arena = arena.borrow_mut();
        match arena.get_mut(array.max(0) as usize) {
            Some(Value::Array(items)) => items.push(value),
            _ => fail("அணி எதிர்பார்க்கப்பட்டது  (expected an array)"),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn etamil_record() -> i64 {
    put(Value::Map(std::collections::HashMap::new().into()))
}

/// Put a field on a record.
///
/// # Safety
///
/// `key` must be a valid, NUL-terminated C string that stays alive for
/// the call. The generated code passes a pointer to a constant in its own
/// module, which satisfies both.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_record_put(record: i64, key: *const c_char, value: i64) {
    let key = unsafe { borrow_str(key) };
    let value = get(value);
    ARENA.with(|arena| {
        let mut arena = arena.borrow_mut();
        match arena.get_mut(record.max(0) as usize) {
            Some(Value::Map(fields)) => {
                fields.insert(key, value);
            }
            _ => fail("பொருள் எதிர்பார்க்கப்பட்டது  (expected a record)"),
        }
    })
}

// --- Arithmetic -----------------------------------------------------------
//
// `to_number` is `Value`'s own coercion, so a string that looks like a number
// behaves here exactly as it does on the VM.

#[unsafe(no_mangle)]
pub extern "C" fn etamil_add(left: i64, right: i64) -> i64 {
    put(Value::Number(
        get(left).to_number() + get(right).to_number(),
    ))
}

#[unsafe(no_mangle)]
pub extern "C" fn etamil_subtract(left: i64, right: i64) -> i64 {
    put(Value::Number(
        get(left).to_number() - get(right).to_number(),
    ))
}

#[unsafe(no_mangle)]
pub extern "C" fn etamil_multiply(left: i64, right: i64) -> i64 {
    put(Value::Number(
        get(left).to_number() * get(right).to_number(),
    ))
}

/// Exact to the decimal type's full precision, and rounding is deliberately
/// not applied — the VM's `Divide` says the same, because Indian tax
/// computation rounds once at the end and rounding intermediates compounds.
#[unsafe(no_mangle)]
pub extern "C" fn etamil_divide(left: i64, right: i64) -> i64 {
    let divisor = get(right).to_number();
    if divisor.is_zero() {
        fail("பூஜ்ஜியத்தால் வகுத்தல்  (division by zero)");
    }
    put(Value::Number(get(left).to_number() / divisor))
}

/// `&`. The VM builds `format!("{}{}", ...)` over `to_string`, so this is that.
#[unsafe(no_mangle)]
pub extern "C" fn etamil_concat(left: i64, right: i64) -> i64 {
    let joined = format!("{}{}", get(left), get(right));
    put(Value::String(joined))
}

// --- Comparison and logic -------------------------------------------------

/// Operator codes, fixed here and mirrored in `codegen.rs`. Kept as small
/// integers rather than strings so the IR passes a constant.
pub const COMPARE_LT: i32 = 0;
pub const COMPARE_LE: i32 = 1;
pub const COMPARE_GT: i32 = 2;
pub const COMPARE_GE: i32 = 3;
pub const COMPARE_EQ: i32 = 4;
pub const COMPARE_NE: i32 = 5;

#[unsafe(no_mangle)]
pub extern "C" fn etamil_compare(left: i64, right: i64, operator: i32) -> i64 {
    let left = get(left);
    let right = get(right);

    // Equality is `Value`'s own, which compares numbers exactly and knows how
    // to compare arrays and records. Ordering goes through `partial_cmp`, and
    // two values with no ordering between them answer false rather than
    // guessing — the same thing the VM's LessThan does.
    let answer = match operator {
        COMPARE_EQ => left == right,
        COMPARE_NE => left != right,
        _ => match left.partial_cmp(&right) {
            Some(ordering) => match operator {
                COMPARE_LT => ordering == std::cmp::Ordering::Less,
                COMPARE_LE => ordering != std::cmp::Ordering::Greater,
                COMPARE_GT => ordering == std::cmp::Ordering::Greater,
                COMPARE_GE => ordering != std::cmp::Ordering::Less,
                _ => false,
            },
            None => false,
        },
    };
    put(Value::Boolean(answer))
}

#[unsafe(no_mangle)]
pub extern "C" fn etamil_and(left: i64, right: i64) -> i64 {
    put(Value::Boolean(
        get(left).is_truthy() && get(right).is_truthy(),
    ))
}

#[unsafe(no_mangle)]
pub extern "C" fn etamil_or(left: i64, right: i64) -> i64 {
    put(Value::Boolean(
        get(left).is_truthy() || get(right).is_truthy(),
    ))
}

#[unsafe(no_mangle)]
pub extern "C" fn etamil_not(value: i64) -> i64 {
    put(Value::Boolean(!get(value).is_truthy()))
}

/// The one place a handle becomes something the IR can branch on.
#[unsafe(no_mangle)]
pub extern "C" fn etamil_truthy(value: i64) -> i32 {
    i32::from(get(value).is_truthy())
}

/// For `?`. The IR tests this and returns the value unchanged when it is a
/// `தவறு`, which is what propagating an error out of a function means.
#[unsafe(no_mangle)]
pub extern "C" fn etamil_is_error(value: i64) -> i32 {
    i32::from(matches!(get(value), Value::Err(_)))
}

/// `?` on a success unwraps it; the IR only calls this once `etamil_is_error`
/// has said no, and handles the `தவறு` case itself by returning it.
///
/// A value that is not a result at all is a runtime error, in the VM's own
/// words. Answering the value unchanged would be friendlier and wrong: `?` on
/// a plain number means the author thinks it is a result.
#[unsafe(no_mangle)]
pub extern "C" fn etamil_unwrap(value: i64) -> i64 {
    match get(value) {
        Value::Ok(inner) => put(*inner),
        Value::Err(error) => put(Value::Err(error)),
        other => fail(&format!(
            "'?' க்கு ஒரு முடிவு தேவை  ('?' needs a result, got {})",
            match other {
                Value::Number(_) => "number",
                Value::String(_) => "string",
                Value::Boolean(_) => "boolean",
                Value::Array(_) => "array",
                Value::Map(_) => "record",
                Value::Null => "nil",
                _ => "value",
            }
        )),
    }
}

/// `?` at the top level, where there is no caller to hand the failure to. The
/// VM stops with this; so does a compiled program.
#[unsafe(no_mangle)]
pub extern "C" fn etamil_unhandled(value: i64) -> ! {
    let shown = get(value).to_string();
    fail(&format!(
        "கையாளப்படாத தவறு: {}  (unhandled error at top level: {})",
        shown, shown
    ))
}

// --- Access ---------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn etamil_index(base: i64, index: i64) -> i64 {
    match VM::index_of(&get(base), &get(index)) {
        Ok(value) => put(value),
        Err(why) => fail(&why),
    }
}

/// Indexed assignment: `a[0] = x` on an array, `r[k] = x` on a record.
///
/// Answers a **new** handle, which the IR stores back into the variable. This
/// used to change the value in place, and a handle is shared by every name it
/// was assigned to: after `நகல் = அசல்; நகல்[0] = 9;` the compiled program's
/// `அசல்` had changed too, and so had a caller's array after a function set an
/// element of its parameter. The VM copies — a name holds a value, not a
/// reference to one — and `நிலை`'s promise that a copy cannot reach the
/// original rests on that. The parity job found it in examples/language/nilY.qmz.
///
/// # Safety
///
/// `name` must be a valid, NUL-terminated C string that stays alive for
/// the call. The generated code passes a pointer to a constant in its own
/// module, which satisfies both.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_index_set(
    base: i64,
    index: i64,
    value: i64,
    name: *const c_char,
) -> i64 {
    let name = unsafe { borrow_str(name) };
    let index = get(index);
    let value = get(value);
    let mut updated = get(base);
    let done = SHAPES
        .with(|shapes| VM::index_assign(&mut updated, &index, value, &name, &shapes.borrow()));
    match done {
        Ok(()) => put(updated),
        Err(why) => fail(&why),
    }
}

/// # Safety
///
/// `key` must be a valid, NUL-terminated C string that stays alive for
/// the call. The generated code passes a pointer to a constant in its own
/// module, which satisfies both.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_field(base: i64, key: *const c_char) -> i64 {
    let key = unsafe { borrow_str(key) };
    match get(base) {
        Value::Map(fields) => match fields.get(&key) {
            Some(value) => put(value.clone()),
            None => fail(&format!(
                "புலம் '{}' இல்லை  (no field '{}' on this record)",
                key, key
            )),
        },
        _ => fail(&format!(
            "'{}' ஒரு பொருள் அல்ல  ('.{}' needs a record)",
            key, key
        )),
    }
}

/// `r.f = x`. A new handle, for the reason `etamil_index_set` gives one.
///
/// # Safety
///
/// `key` must be a valid, NUL-terminated C string that stays alive for
/// the call. The generated code passes a pointer to a constant in its own
/// module, which satisfies both.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_field_set(base: i64, key: *const c_char, value: i64) -> i64 {
    let key = unsafe { borrow_str(key) };
    let value = get(value);
    match get(base) {
        Value::Map(mut fields) => {
            // A shaped record takes only its own fields — the check the VM's
            // SetField makes, through the same function.
            let checked =
                SHAPES.with(|shapes| shape::check_set(&shapes.borrow(), &fields, &key, &value));
            if let Err(why) = checked {
                fail(&why);
            }
            fields.insert(key, value);
            put(Value::Map(fields))
        }
        _ => fail("பொருள் எதிர்பார்க்கப்பட்டது  (expected a record)"),
    }
}

/// How many times a `ஒவ்வொரு` goes round. Separate from the `நீளம்` builtin
/// only because the loop needs a plain integer to compare against rather than
/// a handle — the rule for *what* it counts is the VM's.
///
/// That rule is not the obvious one. A string counts its Tamil letters, and a
/// letter is a cluster, so it is not the same number as its chars.
#[unsafe(no_mangle)]
pub extern "C" fn etamil_count(value: i64) -> i64 {
    match VM::length_of(&get(value)) {
        Ok(count) => count as i64,
        Err(why) => fail(&why),
    }
}

/// What a `ஒவ்வொரு` binds on each turn, through the VM's own `nth_or_key`.
///
/// Not indexing. A record yields its **keys**, sorted, and a string yields one
/// letter — indexing a record with a number would fail instead, which is what
/// this backend did before the rule was read out of the interpreter rather than
/// guessed at.
#[unsafe(no_mangle)]
pub extern "C" fn etamil_nth_or_key(base: i64, position: i64) -> i64 {
    let index = Value::Number(rust_decimal::Decimal::from(position));
    match VM::nth_or_key(&get(base), &index) {
        Ok(value) => put(value),
        Err(why) => fail(&why),
    }
}

// --- Functions as values --------------------------------------------------
//
// The emitted IR calls a function it can name directly, as it always has. It
// comes here only when what is being called is a *value*: a parameter holding
// a function, an array element, `f(1)(2)`. Then the name is not known until the
// program runs, so the registry below answers it — exactly the lookup the VM's
// `Call` makes in `Bytecode::functions`, with builtins behind it in the same
// order.

/// The handles an array of `count` holds, read before anything else borrows
/// the arena.
///
/// # Safety
///
/// `argv` must point to `count` handles, or be null with `count` zero.
unsafe fn handles(argv: *const i64, count: i64) -> Vec<i64> {
    let count = count.max(0) as usize;
    (0..count)
        .map(|position| {
            if argv.is_null() {
                0
            } else {
                unsafe { *argv.add(position) }
            }
        })
        .collect()
}

/// Called once per function when the program starts, before anything else.
///
/// # Safety
///
/// `name` must be a valid, NUL-terminated C string that stays alive for the
/// call, and `entry` a function taking `captures + params` handles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_register_function(
    name: *const c_char,
    entry: Entry,
    captures: i64,
    params: i64,
) {
    let name = unsafe { borrow_str(name) };
    let compiled = Compiled {
        entry,
        captures: captures.max(0) as usize,
        params: params.max(0) as usize,
    };
    FUNCTIONS.with(|functions| functions.borrow_mut().insert(name, compiled));
}

/// A function value: the function's name, and the values it carries.
///
/// `ச = இரட்டி;` carries nothing. `செயல்(x) { … }` inside a function carries
/// the locals it reads, loaded by the IR from their slots at this moment — the
/// same copies the VM's `MakeFunction` takes from its frame.
///
/// # Safety
///
/// `name` must be a valid, NUL-terminated C string that stays alive for the
/// call, and `argv` must point to `count` handles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_function(name: *const c_char, argv: *const i64, count: i64) -> i64 {
    let name = unsafe { borrow_str(name) };
    let captured = unsafe { handles(argv, count) }
        .into_iter()
        .map(get)
        .collect();
    put(Value::Function(Box::new(FunctionValue { name, captured })))
}

/// Run a function value against these argument handles.
fn call(function: FunctionValue, args: Vec<i64>) -> i64 {
    let compiled = FUNCTIONS.with(|functions| functions.borrow().get(&function.name).copied());

    if let Some(compiled) = compiled {
        if args.len() != compiled.params {
            let shown = FunctionValue::shown(&function.name);
            fail(&format!(
                "செயல் {} {} அளவுருக்களை எதிர்பார்க்கிறது, {} வழங்கப்பட்டது  \
                 (function {} expects {} argument(s), got {})",
                shown,
                compiled.params,
                args.len(),
                shown,
                compiled.params,
                args.len()
            ));
        }
        // Captures first, then arguments: the order the function's own
        // parameters were declared in by codegen.rs. A missing capture is
        // இன்மை, as it is in the VM's frame.
        let mut carried = function.captured.into_iter();
        let mut argv: Vec<i64> = (0..compiled.captures)
            .map(|_| carried.next().map(put).unwrap_or(0))
            .collect();
        argv.extend(args);
        // No borrow of the arena or the registry is held here: the function
        // being entered will want both.
        return unsafe { (compiled.entry)(argv.as_ptr()) };
    }

    if crate::vm::is_builtin(&function.name) {
        let arguments: Vec<Value> = args.into_iter().map(get).collect();
        let answer = HOST.with(|host| host.borrow_mut().invoke_builtin(&function.name, arguments));
        return match answer {
            Ok(value) => put(value),
            Err(why) => fail(&why),
        };
    }

    let shown = FunctionValue::shown(&function.name);
    fail(&format!(
        "அறியப்படாத செயல் {}  (unknown function {})",
        shown, shown
    ))
}

/// `f(x)` where `f` is not a name but a value: `விதிகள்[0](x)`, `f(1)(2)`.
///
/// # Safety
///
/// `argv` must point to `argc` handles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_call_value(callee: i64, argv: *const i64, argc: i64) -> i64 {
    let args = unsafe { handles(argv, argc) };
    match get(callee) {
        Value::Function(function) => call(*function, args),
        other => fail(&format!(
            "இது ஒரு செயல் அல்ல  (this is not a function: it is {})",
            VM::type_name(&other)
        )),
    }
}

/// `f(x)` where `f` is also a variable. What the VM's `Call` does: a variable
/// holding a function value is called, and any other value leaves the call to
/// the function of that name.
///
/// # Safety
///
/// `name` must be a valid, NUL-terminated C string that stays alive for the
/// call, and `argv` must point to `argc` handles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_call_named(
    held: i64,
    name: *const c_char,
    argv: *const i64,
    argc: i64,
) -> i64 {
    let args = unsafe { handles(argv, argc) };
    match get(held) {
        Value::Function(function) => call(*function, args),
        _ => {
            let name = unsafe { borrow_str(name) };
            call(
                FunctionValue {
                    name,
                    captured: Vec::new(),
                },
                args,
            )
        }
    }
}

// --- Shapes ---------------------------------------------------------------
//
// Registered at startup, a field at a time, because a declared type crosses
// the ABI as a code and a shape name rather than as a value. The codes are
// `vm::shape::type_code`'s, so both sides of the boundary number them alike.

/// # Safety
///
/// `name` must be a valid, NUL-terminated C string that stays alive for the
/// call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_shape_begin(name: *const c_char) {
    let name = unsafe { borrow_str(name) };
    SHAPES.with(|shapes| {
        shapes.borrow_mut().insert(
            name.clone(),
            shape::Shape {
                name,
                ..Default::default()
            },
        )
    });
}

/// One field, with its declared type's code and — for a field that is itself
/// a shape — that shape's name. `of` may be null otherwise.
///
/// # Safety
///
/// `owner` and `field` must be valid, NUL-terminated C strings, and `of`
/// one too or null, all alive for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_shape_field(
    owner: *const c_char,
    field: *const c_char,
    code: i32,
    of: *const c_char,
) {
    let owner = unsafe { borrow_str(owner) };
    let field = unsafe { borrow_str(field) };
    let declared = shape::from_code(code, unsafe { borrow_str(of) });
    SHAPES.with(|shapes| {
        if let Some(entry) = shapes.borrow_mut().get_mut(&owner) {
            entry.fields.push((field, declared));
        }
    });
}

/// One method, and whether it takes இது.
///
/// # Safety
///
/// `owner` and `method` must be valid, NUL-terminated C strings that stay
/// alive for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_shape_method(
    owner: *const c_char,
    method: *const c_char,
    takes_self: i32,
) {
    let owner = unsafe { borrow_str(owner) };
    let method = unsafe { borrow_str(method) };
    SHAPES.with(|shapes| {
        if let Some(entry) = shapes.borrow_mut().get_mut(&owner) {
            entry.methods.insert(method, takes_self != 0);
        }
    });
}

/// `வடிவம்{…}`: the fields were put on a fresh plain record by the IR, and
/// this checks them against the shape and makes the record that shape.
/// `base` is the `..` record when `has_base` says there is one.
///
/// # Safety
///
/// `name` must be a valid, NUL-terminated C string that stays alive for the
/// call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_shape_make(
    record: i64,
    name: *const c_char,
    base: i64,
    has_base: i32,
) -> i64 {
    let name = unsafe { borrow_str(name) };
    let given: Vec<(String, Value)> = match get(record) {
        Value::Map(fields) => fields.fields.into_iter().collect(),
        _ => fail("பொருள் எதிர்பார்க்கப்பட்டது  (expected a record)"),
    };
    let base = (has_base != 0).then(|| get(base));
    let made = SHAPES.with(|shapes| shape::build(&shapes.borrow(), &name, given, base));
    match made {
        Ok(value) => put(value),
        Err(why) => fail(&why),
    }
}

/// `கடன்(பதிவு)`: a plain record checked and made into the shape, as a result.
///
/// # Safety
///
/// `name` must be a valid, NUL-terminated C string that stays alive for the
/// call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_shape_convert(name: *const c_char, value: i64) -> i64 {
    let name = unsafe { borrow_str(name) };
    let value = get(value);
    let made = SHAPES.with(|shapes| shape::convert(&shapes.borrow(), &name, value));
    match made {
        Ok(value) => put(value),
        Err(why) => fail(&why),
    }
}

/// `r.m(…)`: the method of r's shape with r as its இது, or a function r holds
/// in that field — decided by `vm::shape::method_target`, as the VM decides it.
///
/// # Safety
///
/// `name` must be a valid, NUL-terminated C string that stays alive for the
/// call, and `argv` must point to `argc` handles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_call_method(
    receiver: i64,
    name: *const c_char,
    argv: *const i64,
    argc: i64,
) -> i64 {
    let name = unsafe { borrow_str(name) };
    let mut args = unsafe { handles(argv, argc) };
    let target =
        SHAPES.with(|shapes| shape::method_target(&shapes.borrow(), &get(receiver), &name));
    match target {
        Ok(Target::Method(function)) => {
            args.insert(0, receiver);
            call(
                FunctionValue {
                    name: function,
                    captured: Vec::new(),
                },
                args,
            )
        }
        Ok(Target::Held(function)) => call(function, args),
        Err(why) => fail(&why),
    }
}

// --- Builtins and output --------------------------------------------------

/// Any builtin, by name, through the interpreter's own dispatch.
///
/// # Safety
///
/// `argv` must point to `argc` handles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_call(name: *const c_char, argv: *const i64, argc: i64) -> i64 {
    let name = unsafe { borrow_str(name) };
    let count = argc.max(0) as usize;

    let mut arguments = Vec::with_capacity(count);
    for position in 0..count {
        // Read the handle, then resolve it: `get` borrows the arena, and doing
        // both in one expression would hold that borrow across the loop.
        let handle = if argv.is_null() {
            0
        } else {
            unsafe { *argv.add(position) }
        };
        arguments.push(get(handle));
    }

    let answer = HOST.with(|host| host.borrow_mut().invoke_builtin(&name, arguments));
    match answer {
        Ok(value) => put(value),
        Err(why) => fail(&why),
    }
}

/// `அச்சு`. One `to_string` and one newline, the same call the VM's `Print`
/// makes — which is why `&` between a label and a number comes out identically
/// without the backend having to know anything about text.
#[unsafe(no_mangle)]
pub extern "C" fn etamil_print(value: i64) {
    println!("{}", get(value));
}

/// `உள்ளிடு`. A line of input, as text, exactly as the VM's `Input` leaves it.
#[unsafe(no_mangle)]
pub extern "C" fn etamil_read_line() -> i64 {
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(_) => put(Value::String(line.trim().to_string())),
        Err(why) => fail(&format!(
            "உள்ளிட முடியவில்லை  (could not read input: {})",
            why
        )),
    }
}

/// `தளம்_இணை`. Borrow a connection and file it under `handle`.
///
/// `handle` is never null: an unnamed connection takes the driver's name, and
/// `codegen.rs` applies that default at compile time exactly as the bytecode
/// compiler does — so both backends key the registry the same way and
/// `தளம்_வினா` with no name finds the same connection under either.
///
/// Borrowed rather than opened, through the same pool the VM uses. Under
/// `--server` every request runs on a fresh VM, so this statement is reached
/// once per request and opening each time would cost a connect, a handshake
/// and an authentication round trip per request.
///
/// Pointing one handle at a **second** database is refused, and that refusal is
/// the VM's, word for word. The map is keyed by handle, so a second insert used
/// to overwrite the first silently: the count stayed at one, nothing looked
/// wrong, and every query after it went to the second database while the
/// program still believed it was talking to the first.
///
/// # Safety
///
/// `db_type` and `handle` must be valid, NUL-terminated C strings that stay
/// alive for the call. The generated code passes pointers to constants in its
/// own module, which satisfies both.
#[cfg(not(target_family = "wasm"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_db_connect(
    db_type: *const c_char,
    connection: i64,
    handle: *const c_char,
) {
    let db_type = unsafe { borrow_str(db_type) };
    let handle = unsafe { borrow_str(handle) };
    let connection = get(connection).to_string();

    let already = HOST.with(|host| {
        host.borrow()
            .connections
            .connection_of(&handle)
            .map(str::to_string)
    });
    if let Some(already) = already {
        if already != connection {
            fail(&format!(
                "'{}' ஏற்கனவே '{}' உடன் இணைக்கப்பட்டுள்ளது  \
                 ('{}' is already connected to '{}'): \
                 தளம்_பிரி it first, or give this one its own name",
                handle, already, handle, already
            ));
        }
        // The same database again: already connected, nothing to do, and no
        // second lease to take.
        return;
    }

    let lease = match crate::db::pool::checkout(&db_type, &connection) {
        Ok(lease) => lease,
        Err(why) => fail(&why),
    };
    HOST.with(|host| {
        host.borrow_mut()
            .connections
            .insert(handle, connection, lease)
    });
}

/// `தளம்_செய்`. A statement that returns no rows.
///
/// The row count goes nowhere, which is the VM's behaviour and not an
/// oversight here: `தளம்_செய்` is the statement form and the count is reachable
/// through `தளம்_செய்_முயற்சி`, which returns a result.
///
/// # Safety
///
/// `handle` must be null, or a valid NUL-terminated C string that stays alive
/// for the call. Null is how the generated code says "the only connection
/// open"; anything else is a pointer to a constant in its own module.
#[cfg(not(target_family = "wasm"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_db_execute(sql: i64, params: i64, handle: *const c_char) {
    let named = if handle.is_null() {
        None
    } else {
        Some(unsafe { borrow_str(handle) })
    };

    let bound = match crate::db::params_from(&get(params)) {
        Ok(bound) => bound,
        Err(why) => fail(&why),
    };
    let sql = get(sql).to_string();

    let done = HOST.with(|host| -> Result<i64, String> {
        let mut host = host.borrow_mut();
        host.connection_for(named.as_deref())?.execute(&sql, &bound)
    });
    if let Err(why) = done {
        fail(&why);
    }
}

/// `தளம்_வினா`. One record per row, so a result set is an array of records —
/// a table in the language's own terms, and the same `Value::Array` the VM's
/// `DBQuery` leaves on its stack.
///
/// `handle` is the connection's name, or null for "the only one open". That is
/// how `Option<&str>` crosses the ABI, and it is the same `None` the parser
/// puts in the AST when the author did not name one.
///
/// The registry it asks is the VM's own, on the same `HOST` the builtins
/// dispatch through. Two registries would agree until somebody edited one of
/// them, and "which connection is the only one open" is exactly the kind of
/// question two backends must not answer differently.
///
/// **Nothing in a compiled program opens a connection yet** — `தரவுசேமி_இணை`
/// is still refused by the backend — so today this can only report that there
/// is none. That report is the VM's own message, for the same program.
///
/// # Safety
///
/// `handle` must be null, or a valid NUL-terminated C string that stays alive
/// for the call. Null is how the generated code says "the only connection
/// open"; anything else is a pointer to a constant in its own module.
#[cfg(not(target_family = "wasm"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn etamil_db_query(sql: i64, params: i64, handle: *const c_char) -> i64 {
    let named = if handle.is_null() {
        None
    } else {
        Some(unsafe { borrow_str(handle) })
    };

    // Parameters are coerced before a connection is touched, in that order,
    // because a parameter list that is not a list is the author's mistake and
    // should be reported as one rather than as a database error.
    let bound = match crate::db::params_from(&get(params)) {
        Ok(bound) => bound,
        Err(why) => fail(&why),
    };
    let sql = get(sql).to_string();

    let rows = HOST.with(|host| -> Result<Vec<Value>, String> {
        let mut host = host.borrow_mut();
        host.connection_for(named.as_deref())?.query(&sql, &bound)
    });

    match rows {
        Ok(rows) => put(Value::Array(rows)),
        Err(why) => fail(&why),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a compiled program would print for this handle.
    fn shown(handle: i64) -> String {
        get(handle).to_string()
    }

    fn number(text: &str) -> i64 {
        let c = std::ffi::CString::new(text).unwrap();
        unsafe { etamil_number(c.as_ptr()) }
    }

    fn text(value: &str) -> i64 {
        let c = std::ffi::CString::new(value).unwrap();
        unsafe { etamil_text(c.as_ptr()) }
    }

    // --- தளம்_வினா ------------------------------------------------------

    /// Open an in-memory SQLite connection on the same `HOST` a compiled
    /// program's query reaches, and put two rows in it.
    ///
    /// `:memory:` is never pooled, so each test gets its own database and the
    /// order tests run in cannot matter.
    #[cfg(feature = "sqlite")]
    fn two_rows(name: &str) {
        let lease = crate::db::pool::checkout("SQLite", ":memory:").expect("sqlite opens");
        HOST.with(|host| {
            host.borrow_mut()
                .connections
                .insert(name.to_string(), ":memory:".to_string(), lease)
        });
        HOST.with(|host| {
            let mut host = host.borrow_mut();
            let db = host.connection_for(Some(name)).expect("just opened");
            db.execute("CREATE TABLE t (x INTEGER, y TEXT)", &[])
                .unwrap();
            db.execute("INSERT INTO t VALUES (1, 'a')", &[]).unwrap();
            db.execute("INSERT INTO t VALUES (2, 'b')", &[]).unwrap();
        });
    }

    /// A C string that outlives the call, which is what the emitted IR hands
    /// these functions.
    #[cfg(feature = "sqlite")]
    fn c(text: &str) -> std::ffi::CString {
        std::ffi::CString::new(text).unwrap()
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn connect_execute_and_query_are_one_conversation() {
        // Everything a compiled program does with a database, through the C
        // entry points and nothing else. Until தரவுசேமி_இணை was built there
        // was no way to reach this from here at all: the query could only ever
        // report that no connection was open.
        let driver = c("SQLite");
        let named = c("ledger");
        unsafe { etamil_db_connect(driver.as_ptr(), text(":memory:"), named.as_ptr()) };

        unsafe {
            etamil_db_execute(
                text("CREATE TABLE kaNakku (kuRi INTEGER, peyar TEXT)"),
                etamil_array(),
                named.as_ptr(),
            );
            let row = etamil_array();
            etamil_array_push(row, number("1"));
            etamil_array_push(row, text("வரவு"));
            etamil_db_execute(
                text("INSERT INTO kaNakku VALUES (?, ?)"),
                row,
                named.as_ptr(),
            );
        }

        let rows = unsafe {
            etamil_db_query(
                text("SELECT kuRi, peyar FROM kaNakku"),
                etamil_array(),
                named.as_ptr(),
            )
        };
        assert_eq!(shown(rows), "[{kuRi: 1, peyar: வரவு}]");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn an_unnamed_connection_is_known_by_its_driver() {
        // `தளம்_இணை சீகுலைட், ":memory:"` names nothing, so the backend files it
        // under the driver's name — the default the bytecode compiler applies
        // too. A later statement that names nothing passes a null handle and
        // has to find it anyway.
        let driver = c("SQLite");
        unsafe { etamil_db_connect(driver.as_ptr(), text(":memory:"), driver.as_ptr()) };

        unsafe {
            etamil_db_execute(
                text("CREATE TABLE t (x INTEGER)"),
                etamil_array(),
                std::ptr::null(),
            );
            etamil_db_execute(
                text("INSERT INTO t VALUES (7)"),
                etamil_array(),
                std::ptr::null(),
            );
        }

        let rows =
            unsafe { etamil_db_query(text("SELECT x FROM t"), etamil_array(), std::ptr::null()) };
        assert_eq!(shown(rows), "[{x: 7}]");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn connecting_to_the_same_database_twice_keeps_the_first_connection() {
        // Under --server this statement is reached once per request, so the
        // second connect must be a no-op rather than a second lease. The table
        // created before it is the assertion: a fresh :memory: database would
        // not have one.
        let driver = c("SQLite");
        let named = c("again");
        unsafe {
            etamil_db_connect(driver.as_ptr(), text(":memory:"), named.as_ptr());
            etamil_db_execute(
                text("CREATE TABLE t (x INTEGER)"),
                etamil_array(),
                named.as_ptr(),
            );
            etamil_db_execute(
                text("INSERT INTO t VALUES (3)"),
                etamil_array(),
                named.as_ptr(),
            );
            etamil_db_connect(driver.as_ptr(), text(":memory:"), named.as_ptr());
        }

        let rows =
            unsafe { etamil_db_query(text("SELECT x FROM t"), etamil_array(), named.as_ptr()) };
        assert_eq!(shown(rows), "[{x: 3}]");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn a_query_answers_with_one_record_per_row() {
        two_rows("q");

        let sql = text("SELECT x, y FROM t ORDER BY x");
        let params = etamil_array();
        let named = std::ffi::CString::new("q").unwrap();
        let rows = unsafe { etamil_db_query(sql, params, named.as_ptr()) };

        // A table in the language's own terms: an array of records, printed
        // the way the VM prints the same value.
        assert_eq!(shown(rows), "[{x: 1, y: a}, {x: 2, y: b}]");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn a_bound_parameter_reaches_the_database() {
        two_rows("q");

        let sql = text("SELECT x, y FROM t WHERE x = ?");
        let params = etamil_array();
        etamil_array_push(params, number("2"));
        let named = std::ffi::CString::new("q").unwrap();

        assert_eq!(
            shown(unsafe { etamil_db_query(sql, params, named.as_ptr()) }),
            "[{x: 2, y: b}]"
        );
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn a_null_handle_means_the_only_connection_that_is_open() {
        // What `தளம்_வினா "…", []` without a name compiles to. `Option<&str>`
        // crosses the ABI as a pointer, and this is the None side of it.
        two_rows("whatever_it_is_called");

        let sql = text("SELECT x FROM t ORDER BY x");
        let params = etamil_array();
        let rows = unsafe { etamil_db_query(sql, params, std::ptr::null()) };

        assert_eq!(shown(rows), "[{x: 1}, {x: 2}]");
    }

    // --- the claim the language opens with -------------------------------

    #[test]
    fn a_tenth_and_a_fifth_are_exactly_three_tenths() {
        // The f64 backend answered 0.30000000000000004 and said nothing. This
        // is the whole reason the runtime exists.
        assert_eq!(shown(etamil_add(number("0.1"), number("0.2"))), "0.3");
    }

    #[test]
    fn a_third_keeps_every_digit_the_vm_keeps() {
        // A fixed two-place representation would answer 0.33 here, which is
        // why that workaround was not taken.
        assert_eq!(
            shown(etamil_divide(number("1"), number("3"))),
            "0.3333333333333333333333333333"
        );
    }

    #[test]
    fn trailing_zeros_are_trimmed_the_way_the_vm_trims_them() {
        // 1000 * 18% is 180.00 by scale and prints as 180.
        assert_eq!(
            shown(etamil_multiply(number("1000"), number("0.18"))),
            "180"
        );
        assert_eq!(shown(etamil_add(number("2.50"), number("0"))), "2.5");
        assert_eq!(shown(etamil_divide(number("10"), number("2"))), "5");
    }

    #[test]
    fn money_to_the_paisa_still_balances() {
        // The paise convention keeps working, and now so does writing it in
        // rupees, which the i64 backend refused outright.
        let total = etamil_add(number("2.05"), number("1.95"));
        assert_eq!(shown(total), "4");
        assert_eq!(shown(etamil_multiply(number("2.05"), number("7"))), "14.35");
    }

    // --- text, and why concat needed no knowledge of text ----------------

    #[test]
    fn concat_renders_each_side_the_way_print_would() {
        let label = text("விலை: ");
        assert_eq!(shown(etamil_concat(label, number("2.05"))), "விலை: 2.05");
        // A boolean renders as the VM renders it, not as 1.
        assert_eq!(shown(etamil_concat(text("x"), etamil_boolean(1))), "xtrue");
    }

    #[test]
    fn nil_is_handle_zero_and_prints_as_nil() {
        assert_eq!(etamil_nil(), 0);
        assert_eq!(shown(etamil_nil()), "nil");
    }

    // --- comparison ------------------------------------------------------

    #[test]
    fn ordering_and_equality_go_through_value() {
        assert_eq!(
            shown(etamil_compare(number("2"), number("3"), COMPARE_LT)),
            "true"
        );
        assert_eq!(
            shown(etamil_compare(number("3"), number("3"), COMPARE_LE)),
            "true"
        );
        assert_eq!(
            shown(etamil_compare(number("3"), number("3"), COMPARE_NE)),
            "false"
        );
        // Exactly equal, not equal within an epsilon: the f64 backend compared
        // two amounts a hundredth of a paisa apart as the same.
        assert_eq!(
            shown(etamil_compare(
                number("2.05"),
                number("2.0500000001"),
                COMPARE_EQ
            )),
            "false"
        );
    }

    #[test]
    fn truthiness_matches_what_the_vm_branches_on() {
        // Checked against the VM: 5 takes the branch and 0 does not.
        assert_eq!(etamil_truthy(number("5")), 1);
        assert_eq!(etamil_truthy(number("0")), 0);
        assert_eq!(etamil_truthy(etamil_boolean(0)), 0);
    }

    // --- collections -----------------------------------------------------

    #[test]
    fn an_array_literal_prints_the_way_the_vm_prints_one() {
        let array = etamil_array();
        etamil_array_push(array, number("1"));
        etamil_array_push(array, number("2.5"));
        assert_eq!(shown(array), "[1, 2.5]");
        assert_eq!(etamil_count(array), 2);
    }

    #[test]
    fn indexing_goes_through_the_vms_own_index_of() {
        let array = etamil_array();
        etamil_array_push(array, number("10"));
        etamil_array_push(array, number("20"));
        assert_eq!(shown(etamil_index(array, number("1"))), "20");

        let name = std::ffi::CString::new("aNi").unwrap();
        let updated = unsafe { etamil_index_set(array, number("0"), number("99"), name.as_ptr()) };
        assert_eq!(shown(etamil_index(updated, number("0"))), "99");
    }

    #[test]
    fn setting_an_element_leaves_every_other_holder_of_the_value_alone() {
        // `நகல் = அசல்; நகல்[0] = 9;` — both names held the same handle, and
        // the original must not change. It did, until the parity job caught
        // examples/language/nilY.qmz disagreeing with the VM.
        let original = etamil_array();
        etamil_array_push(original, number("1"));
        let name = std::ffi::CString::new("நகல்").unwrap();
        let copy = unsafe { etamil_index_set(original, number("0"), number("9"), name.as_ptr()) };
        assert_eq!(shown(copy), "[9]");
        assert_eq!(shown(original), "[1]");

        let record = etamil_record();
        let key = std::ffi::CString::new("அ").unwrap();
        unsafe { etamil_record_put(record, key.as_ptr(), number("1")) };
        let changed = unsafe { etamil_field_set(record, key.as_ptr(), number("2")) };
        assert_eq!(shown(changed), "{அ: 2}");
        assert_eq!(shown(record), "{அ: 1}");
    }

    /// The bug the parity job caught: this aborted the compiled program with
    /// "expected an array" while the VM inserted the field, so every program
    /// that filled a record by key — `விடை[திறவுகோல்] = மதிப்பு`, which is how
    /// vilAcam.qmz reads a upi:// link back — computed a different answer under
    /// --llvm than under --vm.
    #[test]
    fn a_record_can_be_filled_by_key_the_way_the_vm_fills_one() {
        let record = etamil_record();
        let name = std::ffi::CString::new("vitY").unwrap();

        let record =
            unsafe { etamil_index_set(record, text("pa"), text("shop@okhdfcbank"), name.as_ptr()) };
        let record = unsafe { etamil_index_set(record, text("tr"), text("INV-9"), name.as_ptr()) };

        assert_eq!(shown(etamil_index(record, text("pa"))), "shop@okhdfcbank");
        assert_eq!(shown(etamil_index(record, text("tr"))), "INV-9");
    }

    #[test]
    fn a_record_holds_and_returns_its_fields() {
        let record = etamil_record();
        let key = std::ffi::CString::new("பெயர்").unwrap();
        unsafe { etamil_record_put(record, key.as_ptr(), text("ராஜா")) };
        assert_eq!(shown(unsafe { etamil_field(record, key.as_ptr()) }), "ராஜா");
    }

    // --- builtins, through the interpreter's dispatch ---------------------

    #[test]
    fn for_each_walks_what_the_vm_walks() {
        // An array yields its items.
        let array = etamil_array();
        etamil_array_push(array, number("10"));
        etamil_array_push(array, number("20"));
        assert_eq!(etamil_count(array), 2);
        assert_eq!(shown(etamil_nth_or_key(array, 1)), "20");

        // A record yields its KEYS, sorted — not its values. Indexing it with a
        // number, which is what this backend did first, fails instead.
        let record = etamil_record();
        let b = std::ffi::CString::new("ஆ").unwrap();
        let a = std::ffi::CString::new("அ").unwrap();
        unsafe { etamil_record_put(record, b.as_ptr(), number("2")) };
        unsafe { etamil_record_put(record, a.as_ptr(), number("1")) };
        assert_eq!(etamil_count(record), 2);
        assert_eq!(shown(etamil_nth_or_key(record, 0)), "அ");
        assert_eq!(shown(etamil_nth_or_key(record, 1)), "ஆ");

        // A string yields Tamil letters, and a letter is a cluster: "வணக்கம்"
        // is five letters and rather more chars than that.
        let greeting = text("வணக்கம்");
        assert_eq!(etamil_count(greeting), 5);
        assert_eq!(shown(etamil_nth_or_key(greeting, 0)), "வ");
        assert_eq!(shown(etamil_nth_or_key(greeting, 1)), "ண");
    }

    #[test]
    fn a_builtin_is_the_interpreters_builtin() {
        // நீளம் is not reimplemented here, so it cannot disagree with the VM's.
        let array = etamil_array();
        etamil_array_push(array, number("1"));
        etamil_array_push(array, number("2"));
        etamil_array_push(array, number("3"));

        let name = std::ffi::CString::new("நீளம்").unwrap();
        let argv = [array];
        assert_eq!(
            shown(unsafe { etamil_call(name.as_ptr(), argv.as_ptr(), 1) }),
            "3"
        );
    }

    #[test]
    fn floor_and_ceiling_come_from_the_same_place_too() {
        let floor = std::ffi::CString::new("தரை").unwrap();
        let argv = [etamil_divide(number("205"), number("100"))];
        assert_eq!(
            shown(unsafe { etamil_call(floor.as_ptr(), argv.as_ptr(), 1) }),
            "2"
        );

        let ceil = std::ffi::CString::new("மேல்").unwrap();
        let argv = [etamil_divide(number("1000"), number("3"))];
        assert_eq!(
            shown(unsafe { etamil_call(ceil.as_ptr(), argv.as_ptr(), 1) }),
            "334"
        );
    }

    // --- functions as values ----------------------------------------------

    /// Stands in for what codegen.rs emits beside a function: the arguments
    /// as one array. This one is `செயல்(வீதம்; தொகை) { திரும்பு தொகை * வீதம்; }`
    /// with one capture.
    unsafe extern "C" fn times_captured(argv: *const i64) -> i64 {
        let (rate, amount) = unsafe { (*argv, *argv.add(1)) };
        etamil_multiply(amount, rate)
    }

    #[test]
    fn a_function_value_carries_its_captures_into_the_call() {
        let name = std::ffi::CString::new("#செயல்_test_rate").unwrap();
        unsafe { etamil_register_function(name.as_ptr(), times_captured, 1, 1) };

        let captured = [number("0.18")];
        let function = unsafe { etamil_function(name.as_ptr(), captured.as_ptr(), 1) };
        assert_eq!(shown(function), "<செயல்>");

        let argv = [number("1000")];
        assert_eq!(
            shown(unsafe { etamil_call_value(function, argv.as_ptr(), 1) }),
            "180"
        );
    }

    #[test]
    fn a_builtin_can_be_called_through_a_value() {
        let name = std::ffi::CString::new("நீளம்").unwrap();
        let function = unsafe { etamil_function(name.as_ptr(), std::ptr::null(), 0) };
        let array = etamil_array();
        etamil_array_push(array, number("1"));
        etamil_array_push(array, number("2"));
        let argv = [array];
        assert_eq!(
            shown(unsafe { etamil_call_value(function, argv.as_ptr(), 1) }),
            "2"
        );
    }

    #[test]
    fn a_variable_that_holds_no_function_leaves_the_call_to_the_name() {
        // What `நீளம்(x)` compiles to when a variable called நீளம் exists and
        // holds a number: the builtin, as the VM calls it.
        let name = std::ffi::CString::new("நீளம்").unwrap();
        let array = etamil_array();
        etamil_array_push(array, number("7"));
        let argv = [array];
        assert_eq!(
            shown(unsafe { etamil_call_named(number("5"), name.as_ptr(), argv.as_ptr(), 1) }),
            "1"
        );
    }

    // --- shapes and methods -----------------------------------------------

    /// What codegen.rs registers for `வடிவம் கடன் { எண் அசல், செயல் இரட்டி(இது) }`.
    fn loan_shape() -> std::ffi::CString {
        let shape = std::ffi::CString::new("கடன்_rt").unwrap();
        let field = std::ffi::CString::new("அசல்").unwrap();
        let method = std::ffi::CString::new("இரட்டி").unwrap();
        unsafe {
            etamil_shape_begin(shape.as_ptr());
            etamil_shape_field(
                shape.as_ptr(),
                field.as_ptr(),
                shape::type_code(&Some(crate::parser::DeclaredType::Number)),
                std::ptr::null(),
            );
            etamil_shape_method(shape.as_ptr(), method.as_ptr(), 1);
        }
        shape
    }

    fn loan(shape: &std::ffi::CString, principal: i64) -> i64 {
        let record = etamil_record();
        let key = std::ffi::CString::new("அசல்").unwrap();
        unsafe {
            etamil_record_put(record, key.as_ptr(), principal);
            etamil_shape_make(record, shape.as_ptr(), 0, 0)
        }
    }

    #[test]
    fn a_shaped_literal_is_checked_and_tagged() {
        let shape = loan_shape();
        let made = loan(&shape, number("1000"));
        assert_eq!(shown(made), "கடன்_rt{அசல்: 1000}");
    }

    #[test]
    fn a_plain_record_converts_to_a_shape_as_a_result() {
        let shape = loan_shape();
        let key = std::ffi::CString::new("அசல்").unwrap();
        let record = etamil_record();
        unsafe { etamil_record_put(record, key.as_ptr(), number("5")) };
        let converted = unsafe { etamil_shape_convert(shape.as_ptr(), record) };
        assert_eq!(shown(converted), "சரி(கடன்_rt{அசல்: 5})");

        let wrong = etamil_record();
        unsafe { etamil_record_put(wrong, key.as_ptr(), text("ஐந்து")) };
        let refused = unsafe { etamil_shape_convert(shape.as_ptr(), wrong) };
        assert!(shown(refused).starts_with("தவறு("), "{}", shown(refused));
    }

    /// Stands in for `கடன்_rt.இரட்டி`'s array entry: இது first.
    unsafe extern "C" fn doubled_principal(argv: *const i64) -> i64 {
        let this = unsafe { *argv };
        let key = std::ffi::CString::new("அசல்").unwrap();
        let principal = unsafe { etamil_field(this, key.as_ptr()) };
        etamil_add(principal, principal)
    }

    #[test]
    fn a_method_is_found_on_the_record_and_given_it_first() {
        let shape = loan_shape();
        let function = std::ffi::CString::new(shape::method_function("கடன்_rt", "இரட்டி")).unwrap();
        unsafe { etamil_register_function(function.as_ptr(), doubled_principal, 0, 1) };

        let made = loan(&shape, number("21"));
        let method = std::ffi::CString::new("இரட்டி").unwrap();
        let answer = unsafe { etamil_call_method(made, method.as_ptr(), std::ptr::null(), 0) };
        assert_eq!(shown(answer), "42");
    }

    #[test]
    fn a_function_held_in_a_plain_record_is_called_by_its_field() {
        let record = etamil_record();
        let key = std::ffi::CString::new("அளவு").unwrap();
        let builtin = std::ffi::CString::new("நீளம்").unwrap();
        let function = unsafe { etamil_function(builtin.as_ptr(), std::ptr::null(), 0) };
        unsafe { etamil_record_put(record, key.as_ptr(), function) };

        let array = etamil_array();
        etamil_array_push(array, number("1"));
        let argv = [array];
        let answer = unsafe { etamil_call_method(record, key.as_ptr(), argv.as_ptr(), 1) };
        assert_eq!(shown(answer), "1");
    }

    #[test]
    fn a_result_can_be_tested_and_unwrapped() {
        let ok = std::ffi::CString::new("சரி").unwrap();
        let argv = [number("7")];
        let wrapped = unsafe { etamil_call(ok.as_ptr(), argv.as_ptr(), 1) };
        assert_eq!(etamil_is_error(wrapped), 0);
        assert_eq!(shown(etamil_unwrap(wrapped)), "7");

        let err = std::ffi::CString::new("தவறு").unwrap();
        let argv = [text("போச்சு")];
        let failed = unsafe { etamil_call(err.as_ptr(), argv.as_ptr(), 1) };
        assert_eq!(etamil_is_error(failed), 1);
    }
}
