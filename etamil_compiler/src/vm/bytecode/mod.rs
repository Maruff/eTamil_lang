// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
// Bytecode instruction set for the eTamil VM
pub mod compiler;

/// Bytecode instruction set
#[derive(Debug, Clone)]
pub enum Instruction {
    // Stack operations
    Push(crate::vm::Value),
    Pop,

    // Variable operations
    StoreVar(String),
    LoadVar(String),
    /// `x = இணை(x, v)` — pop v and push it onto x's own array.
    ///
    /// Emitted only for that exact shape, where the array is read and written
    /// back under the same name in one statement. `இணை` is a function that
    /// returns a new array, so the general form has to copy; this form cannot
    /// be observed mid-flight, so it does not.
    AppendVar(String),

    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,
    NotEqual,
    LessThan,
    LessOrEqual,
    GreaterThan,
    GreaterOrEqual,

    // Logical
    And,
    Or,
    Not,

    // Control flow
    JumpIfFalse(usize),
    Jump(usize),

    // I/O
    Print,
    Input,

    // String operations
    Concat,
    /// `x = x & expr` — append to the string already in x, in place.
    ///
    /// Concat builds a new string from both sides, so accumulating one in
    /// a loop copies everything written so far on every pass. This appends
    /// to the existing buffer, which grows geometrically.
    ConcatVar(String),

    // Collections
    /// Pop n values into an array (அணி — a column).
    MakeArray(usize),
    /// Pop one value per key into a record (பொருள் — a row).
    MakeRecord(Vec<String>),
    /// Pop index then base; push the element.
    Index,
    /// Pop base; push the named field.
    Field(String),
    /// Pop index and value; store into the named variable's collection.
    SetIndex(String),
    /// Pop value; store into the named variable's field.
    SetField(String, String),
    /// `?` — pop a result; push the சரி value, or unwind the current call
    /// and return the தவறு to the caller.
    TryUnwrap,
    /// Pop a collection; push how many elements it has.
    Length,
    /// Pop index then collection; push the nth element of an array, the nth
    /// key of a record (keys sorted so iteration order is stable), or the
    /// nth character of a string. Used to desugar ஒவ்வொரு.
    NthOrKey,

    // File I/O — each pops its operands off the stack
    FileOpen(String), // mode; pops filename
    FileClose,        // pops filename
    FileWrite,        // pops data, then filename
    FileRead,         // pops filename, pushes contents
    ReadCSV,          // pops filename, pushes the number of data rows
    WriteCSV,         // pops row, then filename

    // Database. Queries carry their parameters separately so values are
    // bound by the driver rather than spliced into the SQL text.
    /// Pop the connection string; open a connection under this type name.
    /// Driver, and the name this connection is known by.
    DBConnect(String, String),
    /// Close and forget the connection for this type name.
    DBDisconnect(String),
    /// Pop params then SQL; push an array of records, one per row.
    /// Which connection to ask; `None` means the only open one.
    DBQuery(Option<String>),
    /// Pop params then SQL; run it, discarding the affected-row count.
    /// Which connection to run on; `None` means the only open one.
    DBExecute(Option<String>),

    // API
    DefineRoute(String, String), // method, path
    /// Pop body then status; record them for the server to send back.
    SendResponse,
    /// ஜேசான்_உரை — a response with the JSON content type on it.
    SendJSON,
    StartServer(String, u16), // host, port

    // Functions
    /// Call a named function with this many arguments already on the stack.
    ///
    /// A variable of that name holding a function value is called first, so a
    /// parameter can be called: `செயல் ஒவ்வொன்றுக்கும்(பட்டியல், செ) { … செ(x) … }`.
    Call(String, usize),
    /// Pop this many arguments, then the value to call, and call it.
    CallValue(usize),
    /// Push a function value for this function, carrying the current values of
    /// the named locals with it. Empty for a named function, which captures
    /// nothing; a local that does not exist yet is carried as இன்மை.
    MakeFunction(String, Vec<String>),
    /// `r.m(args)` — pop this many arguments, then the receiver: call the
    /// method of its shape, or a function it holds in that field.
    CallMethod(String, usize),
    /// `வடிவம்{…}` — pop one value per key, and before them the `..` record
    /// when there is one; push a record of that shape, or fail if it does not
    /// fit.
    MakeShaped {
        shape: String,
        keys: Vec<String>,
        with_base: bool,
    },
    /// Pop the return value, restore the caller's frame, push the value back.
    Return,

    // Misc
    Nop,
    /// A statement the VM cannot execute. Carries the message shown to the
    /// user; executing it is a runtime error rather than a silent no-op.
    Unsupported(String),
    Halt,
}

/// Where a function's body starts, and the names its arguments bind to.
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub start: usize,
    pub params: Vec<String>,
    /// Locals of the enclosing function an anonymous செயல் carries with it,
    /// bound before the parameters. Always empty for a named function.
    pub captures: Vec<String>,
}

/// An anonymous செயல் as it was compiled: the hidden name its function values
/// carry, what it captures, and its source.
///
/// Kept because the REPL compiles every line as a new program over a VM that
/// keeps its variables. A function value made on one line names a body that
/// line's bytecode held; the next line's program has to hold it too, under the
/// same name, or calling the value would find nothing.
#[derive(Debug, Clone)]
pub struct LambdaSource {
    pub name: String,
    pub captures: Vec<String>,
    pub params: Vec<crate::parser::Param>,
    pub body: Vec<crate::parser::Stmt>,
}

/// Complete bytecode program
#[derive(Debug, Clone)]
pub struct Bytecode {
    pub instructions: Vec<Instruction>,
    /// Function bodies are emitted inline and jumped over; this maps a name
    /// to its entry point. Resolution happens at call time, so functions may
    /// be defined in any order and may recurse.
    pub functions: std::collections::HashMap<String, FunctionInfo>,
    /// Every வடிவம் the program declares, for the checks a shaped record
    /// gets when it is built or changed at runtime.
    pub shapes: crate::vm::shape::Shapes,
    /// Every anonymous செயல் this program compiled. See `LambdaSource`.
    pub lambdas: Vec<LambdaSource>,
}

impl Default for Bytecode {
    fn default() -> Self {
        Self::new()
    }
}

impl Bytecode {
    pub fn new() -> Self {
        Bytecode {
            instructions: Vec::new(),
            functions: std::collections::HashMap::new(),
            shapes: crate::vm::shape::Shapes::new(),
            lambdas: Vec::new(),
        }
    }

    pub fn push(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}
