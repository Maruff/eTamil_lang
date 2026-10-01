// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! LLVM code generation for eTamil, over `llvm-sys` (LLVM 18).
//!
//! ## What the emitted IR holds
//!
//! Handles. Every eTamil value is an `i64` index into an arena that lives in
//! `crate::runtime`, and every operation on one is a call into that module.
//! The IR carries the *control flow* — branches, loops, calls, returns — and
//! nothing about what a value is.
//!
//! That is a deliberate reversal. Two earlier versions of this file held values
//! in registers, first as `double` and then as `i64`, and both bought the same
//! thing at the same price: arithmetic was fast and the language was whatever
//! fitted in a register. Numbers fitted. Strings, arrays, records, results and
//! `இன்மை` did not, and neither did a decimal — `1 / 3` is
//! `0.3333333333333333333333333333` on the VM, and no register holds that.
//!
//! Handles cost a call per operation and buy the whole language, plus exact
//! decimals, plus formatting that cannot drift from the VM's, plus every
//! builtin there is. Not a number, deliberately: this said "fifty-nine" for
//! long enough to become wrong twice over, and the count was never the
//! point. Nothing here knows one builtin from another, so a builtin added
//! to the interpreter is reachable from compiled code without this file
//! being touched. See `src/runtime.rs` for why each of those follows rather
//! than being implemented twice.
//!
//! ## The consequence for linking
//!
//! `output.ll` is no longer self-contained:
//!
//! ```text
//! clang output.ll -o prog -L etamil_compiler/target/release \
//!       -letamil_compiler -Wl,-rpath,etamil_compiler/target/release
//! ```
//!
//! `scripts/run_parity.sh` does this. `Cargo.toml` already built the `cdylib`,
//! so there is no new artefact — only new exported symbols in it.
//!
//! ## What is still refused
//!
//! Statements, not expressions: files, HTTP, routes, scheduling, and the
//! database statements beyond connecting, executing and querying.
//! Those need the VM's own machinery rather than a value representation, and
//! `stmt_label` names each one so `run_parity.sh` can rank them. Refusing is
//! the whole discipline here — IR that drops a statement or evaluates an
//! expression as a placeholder would make a compiled program quietly disagree
//! with the same source on the VM, and that is the one failure this project
//! does not accept.

#[cfg(feature = "llvm")]
use crate::parser::Expr;
use crate::parser::Stmt;
#[cfg(feature = "llvm")]
use llvm_sys::core::*;
#[cfg(feature = "llvm")]
use llvm_sys::prelude::*;
#[cfg(feature = "llvm")]
use llvm_sys::{LLVMIntPredicate, LLVMLinkage};
#[cfg(feature = "llvm")]
use std::collections::HashMap;
#[cfg(feature = "llvm")]
use std::ffi::CString;
#[cfg(feature = "llvm")]
use std::ptr;

#[cfg(feature = "llvm")]
pub struct Compiler {
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    function: LLVMValueRef,
    /// Name to the stack slot holding its handle. Locals, and parameters.
    variables: HashMap<String, LLVMValueRef>,
    /// Top-level names, as module globals so a `செயல்` can read one. A
    /// function's frame cannot reach `main`'s, and a function reading a global
    /// is how nUlakam is written throughout.
    globals: HashMap<String, LLVMValueRef>,
    functions: HashMap<String, LLVMValueRef>,
    /// Every function compiled, with how many captures lead its parameters
    /// and how many arguments a caller supplies. Each is registered with the
    /// runtime at startup, so a call through a function value can reach it.
    compiled: Vec<(String, usize, usize)>,
    /// Runs as `main`'s first instruction and registers every function.
    registrar: LLVMValueRef,
    /// Numbers the anonymous செயல்s in this module.
    lambdas: usize,
    /// The program's வடிவம்s, registered with the runtime at startup and
    /// known by name before any statement is compiled.
    shapes: crate::vm::shape::Shapes,
    in_function: bool,
    terminated: bool,
    /// Constructs this backend cannot build. The caller must refuse to emit
    /// when this is non-empty; see the module comment.
    unsupported: Vec<String>,
}

#[cfg(not(feature = "llvm"))]
pub struct Compiler {
    // Placeholder struct for non-LLVM builds
}

#[cfg(feature = "llvm")]
impl Compiler {
    pub fn new() -> Self {
        unsafe {
            let context = LLVMContextCreate();
            let module = LLVMModuleCreateWithNameInContext(
                CString::new("etamil_module").unwrap().as_ptr(),
                context,
            );
            let builder = LLVMCreateBuilderInContext(context);

            let i32_type = LLVMInt32TypeInContext(context);
            let fn_type = LLVMFunctionType(i32_type, ptr::null_mut(), 0, 0);
            let function = LLVMAddFunction(module, CString::new("main").unwrap().as_ptr(), fn_type);

            let entry = LLVMAppendBasicBlockInContext(
                context,
                function,
                CString::new("entry").unwrap().as_ptr(),
            );
            LLVMPositionBuilderAtEnd(builder, entry);

            // Registration happens before the program's first statement, so a
            // function value made anywhere can be called anywhere. Its body is
            // filled in once every function is known — see `finish_registrar`.
            // The dot keeps the name out of reach of any source identifier.
            let void_fn = LLVMFunctionType(LLVMVoidTypeInContext(context), ptr::null_mut(), 0, 0);
            let registrar = LLVMAddFunction(
                module,
                CString::new("etamil.register").unwrap().as_ptr(),
                void_fn,
            );
            LLVMSetLinkage(registrar, LLVMLinkage::LLVMInternalLinkage);
            LLVMBuildCall2(
                builder,
                void_fn,
                registrar,
                ptr::null_mut(),
                0,
                CString::new("").unwrap().as_ptr(),
            );

            Compiler {
                context,
                module,
                builder,
                function,
                variables: HashMap::new(),
                globals: HashMap::new(),
                functions: HashMap::new(),
                compiled: Vec::new(),
                registrar,
                lambdas: 0,
                shapes: crate::vm::shape::Shapes::new(),
                in_function: false,
                terminated: false,
                unsupported: Vec::new(),
            }
        }
    }

    /// Constructs encountered that this backend cannot compile.
    pub fn unsupported(&self) -> &[String] {
        &self.unsupported
    }

    // --- Types -------------------------------------------------------------

    /// A handle. Every eTamil value in the emitted IR has this type, which is
    /// what makes the rest of this file short.
    fn value(&self) -> LLVMTypeRef {
        unsafe { LLVMInt64TypeInContext(self.context) }
    }

    fn word(&self) -> LLVMTypeRef {
        unsafe { LLVMInt32TypeInContext(self.context) }
    }

    fn text(&self) -> LLVMTypeRef {
        unsafe { LLVMPointerType(LLVMInt8TypeInContext(self.context), 0) }
    }

    fn nothing(&self) -> LLVMTypeRef {
        unsafe { LLVMVoidTypeInContext(self.context) }
    }

    // --- Calling the runtime -----------------------------------------------

    fn declare(
        &self,
        name: &str,
        mut params: Vec<LLVMTypeRef>,
        returns: LLVMTypeRef,
    ) -> (LLVMValueRef, LLVMTypeRef) {
        unsafe {
            let kind = LLVMFunctionType(returns, params.as_mut_ptr(), params.len() as u32, 0);
            let c_name = CString::new(name).unwrap();
            let existing = LLVMGetNamedFunction(self.module, c_name.as_ptr());
            if !existing.is_null() {
                return (existing, kind);
            }
            (LLVMAddFunction(self.module, c_name.as_ptr(), kind), kind)
        }
    }

    fn invoke(
        &self,
        name: &str,
        params: Vec<LLVMTypeRef>,
        returns: LLVMTypeRef,
        args: &mut [LLVMValueRef],
    ) -> LLVMValueRef {
        unsafe {
            let (function, kind) = self.declare(name, params, returns);
            LLVMBuildCall2(
                self.builder,
                kind,
                function,
                args.as_mut_ptr(),
                args.len() as u32,
                CString::new("").unwrap().as_ptr(),
            )
        }
    }

    /// The common shape: handles in, a handle out.
    fn call_values(&self, name: &str, args: &mut [LLVMValueRef]) -> LLVMValueRef {
        let params = vec![self.value(); args.len()];
        self.invoke(name, params, self.value(), args)
    }

    /// Handles in, nothing out.
    fn call_void(&self, name: &str, args: &mut [LLVMValueRef]) {
        let params = vec![self.value(); args.len()];
        self.invoke(name, params, self.nothing(), args);
    }

    /// A NUL-terminated constant in the module, and a pointer to it.
    ///
    /// Returns `None` for text with a NUL inside it, which would end the C
    /// string early and hand the runtime less than the program wrote.
    fn constant_text(&mut self, body: &str, label: &str) -> Option<LLVMValueRef> {
        let literal = match CString::new(body) {
            Ok(literal) => literal,
            Err(_) => {
                self.unsupported
                    .push("text with a NUL byte in it".to_string());
                return None;
            }
        };
        unsafe {
            Some(LLVMBuildGlobalStringPtr(
                self.builder,
                literal.as_ptr(),
                CString::new(label).unwrap().as_ptr(),
            ))
        }
    }

    fn nil(&self) -> LLVMValueRef {
        self.invoke("etamil_nil", vec![], self.value(), &mut [])
    }

    // --- Storage -----------------------------------------------------------

    /// An `alloca` at the top of the current function's entry block, wherever
    /// the builder is standing. A slot allocated inside a branch is not
    /// dominated by a load after it, which is invalid IR that LLVM only
    /// complains about later.
    fn entry_slot(&mut self, name: &str) -> LLVMValueRef {
        unsafe {
            let here = LLVMGetInsertBlock(self.builder);
            let entry = LLVMGetEntryBasicBlock(self.function);
            let first = LLVMGetFirstInstruction(entry);
            if first.is_null() {
                LLVMPositionBuilderAtEnd(self.builder, entry);
            } else {
                LLVMPositionBuilderBefore(self.builder, first);
            }
            let slot = LLVMBuildAlloca(
                self.builder,
                self.value(),
                CString::new(name)
                    .unwrap_or_else(|_| CString::new("slot").unwrap())
                    .as_ptr(),
            );
            // Zero is the nil handle. A slot read before anything is stored in
            // it — a local captured by a செயல் before the branch that assigns
            // it has run — then reads இன்மை, as the VM's frame does, rather
            // than whatever the stack held.
            LLVMBuildStore(self.builder, LLVMConstInt(self.value(), 0, 0), slot);
            LLVMPositionBuilderAtEnd(self.builder, here);
            slot
        }
    }

    fn declare_global(&mut self, name: &str) -> LLVMValueRef {
        if let Some(global) = self.globals.get(name).copied() {
            return global;
        }
        unsafe {
            let global = LLVMAddGlobal(
                self.module,
                self.value(),
                CString::new(name)
                    .unwrap_or_else(|_| CString::new("global").unwrap())
                    .as_ptr(),
            );
            // Zero is the nil handle, so an unassigned global reads as இன்மை
            // rather than as a wild index.
            LLVMSetInitializer(global, LLVMConstInt(self.value(), 0, 0));
            LLVMSetLinkage(global, LLVMLinkage::LLVMInternalLinkage);
            self.globals.insert(name.to_string(), global);
            global
        }
    }

    /// Where an assignment to this name writes.
    ///
    /// Inside a `செயல்` a name is always local, even when a global of the same
    /// name exists: assigning to one in a function makes a local, which is what
    /// the VM does and what nUlakam is written around.
    fn storage_for(&mut self, name: &str) -> LLVMValueRef {
        if let Some(slot) = self.variables.get(name).copied() {
            return slot;
        }
        if !self.in_function {
            if let Some(global) = self.globals.get(name).copied() {
                return global;
            }
        }
        let slot = self.entry_slot(name);
        self.variables.insert(name.to_string(), slot);
        slot
    }

    fn lookup(&self, name: &str) -> Option<LLVMValueRef> {
        self.variables
            .get(name)
            .copied()
            .or_else(|| self.globals.get(name).copied())
    }

    // --- Labels for what is refused ----------------------------------------
    //
    // `stmt_label` and `refusals` are free functions at the end of this file,
    // outside the `llvm` feature gate, so that the gap can be counted on a
    // machine that cannot build the backend. See the note there.

    // --- Compiling ---------------------------------------------------------

    pub fn compile(&mut self, statements: Vec<Stmt>) {
        // Top-level names get their storage before any function body is
        // compiled, because a body may read one and bodies are compiled first —
        // so a name assigned further down the file would otherwise be invisible
        // from inside a `செயல்` written above it.
        //
        // Every name the top level binds, at any depth outside a function: a
        // loop variable or a query's rows is a global on the VM too, and a
        // `செயல்` — named or written as a value — reads it live from there.
        let mut top_level = Vec::new();
        top_level_bindings(&statements, &mut top_level);
        for name in &top_level {
            self.declare_global(name);
        }
        self.shapes = crate::vm::shape::from_program(&statements);

        unsafe {
            for statement in &statements {
                match statement {
                    Stmt::FunctionDef { name, params, .. } => {
                        self.declare_function(name, params.len());
                    }
                    // A method is a function named `வடிவம்.முறை`, இது first.
                    Stmt::ShapeDef { name, methods, .. } => {
                        for method in methods {
                            let function = crate::vm::shape::method_function(name, &method.name);
                            self.declare_function(&function, method.params.len());
                        }
                    }
                    _ => {}
                }
            }

            for statement in &statements {
                match statement {
                    Stmt::FunctionDef {
                        name, params, body, ..
                    } => self.compile_function(name, &[], params, body),
                    Stmt::ShapeDef { name, methods, .. } => {
                        for method in methods {
                            let function = crate::vm::shape::method_function(name, &method.name);
                            self.compile_function(&function, &[], &method.params, &method.body);
                        }
                    }
                    _ => {}
                }
            }

            for statement in statements {
                if !matches!(statement, Stmt::FunctionDef { .. } | Stmt::ShapeDef { .. }) {
                    self.compile_stmt(statement);
                }
            }

            let zero = LLVMConstInt(self.word(), 0, 0);
            LLVMBuildRet(self.builder, zero);

            self.finish_registrar();
        }
    }

    /// Fill in the function `main` calls first: one registration per function
    /// this module compiled, each with an entry that takes its arguments as an
    /// array, which is the only shape a call through a value can supply.
    fn finish_registrar(&mut self) {
        unsafe {
            let saved_function = self.function;
            self.function = self.registrar;
            let entry = LLVMAppendBasicBlockInContext(
                self.context,
                self.registrar,
                CString::new("entry").unwrap().as_ptr(),
            );
            LLVMPositionBuilderAtEnd(self.builder, entry);

            for (name, captures, params) in self.compiled.clone() {
                let function = match self.functions.get(&name).copied() {
                    Some(function) => function,
                    None => continue,
                };
                let array_entry = self.array_entry(&name, function, captures + params);
                if let Some(label) = self.constant_text(&name, "function") {
                    let captures = LLVMConstInt(self.value(), captures as u64, 0);
                    let params = LLVMConstInt(self.value(), params as u64, 0);
                    self.invoke(
                        "etamil_register_function",
                        vec![self.text(), self.text(), self.value(), self.value()],
                        self.nothing(),
                        &mut [label, array_entry, captures, params],
                    );
                }
            }

            self.register_shapes();
            LLVMBuildRetVoid(self.builder);
            self.function = saved_function;
        }
    }

    /// Each shape, a field and a method at a time, for the runtime's checks.
    /// Emitted into the registrar, whose block the builder is standing in.
    fn register_shapes(&mut self) {
        let mut shapes: Vec<crate::vm::shape::Shape> = self.shapes.values().cloned().collect();
        // Sorted so the emitted IR is the same from one build to the next.
        shapes.sort_by(|a, b| a.name.cmp(&b.name));

        for shape in shapes {
            let Some(owner) = self.constant_text(&shape.name, "shape") else {
                continue;
            };
            self.invoke(
                "etamil_shape_begin",
                vec![self.text()],
                self.nothing(),
                &mut [owner],
            );

            for (field, declared) in &shape.fields {
                let Some(label) = self.constant_text(field, "field") else {
                    continue;
                };
                let code = crate::vm::shape::type_code(declared);
                let of = match declared {
                    Some(crate::parser::DeclaredType::Shape(inner)) => {
                        match self.constant_text(inner, "shape") {
                            Some(inner) => inner,
                            None => continue,
                        }
                    }
                    _ => unsafe { LLVMConstNull(self.text()) },
                };
                unsafe {
                    let code = LLVMConstInt(self.word(), code as u64, 0);
                    self.invoke(
                        "etamil_shape_field",
                        vec![self.text(), self.text(), self.word(), self.text()],
                        self.nothing(),
                        &mut [owner, label, code, of],
                    );
                }
            }

            let mut methods: Vec<(&String, &bool)> = shape.methods.iter().collect();
            methods.sort();
            for (method, takes_self) in methods {
                let Some(label) = self.constant_text(method, "method") else {
                    continue;
                };
                unsafe {
                    let takes_self = LLVMConstInt(self.word(), u64::from(*takes_self), 0);
                    self.invoke(
                        "etamil_shape_method",
                        vec![self.text(), self.text(), self.word()],
                        self.nothing(),
                        &mut [owner, label, takes_self],
                    );
                }
            }
        }
    }

    /// `கடன்{அசல்: …, ..பழையது}`: the fields go on a fresh plain record, and
    /// the runtime checks them against the shape and makes it one — with the
    /// same `vm::shape::build` the VM's `MakeShaped` calls.
    fn compile_shape_literal(
        &mut self,
        shape: &str,
        fields: &[(String, Expr)],
        base: Option<&Expr>,
    ) -> LLVMValueRef {
        // The `..` record first, as the VM evaluates it first.
        let (base, has_base) = match base {
            Some(base) => (self.compile_expr(base), 1),
            None => (self.nil(), 0),
        };
        let record = self.invoke("etamil_record", vec![], self.value(), &mut []);
        for (field, value) in fields {
            let handle = self.compile_expr(value);
            if let Some(key) = self.constant_text(field, "field") {
                self.invoke(
                    "etamil_record_put",
                    vec![self.value(), self.text(), self.value()],
                    self.nothing(),
                    &mut [record, key, handle],
                );
            }
        }
        let Some(name) = self.constant_text(shape, "shape") else {
            return self.nil();
        };
        unsafe {
            let has_base = LLVMConstInt(self.word(), has_base, 0);
            self.invoke(
                "etamil_shape_make",
                vec![self.value(), self.text(), self.value(), self.word()],
                self.value(),
                &mut [record, name, base, has_base],
            )
        }
    }

    /// `r.m(…)`. With a shape's name for r it is that shape's function,
    /// called directly; otherwise the runtime finds the method of r's shape,
    /// or a function r holds in that field.
    fn compile_method_call(&mut self, receiver: &Expr, name: &str, args: &[Expr]) -> LLVMValueRef {
        if let Expr::Variable(shape) = receiver
            && self.shapes.contains_key(shape)
        {
            let function = crate::vm::shape::method_function(shape, name);
            return self.compile_call(&function, args);
        }

        let receiver = self.compile_expr(receiver);
        let handles: Vec<LLVMValueRef> = args.iter().map(|arg| self.compile_expr(arg)).collect();
        let Some(label) = self.constant_text(name, "method") else {
            return self.nil();
        };
        let (argv, argc) = self.argv(&handles);
        unsafe {
            self.invoke(
                "etamil_call_method",
                vec![
                    self.value(),
                    self.text(),
                    LLVMPointerType(self.value(), 0),
                    self.value(),
                ],
                self.value(),
                &mut [receiver, label, argv, argc],
            )
        }
    }

    /// `i64 name.entry(ptr argv)`: load `count` handles and call `function`
    /// with them. Leaves the builder where it found it.
    fn array_entry(&mut self, name: &str, function: LLVMValueRef, count: usize) -> LLVMValueRef {
        unsafe {
            let mut params = [LLVMPointerType(self.value(), 0)];
            let kind = LLVMFunctionType(self.value(), params.as_mut_ptr(), 1, 0);
            let entry_fn = LLVMAddFunction(
                self.module,
                CString::new(format!("{}.entry", name))
                    .unwrap_or_else(|_| CString::new("entry").unwrap())
                    .as_ptr(),
                kind,
            );
            LLVMSetLinkage(entry_fn, LLVMLinkage::LLVMInternalLinkage);

            let here = LLVMGetInsertBlock(self.builder);
            let block = LLVMAppendBasicBlockInContext(
                self.context,
                entry_fn,
                CString::new("entry").unwrap().as_ptr(),
            );
            LLVMPositionBuilderAtEnd(self.builder, block);

            let argv = LLVMGetParam(entry_fn, 0);
            let mut args = Vec::with_capacity(count);
            for position in 0..count {
                let mut index = [LLVMConstInt(self.value(), position as u64, 0)];
                let slot = LLVMBuildGEP2(
                    self.builder,
                    self.value(),
                    argv,
                    index.as_mut_ptr(),
                    1,
                    CString::new("arg").unwrap().as_ptr(),
                );
                args.push(LLVMBuildLoad2(
                    self.builder,
                    self.value(),
                    slot,
                    CString::new("arg").unwrap().as_ptr(),
                ));
            }
            let answer = LLVMBuildCall2(
                self.builder,
                self.llvm_function_type(count),
                function,
                args.as_mut_ptr(),
                count as u32,
                CString::new("answer").unwrap().as_ptr(),
            );
            LLVMBuildRet(self.builder, answer);

            LLVMPositionBuilderAtEnd(self.builder, here);
            entry_fn
        }
    }

    /// A stack array of handles in the current function's entry block, filled
    /// here, and a pointer to its first element: how a variable number of
    /// handles crosses the C ABI.
    fn argv(&mut self, handles: &[LLVMValueRef]) -> (LLVMValueRef, LLVMValueRef) {
        unsafe {
            let count = handles.len();
            let array_type = LLVMArrayType2(self.value(), count.max(1) as u64);
            let argv = {
                let here = LLVMGetInsertBlock(self.builder);
                let entry = LLVMGetEntryBasicBlock(self.function);
                let first = LLVMGetFirstInstruction(entry);
                if first.is_null() {
                    LLVMPositionBuilderAtEnd(self.builder, entry);
                } else {
                    LLVMPositionBuilderBefore(self.builder, first);
                }
                let argv = LLVMBuildAlloca(
                    self.builder,
                    array_type,
                    CString::new("argv").unwrap().as_ptr(),
                );
                LLVMPositionBuilderAtEnd(self.builder, here);
                argv
            };

            for (position, handle) in handles.iter().enumerate() {
                let mut indices = [
                    LLVMConstInt(self.word(), 0, 0),
                    LLVMConstInt(self.word(), position as u64, 0),
                ];
                let slot = LLVMBuildGEP2(
                    self.builder,
                    array_type,
                    argv,
                    indices.as_mut_ptr(),
                    2,
                    CString::new("arg").unwrap().as_ptr(),
                );
                LLVMBuildStore(self.builder, *handle, slot);
            }

            let mut indices = [
                LLVMConstInt(self.word(), 0, 0),
                LLVMConstInt(self.word(), 0, 0),
            ];
            let first_arg = LLVMBuildGEP2(
                self.builder,
                array_type,
                argv,
                indices.as_mut_ptr(),
                2,
                CString::new("argv_first").unwrap().as_ptr(),
            );
            (first_arg, LLVMConstInt(self.value(), count as u64, 1))
        }
    }

    /// A function value naming `name` and carrying these handles.
    fn function_value(&mut self, name: &str, captured: &[LLVMValueRef]) -> LLVMValueRef {
        let label = match self.constant_text(name, "function") {
            Some(label) => label,
            None => return self.nil(),
        };
        let (argv, count) = self.argv(captured);
        unsafe {
            self.invoke(
                "etamil_function",
                vec![self.text(), LLVMPointerType(self.value(), 0), self.value()],
                self.value(),
                &mut [label, argv, count],
            )
        }
    }

    /// `செயல்(…) { … }` where a value goes: compiled as a function of its own
    /// whose leading parameters are the locals it captures, and made into a
    /// value carrying their current handles.
    ///
    /// What it captures is decided by `parser::captures` against the locals
    /// known at this point — the same function and the same set the bytecode
    /// compiler uses, which is what keeps the two backends agreeing on it.
    fn compile_lambda(&mut self, params: &[crate::parser::Param], body: &[Stmt]) -> LLVMValueRef {
        let captures = if self.in_function {
            let variables = &self.variables;
            crate::parser::captures(params, body, |name| variables.contains_key(name))
        } else {
            Vec::new()
        };
        let name = format!("#செயல்_{}", self.lambdas);
        self.lambdas += 1;

        let function = self.declare_function(&name, captures.len() + params.len());
        // Reached only through its value, never by a name from outside.
        unsafe { LLVMSetLinkage(function, LLVMLinkage::LLVMInternalLinkage) };
        self.compile_function(&name, &captures, params, body);

        let mut handles = Vec::with_capacity(captures.len());
        for capture in &captures {
            let slot = self.variables[capture];
            handles.push(unsafe {
                LLVMBuildLoad2(
                    self.builder,
                    self.value(),
                    slot,
                    CString::new("captured").unwrap().as_ptr(),
                )
            });
        }
        self.function_value(&name, &handles)
    }

    fn compile_stmt(&mut self, statement: Stmt) {
        unsafe {
            match statement {
                Stmt::Assign { name, value, .. } => {
                    let handle = self.compile_expr(&value);
                    let slot = self.storage_for(&name);
                    LLVMBuildStore(self.builder, handle, slot);
                }
                // Compiled up front in `compile`, with every other function.
                Stmt::FunctionDef { .. } | Stmt::ShapeDef { .. } => {}
                Stmt::Return(value) => {
                    if self.in_function {
                        let handle = match value.as_ref() {
                            Some(expr) => self.compile_expr(expr),
                            None => self.nil(),
                        };
                        LLVMBuildRet(self.builder, handle);
                        self.terminated = true;
                    } else {
                        self.unsupported.push("திரும்பு (return)".to_string());
                    }
                }
                Stmt::Print(expr) => {
                    // One call. `&` is a real operation now, so there is no
                    // printing of pieces and no format string to get wrong:
                    // the runtime renders through the same `to_string` the VM
                    // prints through.
                    let handle = self.compile_expr(&expr);
                    self.call_void("etamil_print", &mut [handle]);
                }
                Stmt::Input(expr) => {
                    // Exactly what the VM's bytecode does, which is less than
                    // it looks like it should: a line is always read, and it is
                    // stored only when the operand is a bare name. So
                    // `உள்ளிடு "prompt" & பெயர்` reads a line and throws it
                    // away, and no prompt is printed by either backend.
                    //
                    // This file used to print the prompt and, for a bare name,
                    // print the name's current value instead of reading at all.
                    // That was invented rather than read out of the VM, and it
                    // is what made examples/basic_samples/example.qmz the one
                    // program where the two backends disagreed: the compiled
                    // one printed `nil` and never consumed its input.
                    let line = self.invoke("etamil_read_line", vec![], self.value(), &mut []);
                    if let Expr::Variable(name) = &expr {
                        let slot = self.storage_for(name);
                        LLVMBuildStore(self.builder, line, slot);
                    }
                }
                // The runtime answers a new handle rather than changing the
                // value in place — see `etamil_index_set` — and it is stored
                // where an assignment to the name would go. Inside a function
                // that is a local even when the value was read from a global,
                // which is the VM's SetIndex exactly: it reads the name, changes
                // a copy, and assigns the copy. The base is read after the index
                // and the value, as the VM reads it after they are on its stack.
                Stmt::SetIndex {
                    name, index, value, ..
                } => {
                    match self.lookup(&name) {
                        Some(slot) => {
                            let position = self.compile_expr(&index);
                            let handle = self.compile_expr(&value);
                            let base = LLVMBuildLoad2(
                                self.builder,
                                self.value(),
                                slot,
                                CString::new("base").unwrap().as_ptr(),
                            );
                            // The name travels with the call only so that a
                            // base that cannot be indexed reports the same
                            // message the VM reports.
                            if let Some(label) = self.constant_text(&name, "name") {
                                let updated = self.invoke(
                                    "etamil_index_set",
                                    vec![self.value(), self.value(), self.value(), self.text()],
                                    self.value(),
                                    &mut [base, position, handle, label],
                                );
                                let target = self.storage_for(&name);
                                LLVMBuildStore(self.builder, updated, target);
                            }
                        }
                        None => self
                            .unsupported
                            .push(format!("the name {} (nothing here defines it)", name)),
                    }
                }
                Stmt::SetField {
                    name, field, value, ..
                } => match self.lookup(&name) {
                    Some(slot) => {
                        let handle = self.compile_expr(&value);
                        let base = LLVMBuildLoad2(
                            self.builder,
                            self.value(),
                            slot,
                            CString::new("base").unwrap().as_ptr(),
                        );
                        if let Some(key) = self.constant_text(&field, "field") {
                            let updated = self.invoke(
                                "etamil_field_set",
                                vec![self.value(), self.text(), self.value()],
                                self.value(),
                                &mut [base, key, handle],
                            );
                            let target = self.storage_for(&name);
                            LLVMBuildStore(self.builder, updated, target);
                        }
                    }
                    None => self
                        .unsupported
                        .push(format!("the name {} (nothing here defines it)", name)),
                },
                Stmt::Expression(expr) => {
                    // Evaluated for its effect. Discarding the handle is right:
                    // the arena keeps the value alive and nothing reads it.
                    self.compile_expr(&expr);
                }
                Stmt::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    let test = self.compile_condition(&condition);

                    let then_block = self.block("then");
                    let else_block = self.block("else");
                    let merge_block = self.block("merge");
                    LLVMBuildCondBr(self.builder, test, then_block, else_block);

                    // A branch that returns has already terminated its block,
                    // and a second terminator after `ret` is invalid IR.
                    LLVMPositionBuilderAtEnd(self.builder, then_block);
                    self.terminated = false;
                    for inner in then_branch {
                        if self.terminated {
                            break;
                        }
                        self.compile_stmt(inner);
                    }
                    let then_returned = self.terminated;
                    if !then_returned {
                        LLVMBuildBr(self.builder, merge_block);
                    }

                    LLVMPositionBuilderAtEnd(self.builder, else_block);
                    self.terminated = false;
                    if let Some(otherwise) = else_branch {
                        for inner in otherwise {
                            if self.terminated {
                                break;
                            }
                            self.compile_stmt(inner);
                        }
                    }
                    let else_returned = self.terminated;
                    if !else_returned {
                        LLVMBuildBr(self.builder, merge_block);
                    }

                    LLVMPositionBuilderAtEnd(self.builder, merge_block);
                    self.terminated = then_returned && else_returned;
                    if self.terminated {
                        // Nothing branches here, and a block still needs a
                        // terminator. `unreachable` is the honest one.
                        LLVMBuildUnreachable(self.builder);
                    }
                }
                Stmt::Loop { condition, body } => {
                    let test_block = self.block("loop_test");
                    let body_block = self.block("loop_body");
                    let after_block = self.block("loop_after");

                    LLVMBuildBr(self.builder, test_block);

                    LLVMPositionBuilderAtEnd(self.builder, test_block);
                    let test = self.compile_condition(&condition);
                    LLVMBuildCondBr(self.builder, test, body_block, after_block);

                    LLVMPositionBuilderAtEnd(self.builder, body_block);
                    self.terminated = false;
                    for inner in body {
                        if self.terminated {
                            break;
                        }
                        self.compile_stmt(inner);
                    }
                    if !self.terminated {
                        LLVMBuildBr(self.builder, test_block);
                    }

                    // Reached from the test however the body ended.
                    LLVMPositionBuilderAtEnd(self.builder, after_block);
                    self.terminated = false;
                }
                Stmt::ForEach {
                    var,
                    collection,
                    body,
                } => {
                    self.compile_for_each(&var, &collection, &body);
                }
                // `தளம்_இணை சீகுலைட், "file.db";` — borrow a connection.
                //
                // The driver is a compile-time word and the connection string
                // is an expression, which is why one crosses as a C string and
                // the other as a handle. An unnamed connection takes the
                // driver's name here, the same default the bytecode compiler
                // applies — so the two backends file it under the same key and
                // a later query with no name finds it either way.
                Stmt::DBConnect {
                    db_type,
                    connection_string,
                    handle,
                } => {
                    let target = self.compile_expr(&connection_string);
                    let name = handle.clone().unwrap_or_else(|| db_type.clone());
                    let driver = self.constant_text(&db_type, "driver");
                    let named = self.constant_text(&name, "connection");
                    if let (Some(driver), Some(named)) = (driver, named) {
                        self.invoke(
                            "etamil_db_connect",
                            vec![self.text(), self.value(), self.text()],
                            self.nothing(),
                            &mut [driver, target, named],
                        );
                    }
                }
                // `தளம்_செய் "sql", [params];` — a statement with no rows.
                //
                // The same shape as a query without the store: the row count
                // goes nowhere, which is what the VM does with it too.
                Stmt::DBExecute {
                    command,
                    params,
                    handle,
                } => {
                    let sql = self.compile_expr(&command);
                    let bound = self.compile_expr(&params);
                    let named = match &handle {
                        Some(name) => self.constant_text(name, "connection"),
                        None => Some(unsafe { LLVMConstNull(self.text()) }),
                    };
                    if let Some(named) = named {
                        self.invoke(
                            "etamil_db_execute",
                            vec![self.value(), self.value(), self.text()],
                            self.nothing(),
                            &mut [sql, bound, named],
                        );
                    }
                }
                // `தளம்_வினா "sql", [params], பெயர்;` — the rows, into a name.
                //
                // The one database statement this backend builds. The SQL and
                // the parameter list are ordinary expressions, so they compile
                // like any others; what is new is the connection, which is a C
                // string for a named one and a null pointer for "the only one
                // open" — the shape `Option<&str>` takes across the ABI.
                //
                // A compiled program cannot open a connection yet, so this
                // reports that there is none. It reports it with the VM's own
                // message, which is the only behaviour worth having until
                // தரவுசேமி_இணை is built too.
                Stmt::DBQuery {
                    query,
                    params,
                    result_var,
                    handle,
                } => {
                    let sql = self.compile_expr(&query);
                    let bound = self.compile_expr(&params);
                    let named = match &handle {
                        Some(name) => self.constant_text(name, "connection"),
                        None => Some(unsafe { LLVMConstNull(self.text()) }),
                    };
                    if let Some(named) = named {
                        let rows = self.invoke(
                            "etamil_db_query",
                            vec![self.value(), self.value(), self.text()],
                            self.value(),
                            &mut [sql, bound, named],
                        );
                        let slot = self.storage_for(&result_var);
                        unsafe {
                            LLVMBuildStore(self.builder, rows, slot);
                        }
                    }
                }
                other => {
                    // Recording rather than ignoring: a statement this backend
                    // drops would make the compiled program quietly disagree
                    // with the same source on the VM.
                    self.unsupported
                        .push(format!("statement {}", stmt_label(&other)));
                }
            }
        }
    }

    fn block(&self, label: &str) -> LLVMBasicBlockRef {
        unsafe {
            LLVMAppendBasicBlockInContext(
                self.context,
                self.function,
                CString::new(label).unwrap().as_ptr(),
            )
        }
    }

    /// The `i1` that `எனில்` and `சுற்று` branch on.
    ///
    /// One rule for every expression: truthiness, as the runtime decides it,
    /// which is `Value::is_truthy` — so a number is true when it is not zero,
    /// checked against the VM rather than assumed.
    fn compile_condition(&mut self, expr: &Expr) -> LLVMValueRef {
        let handle = self.compile_expr(expr);
        let truthy = self.invoke(
            "etamil_truthy",
            vec![self.value()],
            self.word(),
            &mut [handle],
        );
        unsafe {
            LLVMBuildICmp(
                self.builder,
                LLVMIntPredicate::LLVMIntNE,
                truthy,
                LLVMConstInt(self.word(), 0, 0),
                CString::new("condition").unwrap().as_ptr(),
            )
        }
    }

    /// `ஒவ்வொரு x இல் xs` — a counted loop over the runtime's own count.
    fn compile_for_each(&mut self, variable: &str, collection: &Expr, body: &[Stmt]) {
        unsafe {
            let items = self.compile_expr(collection);
            let count = self.invoke(
                "etamil_count",
                vec![self.value()],
                self.value(),
                &mut [items],
            );

            let position = self.entry_slot("each_position");
            LLVMBuildStore(self.builder, LLVMConstInt(self.value(), 0, 1), position);

            let test_block = self.block("each_test");
            let body_block = self.block("each_body");
            let after_block = self.block("each_after");
            LLVMBuildBr(self.builder, test_block);

            LLVMPositionBuilderAtEnd(self.builder, test_block);
            let at = LLVMBuildLoad2(
                self.builder,
                self.value(),
                position,
                CString::new("at").unwrap().as_ptr(),
            );
            let more = LLVMBuildICmp(
                self.builder,
                LLVMIntPredicate::LLVMIntSLT,
                at,
                count,
                CString::new("more").unwrap().as_ptr(),
            );
            LLVMBuildCondBr(self.builder, more, body_block, after_block);

            LLVMPositionBuilderAtEnd(self.builder, body_block);
            let at = LLVMBuildLoad2(
                self.builder,
                self.value(),
                position,
                CString::new("at").unwrap().as_ptr(),
            );
            // `nth_or_key`, not indexing: a record yields its sorted keys and a
            // string yields one Tamil letter, which is what the VM's `ஒவ்வொரு`
            // binds. Indexing a record with a number would just fail.
            let item = self.call_values("etamil_nth_or_key", &mut [items, at]);
            let slot = self.storage_for(variable);
            LLVMBuildStore(self.builder, item, slot);

            self.terminated = false;
            for inner in body {
                if self.terminated {
                    break;
                }
                self.compile_stmt(inner.clone());
            }
            if !self.terminated {
                let at = LLVMBuildLoad2(
                    self.builder,
                    self.value(),
                    position,
                    CString::new("at").unwrap().as_ptr(),
                );
                let next = LLVMBuildAdd(
                    self.builder,
                    at,
                    LLVMConstInt(self.value(), 1, 1),
                    CString::new("next").unwrap().as_ptr(),
                );
                LLVMBuildStore(self.builder, next, position);
                LLVMBuildBr(self.builder, test_block);
            }

            LLVMPositionBuilderAtEnd(self.builder, after_block);
            self.terminated = false;
        }
    }

    fn llvm_function_type(&self, parameter_count: usize) -> LLVMTypeRef {
        unsafe {
            let mut params = vec![self.value(); parameter_count];
            LLVMFunctionType(self.value(), params.as_mut_ptr(), params.len() as u32, 0)
        }
    }

    fn declare_function(&mut self, name: &str, parameter_count: usize) -> LLVMValueRef {
        if let Some(function) = self.functions.get(name).copied() {
            return function;
        }
        unsafe {
            let function = LLVMAddFunction(
                self.module,
                CString::new(name)
                    .unwrap_or_else(|_| CString::new("fn").unwrap())
                    .as_ptr(),
                self.llvm_function_type(parameter_count),
            );
            self.functions.insert(name.to_string(), function);
            function
        }
    }

    /// A function's body. `captures` lead its parameters: a named `செயல்` has
    /// none, and one written as a value has the locals it carries.
    fn compile_function(
        &mut self,
        name: &str,
        captures: &[String],
        params: &[crate::parser::Param],
        body: &[Stmt],
    ) {
        let params: Vec<crate::parser::Param> = captures
            .iter()
            .map(|capture| crate::parser::Param {
                name: capture.clone(),
                declared: None,
                immutable: false,
                at: crate::parser::Position { line: 0, column: 0 },
            })
            .chain(params.iter().cloned())
            .collect();
        self.compiled.push((
            name.to_string(),
            captures.len(),
            params.len() - captures.len(),
        ));
        let params = params.as_slice();
        unsafe {
            let function = self.declare_function(name, params.len());
            let saved_function = self.function;
            let saved_block = LLVMGetInsertBlock(self.builder);
            let saved_variables = std::mem::take(&mut self.variables);
            let saved_in_function = self.in_function;
            let saved_terminated = self.terminated;

            self.function = function;
            self.in_function = true;
            self.terminated = false;
            let entry = LLVMAppendBasicBlockInContext(
                self.context,
                function,
                CString::new("entry").unwrap().as_ptr(),
            );
            LLVMPositionBuilderAtEnd(self.builder, entry);

            for (index, parameter) in params.iter().enumerate() {
                let slot = LLVMBuildAlloca(
                    self.builder,
                    self.value(),
                    CString::new(parameter.name.as_str())
                        .unwrap_or_else(|_| CString::new("param").unwrap())
                        .as_ptr(),
                );
                LLVMBuildStore(self.builder, LLVMGetParam(function, index as u32), slot);
                self.variables.insert(parameter.name.clone(), slot);
            }

            for statement in body {
                if self.terminated {
                    break;
                }
                self.compile_stmt(statement.clone());
            }

            if !self.terminated {
                // A function that runs off its end answers இன்மை, which is
                // what the VM's frame does.
                let nil = self.nil();
                LLVMBuildRet(self.builder, nil);
            }

            self.function = saved_function;
            self.variables = saved_variables;
            self.in_function = saved_in_function;
            self.terminated = saved_terminated;
            LLVMPositionBuilderAtEnd(self.builder, saved_block);
        }
    }

    fn compile_expr(&mut self, expr: &Expr) -> LLVMValueRef {
        match expr {
            // The decimal's own text, parsed back by the runtime. Exact: a
            // `Decimal` written out and read in is the same value, where a
            // double would lose the thing the runtime exists to keep.
            Expr::Number(number) => match self.constant_text(&number.to_string(), "number") {
                Some(literal) => self.invoke(
                    "etamil_number",
                    vec![self.text()],
                    self.value(),
                    &mut [literal],
                ),
                None => self.nil(),
            },
            Expr::String(body) => match self.constant_text(body, "text") {
                Some(literal) => self.invoke(
                    "etamil_text",
                    vec![self.text()],
                    self.value(),
                    &mut [literal],
                ),
                None => self.nil(),
            },
            Expr::Boolean(flag) => unsafe {
                let constant = LLVMConstInt(self.word(), u64::from(*flag), 0);
                self.invoke(
                    "etamil_boolean",
                    vec![self.word()],
                    self.value(),
                    &mut [constant],
                )
            },
            Expr::Null => self.nil(),
            Expr::Variable(name) => match self.lookup(name) {
                Some(slot) => unsafe {
                    LLVMBuildLoad2(
                        self.builder,
                        self.value(),
                        slot,
                        CString::new("load").unwrap().as_ptr(),
                    )
                },
                // No variable, but a function — the program's own or a
                // builtin — and so that function as a value: `ச = இரட்டி;`.
                None if self.functions.contains_key(name) || crate::vm::is_builtin(name) => {
                    self.function_value(name, &[])
                }
                None => {
                    self.unsupported
                        .push(format!("the name {} (nothing here defines it)", name));
                    self.nil()
                }
            },
            Expr::BinaryOp { op, left, right } => {
                let lhs = self.compile_expr(left);
                let rhs = self.compile_expr(right);
                let runtime = match op.as_str() {
                    "+" => "etamil_add",
                    "-" => "etamil_subtract",
                    "*" => "etamil_multiply",
                    "/" => "etamil_divide",
                    other => {
                        // The parser builds only + - * / today. Recording
                        // rather than answering something keeps that true of
                        // tomorrow's parser.
                        self.unsupported.push(format!("the operator {}", other));
                        return self.nil();
                    }
                };
                self.call_values(runtime, &mut [lhs, rhs])
            }
            Expr::Comparison { left, op, right } => {
                let lhs = self.compile_expr(left);
                let rhs = self.compile_expr(right);
                // Codes fixed in runtime.rs; ordering goes through
                // `Value::partial_cmp` there, so arrays and records compare as
                // they do on the VM.
                let code = match op.as_str() {
                    "<" => 0,
                    "<=" => 1,
                    ">" => 2,
                    ">=" => 3,
                    "==" => 4,
                    "!=" => 5,
                    other => {
                        self.unsupported.push(format!("the comparison {}", other));
                        return self.nil();
                    }
                };
                unsafe {
                    let code = LLVMConstInt(self.word(), code as u64, 0);
                    self.invoke(
                        "etamil_compare",
                        vec![self.value(), self.value(), self.word()],
                        self.value(),
                        &mut [lhs, rhs, code],
                    )
                }
            }
            Expr::Concat { left, right } => {
                let lhs = self.compile_expr(left);
                let rhs = self.compile_expr(right);
                self.call_values("etamil_concat", &mut [lhs, rhs])
            }
            Expr::Logical { op, left, right } => self.compile_logical(op, left, right),
            Expr::Not(inner) => {
                let handle = self.compile_expr(inner);
                self.call_values("etamil_not", &mut [handle])
            }
            Expr::ArrayLiteral(items) => {
                let array = self.invoke("etamil_array", vec![], self.value(), &mut []);
                for item in items {
                    let handle = self.compile_expr(item);
                    self.call_void("etamil_array_push", &mut [array, handle]);
                }
                array
            }
            Expr::RecordLiteral(fields) => {
                let record = self.invoke("etamil_record", vec![], self.value(), &mut []);
                for (field, value) in fields {
                    let handle = self.compile_expr(value);
                    if let Some(key) = self.constant_text(field, "field") {
                        self.invoke(
                            "etamil_record_put",
                            vec![self.value(), self.text(), self.value()],
                            self.nothing(),
                            &mut [record, key, handle],
                        );
                    }
                }
                record
            }
            Expr::Index { base, index } => {
                let base = self.compile_expr(base);
                let position = self.compile_expr(index);
                self.call_values("etamil_index", &mut [base, position])
            }
            Expr::Field { base, name, .. } => {
                let base = self.compile_expr(base);
                match self.constant_text(name, "field") {
                    Some(key) => self.invoke(
                        "etamil_field",
                        vec![self.value(), self.text()],
                        self.value(),
                        &mut [base, key],
                    ),
                    None => self.nil(),
                }
            }
            Expr::Call { name, args } => self.compile_call(name, args),
            Expr::Try(inner) => self.compile_try(inner),
            Expr::Lambda { params, body, .. } => self.compile_lambda(params, body),
            Expr::ShapeLiteral {
                shape,
                fields,
                base,
                ..
            } => self.compile_shape_literal(shape, fields, base.as_deref()),
            Expr::MethodCall {
                receiver,
                name,
                args,
                ..
            } => self.compile_method_call(receiver, name, args),
            Expr::CallValue { callee, args } => {
                let callee = self.compile_expr(callee);
                let handles: Vec<LLVMValueRef> =
                    args.iter().map(|arg| self.compile_expr(arg)).collect();
                let (argv, argc) = self.argv(&handles);
                unsafe {
                    self.invoke(
                        "etamil_call_value",
                        vec![self.value(), LLVMPointerType(self.value(), 0), self.value()],
                        self.value(),
                        &mut [callee, argv, argc],
                    )
                }
            }
        }
    }

    /// `மற்றும்` and `அல்லது`, short-circuiting.
    ///
    /// The right side is not evaluated once the left has decided the answer,
    /// which the VM's bytecode does with jumps rather than with its `And`
    /// instruction — because a guard has to be able to guard:
    ///
    /// ```text
    /// (நீளம்(அ) > 0 மற்றும் அ[0] == 1)
    /// ```
    ///
    /// would index an empty array on the very step that proved it was empty.
    /// The answer is a Boolean either way, so both arms go through
    /// `etamil_boolean` on the operand's truthiness rather than yielding the
    /// operand.
    fn compile_logical(&mut self, op: &str, left: &Expr, right: &Expr) -> LLVMValueRef {
        let stops_on = match op {
            "&&" => false,
            "||" => true,
            other => {
                self.unsupported
                    .push(format!("the logical operator {}", other));
                return self.nil();
            }
        };

        unsafe {
            let answer = self.entry_slot("logical");
            let decided = self.compile_condition(left);

            let right_block = self.block("logical_right");
            let short_block = self.block("logical_short");
            let done_block = self.block("logical_done");

            // For `&&` a true left means the right side decides; for `||` a
            // true left is already the answer.
            if stops_on {
                LLVMBuildCondBr(self.builder, decided, short_block, right_block);
            } else {
                LLVMBuildCondBr(self.builder, decided, right_block, short_block);
            }

            LLVMPositionBuilderAtEnd(self.builder, short_block);
            let constant = LLVMConstInt(self.word(), u64::from(stops_on), 0);
            let short = self.invoke(
                "etamil_boolean",
                vec![self.word()],
                self.value(),
                &mut [constant],
            );
            LLVMBuildStore(self.builder, short, answer);
            LLVMBuildBr(self.builder, done_block);

            LLVMPositionBuilderAtEnd(self.builder, right_block);
            let decided = self.compile_condition(right);
            let widened = LLVMBuildZExt(
                self.builder,
                decided,
                self.word(),
                CString::new("as_word").unwrap().as_ptr(),
            );
            let from_right = self.invoke(
                "etamil_boolean",
                vec![self.word()],
                self.value(),
                &mut [widened],
            );
            LLVMBuildStore(self.builder, from_right, answer);
            LLVMBuildBr(self.builder, done_block);

            LLVMPositionBuilderAtEnd(self.builder, done_block);
            LLVMBuildLoad2(
                self.builder,
                self.value(),
                answer,
                CString::new("logical_value").unwrap().as_ptr(),
            )
        }
    }

    /// A call: the author's `செயல்` first, then any builtin at all through
    /// the runtime, which dispatches them with the interpreter's own table.
    /// That order is the VM's order.
    fn compile_call(&mut self, name: &str, args: &[Expr]) -> LLVMValueRef {
        // `கடன்(பதிவு)` — a shape's name called makes a record of that shape
        // out of a plain one, as a result, through `vm::shape::convert`.
        if self.shapes.contains_key(name) && !self.functions.contains_key(name) && args.len() == 1 {
            let record = self.compile_expr(&args[0]);
            return match self.constant_text(name, "shape") {
                Some(label) => self.invoke(
                    "etamil_shape_convert",
                    vec![self.text(), self.value()],
                    self.value(),
                    &mut [label, record],
                ),
                None => self.nil(),
            };
        }

        // A variable of this name may hold a function value, and if it does
        // it is what is called — a parameter called by its name. Only the
        // runtime can see what it holds, so it decides, the way the VM's
        // `Call` does. The arguments go first, as they do on the VM's stack.
        if let Some(slot) = self.lookup(name) {
            let handles: Vec<LLVMValueRef> =
                args.iter().map(|arg| self.compile_expr(arg)).collect();
            let label = match self.constant_text(name, "builtin") {
                Some(label) => label,
                None => return self.nil(),
            };
            let (argv, argc) = self.argv(&handles);
            return unsafe {
                let held = LLVMBuildLoad2(
                    self.builder,
                    self.value(),
                    slot,
                    CString::new("held").unwrap().as_ptr(),
                );
                self.invoke(
                    "etamil_call_named",
                    vec![
                        self.value(),
                        self.text(),
                        LLVMPointerType(self.value(), 0),
                        self.value(),
                    ],
                    self.value(),
                    &mut [held, label, argv, argc],
                )
            };
        }

        if let Some(function) = self.functions.get(name).copied() {
            let mut handles: Vec<LLVMValueRef> =
                args.iter().map(|arg| self.compile_expr(arg)).collect();
            return unsafe {
                LLVMBuildCall2(
                    self.builder,
                    self.llvm_function_type(handles.len()),
                    function,
                    handles.as_mut_ptr(),
                    handles.len() as u32,
                    CString::new("call").unwrap().as_ptr(),
                )
            };
        }

        // A builtin. The handles go into a stack array and the runtime reads
        // them back, which is how a variadic call crosses the C ABI without
        // this file knowing anything about any particular builtin.
        let handles: Vec<LLVMValueRef> = args.iter().map(|arg| self.compile_expr(arg)).collect();
        let label = match self.constant_text(name, "builtin") {
            Some(label) => label,
            None => return self.nil(),
        };

        let (first_arg, argc) = self.argv(&handles);
        unsafe {
            self.invoke(
                "etamil_call",
                vec![self.text(), LLVMPointerType(self.value(), 0), self.value()],
                self.value(),
                &mut [label, first_arg, argc],
            )
        }
    }

    /// `?` — hand a `தவறு` to the caller, unchanged, and unwrap a `சரி`.
    ///
    /// The branch is here rather than in the runtime because propagating means
    /// returning from *this* function, which only the IR can do.
    fn compile_try(&mut self, inner: &Expr) -> LLVMValueRef {
        let handle = self.compile_expr(inner);

        let failed = self.invoke(
            "etamil_is_error",
            vec![self.value()],
            self.word(),
            &mut [handle],
        );

        unsafe {
            let test = LLVMBuildICmp(
                self.builder,
                LLVMIntPredicate::LLVMIntNE,
                failed,
                LLVMConstInt(self.word(), 0, 0),
                CString::new("failed").unwrap().as_ptr(),
            );
            let propagate_block = self.block("propagate");
            let carry_on_block = self.block("carry_on");
            LLVMBuildCondBr(self.builder, test, propagate_block, carry_on_block);

            LLVMPositionBuilderAtEnd(self.builder, propagate_block);
            if self.in_function {
                LLVMBuildRet(self.builder, handle);
            } else {
                // The VM stops with "கையாளப்படாத தவறு" at top level, because
                // there is no caller to hand it to.
                self.invoke(
                    "etamil_unhandled",
                    vec![self.value()],
                    self.nothing(),
                    &mut [handle],
                );
                LLVMBuildUnreachable(self.builder);
            }

            LLVMPositionBuilderAtEnd(self.builder, carry_on_block);
            self.call_values("etamil_unwrap", &mut [handle])
        }
    }

    /// Emit LLVM IR to a file
    pub fn emit_ir(&self, filename: &str) -> Result<(), String> {
        let c_filename = CString::new(filename).map_err(|e| e.to_string())?;
        unsafe {
            let mut error: *mut i8 = std::ptr::null_mut();
            let success = LLVMPrintModuleToFile(self.module, c_filename.as_ptr(), &mut error);

            if success != 0 {
                let message = if !error.is_null() {
                    std::ffi::CStr::from_ptr(error)
                        .to_string_lossy()
                        .to_string()
                } else {
                    "Unknown LLVM error".to_string()
                };
                LLVMDisposeMessage(error);
                return Err(message);
            }
        }
        Ok(())
    }

    /// Print the module to stderr for debugging
    pub fn dump_module(&self) {
        unsafe {
            LLVMDumpModule(self.module);
        }
    }
}

// Placeholder implementations for non-LLVM builds (e.g., Windows without LLVM)
//
// Both impls need the cfg. `clippy --fix` added the Default impl directly under
// the attribute, which silently moved the guard off the impl below it — so a
// build with --features llvm had two `impl Compiler` blocks and stopped
// compiling. Nothing but the llvm job builds this file, so nothing else notices.
#[cfg(not(feature = "llvm"))]
impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "llvm"))]
impl Compiler {
    pub fn new() -> Self {
        Compiler {}
    }

    pub fn compile(&mut self, _statements: Vec<Stmt>) {
        eprintln!("WARNING: LLVM code generation is not available on this platform.");
        eprintln!("Please use --vm flag or install LLVM to use --llvm flag.");
    }

    pub fn emit_ir(&self, _filename: &str) -> Result<(), String> {
        Err(
            "LLVM code generation is not available on this platform. Use --vm flag instead."
                .to_string(),
        )
    }

    pub fn dump_module(&self) {
        eprintln!("LLVM module dumping is not available on this platform.");
    }
}

#[cfg(feature = "llvm")]
impl Drop for Compiler {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeBuilder(self.builder);
            LLVMDisposeModule(self.module);
            LLVMContextDispose(self.context);
        }
    }
}

// --- What the backend refuses, countable without building it ----------------
//
// These two are deliberately outside `#[cfg(feature = "llvm")]`. The list of
// what this backend cannot build is a pure function of the AST, and keeping it
// behind the feature gate meant the gap could only be measured on a machine
// with LLVM 18 — which is the one measurement you want *before* going to that
// machine. `etamil --llvm-gaps` walks a program with these and prints what
// would be refused, on any platform.
//
// `builds` is the same list of statement kinds `Compiler::compile_stmt` handles
// explicitly. **Adding an arm there means adding it here**, or the report will
// claim a refusal the backend no longer makes.

/// Every name the top level binds, at any depth short of a function body.
#[cfg(feature = "llvm")]
fn top_level_bindings(statements: &[Stmt], into: &mut Vec<String>) {
    for statement in statements {
        into.extend(
            crate::parser::bound_names(statement)
                .into_iter()
                .map(str::to_string),
        );
        match statement {
            Stmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                top_level_bindings(then_branch, into);
                if let Some(otherwise) = else_branch {
                    top_level_bindings(otherwise, into);
                }
            }
            Stmt::Loop { body, .. } | Stmt::ForEach { body, .. } => top_level_bindings(body, into),
            _ => {}
        }
    }
}

/// Does the LLVM backend build this kind of statement?
pub fn builds(statement: &Stmt) -> bool {
    matches!(
        statement,
        Stmt::Assign { .. }
            | Stmt::FunctionDef { .. }
            | Stmt::ShapeDef { .. }
            | Stmt::Return(_)
            | Stmt::Print(_)
            | Stmt::Input(_)
            | Stmt::SetIndex { .. }
            | Stmt::SetField { .. }
            | Stmt::Expression(_)
            | Stmt::If { .. }
            | Stmt::Loop { .. }
            | Stmt::ForEach { .. }
            | Stmt::DBQuery { .. }
            | Stmt::DBConnect { .. }
            | Stmt::DBExecute { .. }
    )
}

/// What to call a statement this backend will not build. This is the roadmap
/// `scripts/run_parity.sh` ranks, so naming the statement is worth more than
/// naming its category.
pub fn stmt_label(statement: &Stmt) -> &'static str {
    match statement {
        Stmt::Import(_) => "இறக்கு (import)",
        // Files
        Stmt::FileOpen { .. } => "கோப்பு_திற (open a file)",
        Stmt::FileClose { .. } => "கோப்பு_மூடு (close a file)",
        Stmt::FileRead { .. } => "கோப்பு_படி (read a file)",
        Stmt::FileWrite { .. } => "கோப்பு_எழுது (write a file)",
        Stmt::ReadCSV { .. } => "CSV_படி (read a CSV)",
        Stmt::WriteCSV { .. } => "CSV_எழுது (write a CSV)",
        // Database
        Stmt::DBConnect { .. } => "தரவுசேமி_இணை (connect to a database)",
        Stmt::DBDisconnect { .. } => "தரவுசேமி_பிரி (disconnect)",
        Stmt::DBQuery { .. } => "தளம்_வினா (query)",
        Stmt::DBExecute { .. } => "தளம்_செய் (execute)",
        Stmt::DBInsert { .. } => "தளம்_நுழை (insert)",
        Stmt::DBUpdate { .. } => "தளம்_புதுப்பி (update)",
        Stmt::DBDelete { .. } => "தளம்_நீக்கு (delete)",
        Stmt::CreateTable { .. } => "அட்டவணை_உருவாக்கு (create a table)",
        Stmt::Select { .. } => "தேர்ந்தெடு (select)",
        // Server
        Stmt::DefineRoute { .. } => "வழி (a route)",
        // Lifted out of the program at startup like வழி, and refused here for
        // the same reason: it needs a server to run inside.
        Stmt::Schedule { .. } => "இடைவெளி (a scheduled block)",
        Stmt::StartServer { .. } => "சேவையகம்_தொடங்கு (start a server)",
        Stmt::StopServer => "சேவையகம்_நிறுத்து (stop a server)",
        Stmt::SendResponse { .. } => "பதில்_அனுப்பு (send a response)",
        Stmt::SendJSON { .. } => "ஜேசான்_உரை (send JSON)",
        Stmt::GetRequestBody { .. } => "வேண்டுகோள்_உடல் (the request body)",
        Stmt::GetRequestParam { .. } => "வேண்டுகோள்_அளபுரு (a request parameter)",
        Stmt::GetHeader { .. } => "தலைப்பு_பெறு (read a header)",
        Stmt::SetHeader { .. } => "தலைப்பு_அமை (set a header)",
        _ => "a statement this backend does not build",
    }
}

/// Every statement in this program the backend would refuse, in source order.
///
/// Duplicates are kept: a program that opens four files is four refusals of
/// one kind, and the ranking wants both numbers. Nested bodies are walked,
/// because the backend compiles them and refuses inside them.
pub fn refusals(statements: &[Stmt]) -> Vec<&'static str> {
    let mut found = Vec::new();
    collect(statements, &mut found);
    found
}

fn collect(statements: &[Stmt], found: &mut Vec<&'static str>) {
    for statement in statements {
        if !builds(statement) {
            found.push(stmt_label(statement));
            continue;
        }
        match statement {
            Stmt::FunctionDef { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::ForEach { body, .. } => collect(body, found),
            Stmt::ShapeDef { methods, .. } => {
                for method in methods {
                    collect(&method.body, found);
                }
            }
            Stmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                collect(then_branch, found);
                if let Some(other) = else_branch {
                    collect(other, found);
                }
            }
            _ => {}
        }
    }
}
