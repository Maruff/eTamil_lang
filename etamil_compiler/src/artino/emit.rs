// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! The analysed program, lowered to LLVM for one board, written as an object.
//!
//! Values are what `analyse` decided, and each has a size fixed now:
//!
//! | eTamil | held as |
//! |---|---|
//! | number | `i64`, the value × 1000 |
//! | boolean | `i1` |
//! | text | a `[49 x i8]` buffer, NUL-terminated UTF-8; passed as a pointer |
//! | array | an `[n x i64]` or `[n x i1]`; passed as a pointer |
//! | result | `{ i1 ok, [49 x i8] payload }`: the value when சரி, the error text when தவறு; passed as a pointer |
//! | வடிவம் record | an LLVM struct of its fields, in declared order; passed as a pointer |
//!
//! Buffers the program names are globals or locals. Buffers an expression
//! needs on the way — a `&` chain, a literal used as a value, an array literal
//! — are allocas in the function's entry block, made once per call, never
//! inside a loop. Assigning text or an array copies it, which is what the VM's
//! value semantics mean.
//!
//! Comparison is a single instruction. Arithmetic goes through the runtime
//! (`artino_num_add` … `artino_num_div`), because each operation can come out
//! differently from the VM — an overflow, a fourth decimal, a division by zero
//! — and when it does the board must say so rather than wrap or truncate in
//! silence. Each operation carries a *site*: a flag and a description of where
//! it is, so the runtime reports it once. Text that is cut to fit its buffer,
//! and an index outside its array, are reported the same way. Sums of two
//! literals, which is how `-5` arrives, are folded here and cost nothing.
//!
//! Text from the program — literals and site descriptions — lives in flash on
//! AVR (`.progmem.data`), where an Uno has 32 KB, not in its 2 KB of RAM; the
//! runtime reads it back as flash.
//!
//! What the board calls:
//!
//! - `artino_setup` — the top-level statements, once;
//! - `artino_loop` — each `இடைவெளி` block whose period has passed, by a
//!   wrap-safe `millis()` comparison, then `சுழற்சி` if the program has one.
//!
//! Every other symbol is internal and named with a dot (`et.f.…`, `et.g.…`),
//! which no source identifier can contain, so nothing a program names can
//! collide with the runtime or the Arduino core.

use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;

use llvm_sys::analysis::{LLVMVerifierFailureAction, LLVMVerifyModule};
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use llvm_sys::target::*;
use llvm_sys::target_machine::*;
use llvm_sys::transforms::pass_builder::*;
use llvm_sys::{LLVMIntPredicate, LLVMLinkage};

use super::Board;
use super::analyse::{
    self, Arg, Builtin, Elem, Inner, LETTER_BYTES, Program, Ret, ShapeInfo, TEXT_BYTES, Ty,
};
use crate::parser::{Expr, Stmt};

fn c(text: &str) -> CString {
    CString::new(text).expect("no NUL in an LLVM name")
}

/// An LLVM-allocated message as a `String`, disposed of. Null is "".
unsafe fn take_message(message: *mut std::os::raw::c_char) -> String {
    if message.is_null() {
        return String::new();
    }
    unsafe {
        let text = CStr::from_ptr(message).to_string_lossy().into_owned();
        LLVMDisposeMessage(message);
        text
    }
}

/// Lower `program` for `board` and write an object file to `object`. When
/// `ir` is given, the optimised IR is written there too, for reading.
pub use super::sketch::Compiled;

pub fn compile(
    program: &Program,
    board: &Board,
    object: &Path,
    ir: Option<&Path>,
) -> Result<Compiled, String> {
    unsafe {
        LLVM_InitializeAllTargetInfos();
        LLVM_InitializeAllTargets();
        LLVM_InitializeAllTargetMCs();
        LLVM_InitializeAllAsmPrinters();

        // The host board is this machine, for the conformance suite.
        let host = board.triple.is_empty();
        let triple = if host {
            let default = LLVMGetDefaultTargetTriple();
            let text = take_message(default);
            c(&text)
        } else {
            c(board.triple)
        };
        let mut target: LLVMTargetRef = ptr::null_mut();
        let mut message = ptr::null_mut();
        if LLVMGetTargetFromTriple(triple.as_ptr(), &mut target, &mut message) != 0 {
            return Err(format!(
                "this LLVM has no {} target: {}",
                triple.to_string_lossy(),
                take_message(message)
            ));
        }
        let machine = LLVMCreateTargetMachine(
            target,
            triple.as_ptr(),
            c(board.cpu).as_ptr(),
            c("").as_ptr(),
            LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
            // A board image is linked at fixed addresses; a host executable
            // today is position-independent.
            if host {
                LLVMRelocMode::LLVMRelocPIC
            } else {
                LLVMRelocMode::LLVMRelocStatic
            },
            LLVMCodeModel::LLVMCodeModelDefault,
        );

        let context = LLVMContextCreate();
        let module = LLVMModuleCreateWithNameInContext(c("artino").as_ptr(), context);
        LLVMSetTarget(module, triple.as_ptr());
        // Before any function exists: on AVR, code lives in address space 1,
        // and a function takes its address space from the module's layout.
        let layout = LLVMCreateTargetDataLayout(machine);
        LLVMSetModuleDataLayout(module, layout);

        // host-small is the host, built another way: பலகை() answers "host" on both.
        let name = if board.name == "host-small" {
            "host"
        } else {
            board.name
        };
        let mut emitter = Emitter::new(context, module, layout, board.triple == "avr", name);
        emitter.flash = board.flash_constants;
        emitter.line_reports = board.line_reports;
        let result = emitter.program(program);

        let outcome = result.and_then(|_| {
            let mut message = ptr::null_mut();
            if LLVMVerifyModule(
                module,
                LLVMVerifierFailureAction::LLVMReturnStatusAction,
                &mut message,
            ) != 0
            {
                return Err(format!(
                    "artino built IR LLVM rejects: {}",
                    take_message(message)
                ));
            }
            take_message(message);

            // Size first: an Uno has 32 KB of flash.
            let options = LLVMCreatePassBuilderOptions();
            let error = LLVMRunPasses(module, c("default<Os>").as_ptr(), machine, options);
            LLVMDisposePassBuilderOptions(options);
            if !error.is_null() {
                let text = llvm_sys::error::LLVMGetErrorMessage(error);
                let message = CStr::from_ptr(text).to_string_lossy().into_owned();
                llvm_sys::error::LLVMDisposeErrorMessage(text);
                return Err(format!("optimising: {}", message));
            }

            if let Some(ir) = ir {
                let mut message = ptr::null_mut();
                let path = c(&ir.to_string_lossy());
                if LLVMPrintModuleToFile(module, path.as_ptr(), &mut message) != 0 {
                    return Err(format!(
                        "writing {}: {}",
                        ir.display(),
                        take_message(message)
                    ));
                }
            }

            let mut message = ptr::null_mut();
            let path = c(&object.to_string_lossy());
            if LLVMTargetMachineEmitToFile(
                machine,
                module,
                path.as_ptr() as *mut _,
                LLVMCodeGenFileType::LLVMObjectFile,
                &mut message,
            ) != 0
            {
                return Err(format!(
                    "writing {}: {}",
                    object.display(),
                    take_message(message)
                ));
            }
            let used = |name: &str| {
                let function = LLVMGetNamedFunction(module, c(name).as_ptr());
                !function.is_null() && !LLVMGetFirstUse(function).is_null()
            };
            Ok(Compiled {
                sites: std::mem::take(&mut emitter.site_texts),
                tone: used("artino_tone") || used("artino_no_tone"),
            })
        });

        LLVMDisposeBuilder(emitter.builder);
        LLVMDisposeModule(module);
        LLVMDisposeTargetData(layout);
        LLVMDisposeTargetMachine(machine);
        LLVMContextDispose(context);
        outcome
    }
}

struct Emitter {
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    layout: llvm_sys::target::LLVMTargetDataRef,
    /// Runtime and shim functions, by C name.
    runtime: HashMap<&'static str, (LLVMValueRef, LLVMTypeRef)>,
    globals: HashMap<String, (LLVMValueRef, Ty)>,
    /// A function's value, its LLVM type, what it returns, and whether it
    /// returns text through a buffer its caller passes first.
    functions: HashMap<String, (LLVMValueRef, LLVMTypeRef, Ty)>,
    /// The current function's locals and parameters; empty at the top level.
    locals: HashMap<String, (LLVMValueRef, Ty)>,
    current: LLVMValueRef,
    /// The current function's return type, or `None` at the top level.
    returns: Option<Ty>,
    /// Where a function that returns text writes it: its first parameter.
    out: Option<LLVMValueRef>,
    /// Put program text in `.progmem.data`: AVR, where flash is a separate space.
    /// Place constant data in `.progmem.data`, AVR's flash.
    avr: bool,
    /// Read `நிலை` texts and arrays by copying them out (`Board::flash_constants`).
    flash: bool,
    /// Program text already in the module, so one literal is stored once.
    texts: HashMap<String, LLVMValueRef>,
    /// Where lowering is, for site descriptions: a செயல், a block, or the start.
    place: String,
    /// The nearest source line the AST knows for what is being lowered.
    line: Option<usize>,
    /// `நிலை` texts and arrays whose value is known now: a constant global,
    /// in flash on AVR, never written, so it costs no RAM until it is read.
    flash_constants: HashSet<String>,
    /// The manifest's C++ functions, and each one's shim once declared.
    externs: Vec<super::manifest::Extern>,
    extern_shims: HashMap<usize, (LLVMValueRef, LLVMTypeRef)>,
    /// Each report site's full text, by number.
    site_texts: Vec<String>,
    /// Sites hold their line only, not their text (`Board::line_reports`).
    line_reports: bool,
    /// For each statement being lowered, innermost last: the block before it,
    /// where its temporaries' lifetimes start, and those temporaries.
    scopes: Vec<(LLVMBasicBlockRef, Vec<LLVMValueRef>)>,
    /// What பலகை() answers.
    board: &'static str,
    /// The program's வடிவங்கள், and the struct type each is.
    shapes: Vec<ShapeInfo>,
    shape_types: Vec<LLVMTypeRef>,
}

impl Emitter {
    fn new(
        context: LLVMContextRef,
        module: LLVMModuleRef,
        layout: llvm_sys::target::LLVMTargetDataRef,
        avr: bool,
        board: &'static str,
    ) -> Self {
        unsafe {
            Emitter {
                context,
                module,
                builder: LLVMCreateBuilderInContext(context),
                layout,
                runtime: HashMap::new(),
                globals: HashMap::new(),
                functions: HashMap::new(),
                locals: HashMap::new(),
                current: ptr::null_mut(),
                returns: None,
                out: None,
                avr,
                flash: false,
                texts: HashMap::new(),
                place: String::new(),
                line: None,
                flash_constants: HashSet::new(),
                externs: Vec::new(),
                extern_shims: HashMap::new(),
                site_texts: Vec::new(),
                line_reports: false,
                scopes: Vec::new(),
                board,
                shapes: Vec::new(),
                shape_types: Vec::new(),
            }
        }
    }

    // --- types -------------------------------------------------------------------

    fn i1(&self) -> LLVMTypeRef {
        unsafe { LLVMInt1TypeInContext(self.context) }
    }
    fn i8(&self) -> LLVMTypeRef {
        unsafe { LLVMInt8TypeInContext(self.context) }
    }
    fn i32(&self) -> LLVMTypeRef {
        unsafe { LLVMInt32TypeInContext(self.context) }
    }
    fn i64(&self) -> LLVMTypeRef {
        unsafe { LLVMInt64TypeInContext(self.context) }
    }
    fn void(&self) -> LLVMTypeRef {
        unsafe { LLVMVoidTypeInContext(self.context) }
    }
    fn ptr(&self) -> LLVMTypeRef {
        unsafe { LLVMPointerTypeInContext(self.context, 0) }
    }

    fn elem_type(&self, elem: Elem) -> LLVMTypeRef {
        self.storage(elem.ty())
    }

    /// Held in memory and passed by address: text, arrays, results, records.
    fn is_buffer(ty: Ty) -> bool {
        matches!(ty, Ty::Text | Ty::Array(..) | Ty::Result(_) | Ty::Shape(_))
    }

    /// What a variable of this type occupies.
    fn storage(&self, ty: Ty) -> LLVMTypeRef {
        unsafe {
            match ty {
                Ty::Num => self.i64(),
                Ty::Bool => self.i1(),
                Ty::Text => LLVMArrayType2(self.i8(), TEXT_BYTES as u64),
                Ty::Array(elem, n) => LLVMArrayType2(self.elem_type(elem), n as u64),
                Ty::Result(_) => self.result_type(),
                Ty::Shape(id) => self.shape_types[id as usize],
                Ty::Void => self.void(),
            }
        }
    }

    /// Every result, whatever it holds: `{ ok, payload }`. The runtime's
    /// `artino_result` is the same two fields.
    fn result_type(&self) -> LLVMTypeRef {
        unsafe {
            let mut fields = [self.i1(), LLVMArrayType2(self.i8(), TEXT_BYTES as u64)];
            LLVMStructTypeInContext(self.context, fields.as_mut_ptr(), 2, 0)
        }
    }

    /// Text and results come back through a buffer the caller passes first.
    fn returns_buffer(ty: Ty) -> bool {
        Self::is_buffer(ty)
    }

    /// How a value of this type is passed: buffers by pointer.
    fn passed(&self, ty: Ty) -> LLVMTypeRef {
        match ty {
            ty if Self::is_buffer(ty) => self.ptr(),
            other => self.storage(other),
        }
    }

    /// The byte size of a buffer, as the target's pointer-sized integer.
    fn size_of(&self, ty: Ty) -> LLVMValueRef {
        unsafe {
            let bytes = LLVMABISizeOfType(self.layout, self.storage(ty));
            LLVMConstInt(LLVMIntPtrTypeInContext(self.context, self.layout), bytes, 0)
        }
    }

    fn num(&self, value: i64) -> LLVMValueRef {
        unsafe { LLVMConstInt(self.i64(), value as u64, 1) }
    }

    // --- the runtime ----------------------------------------------------------------

    fn declare_runtime(&mut self) {
        let (i32, i64, void, ptr) = (self.i32(), self.i64(), self.void(), self.ptr());
        let i16 = unsafe { LLVMInt16TypeInContext(self.context) };
        let mut entries: Vec<(&'static str, Vec<LLVMTypeRef>, LLVMTypeRef)> = vec![
            ("artino_num_add", vec![i64, i64, ptr], i64),
            ("artino_num_sub", vec![i64, i64, ptr], i64),
            ("artino_num_mul", vec![i64, i64, ptr], i64),
            ("artino_num_div", vec![i64, i64, ptr], i64),
            ("artino_print_text", vec![ptr], void),
            ("artino_print_str", vec![ptr], void),
            ("artino_print_num", vec![i64], void),
            ("artino_print_bool", vec![i32], void),
            ("artino_print_num_array", vec![ptr, i16], void),
            ("artino_print_bool_array", vec![ptr, i16], void),
            ("artino_print_line", vec![], void),
            ("artino_text_clear", vec![ptr], void),
            ("artino_text_copy", vec![ptr, ptr], void),
            ("artino_copy_flash", vec![ptr, ptr, i16], void),
            ("artino_to_int", vec![i64, ptr], i32),
            ("artino_num_mul_whole", vec![i64, i32, ptr], i64),
            ("artino_text_append", vec![ptr, ptr, ptr], void),
            ("artino_text_append_flash", vec![ptr, ptr, ptr], void),
            ("artino_text_append_num", vec![ptr, i64, ptr], void),
            ("artino_text_append_bool", vec![ptr, i32, ptr], void),
            ("artino_text_equal", vec![ptr, ptr], i32),
            ("artino_report_index", vec![ptr], void),
            ("artino_report_unwrap", vec![ptr, ptr], void),
            ("artino_report_unwrap_err", vec![ptr], void),
            ("artino_print_result", vec![ptr, i32], void),
            ("artino_num_round", vec![i64, i32, i32], i64),
            ("artino_num_div_round", vec![i64, i64, i32, i32, ptr], i64),
            ("artino_num_mul_round", vec![i64, i64, i32, i32, ptr], i64),
            ("artino_text_to_number", vec![ptr, ptr, ptr], void),
            ("artino_text_letters", vec![ptr, ptr], i32),
            ("artino_text_letter", vec![ptr, i32, ptr, ptr], void),
            ("artino_text_letter_flash", vec![ptr, i32, ptr, ptr], void),
            ("artino_serial_open", vec![i32, i32, ptr], void),
            ("artino_serial_read_line", vec![i32, i32, ptr, ptr], void),
            ("artino_serial_write", vec![i32, ptr, i32, ptr], void),
            ("artino_serial_close", vec![i32, ptr], void),
        ];
        for board in analyse::INTRINSICS {
            let params = board.args.iter().map(|_| i32).collect();
            let ret = match board.ret {
                Ret::Void => void,
                Ret::Int | Ret::Millis | Ret::Flag => i32,
            };
            entries.push((board.shim, params, ret));
        }
        for (name, mut params, ret) in entries {
            unsafe {
                let kind = LLVMFunctionType(ret, params.as_mut_ptr(), params.len() as u32, 0);
                let function = LLVMAddFunction(self.module, c(name).as_ptr(), kind);
                self.runtime.insert(name, (function, kind));
            }
        }
    }

    fn call(&self, name: &str, args: &mut [LLVMValueRef]) -> LLVMValueRef {
        let (function, kind) = self.runtime[name];
        unsafe {
            LLVMBuildCall2(
                self.builder,
                kind,
                function,
                args.as_mut_ptr(),
                args.len() as u32,
                c("").as_ptr(),
            )
        }
    }

    /// A NUL-terminated constant holding `text`, in flash on AVR.
    fn program_text(&mut self, text: &str) -> Result<LLVMValueRef, String> {
        if let Some(existing) = self.texts.get(text) {
            return Ok(*existing);
        }
        if text.contains('\0') {
            return Err("text with a NUL byte in it".to_string());
        }
        unsafe {
            let bytes = text.as_bytes();
            let value = LLVMConstStringInContext(
                self.context,
                bytes.as_ptr() as *const _,
                bytes.len() as u32,
                0,
            );
            let global = LLVMAddGlobal(self.module, LLVMTypeOf(value), c("et.text").as_ptr());
            LLVMSetInitializer(global, value);
            LLVMSetGlobalConstant(global, 1);
            LLVMSetLinkage(global, LLVMLinkage::LLVMPrivateLinkage);
            LLVMSetUnnamedAddress(global, llvm_sys::LLVMUnnamedAddr::LLVMGlobalUnnamedAddr);
            if self.avr {
                // Only its address is ever taken; the runtime reads it with
                // flash instructions, as avr-gcc's PROGMEM does.
                LLVMSetSection(global, c(".progmem.data").as_ptr());
            }
            self.texts.insert(text.to_string(), global);
            Ok(global)
        }
    }

    /// A site for one operation: `{ number, line, "<place>, வரி <n>: <text>" }`,
    /// constant, and in flash on AVR. Whether it has been reported is a bit
    /// of `artino_said`, so the site itself never changes. On a board with
    /// `line_reports` the text is left out.
    fn site(&mut self, operation: &Expr) -> Result<LLVMValueRef, String> {
        let shown = self
            .line
            .map(|n| format!(", வரி {}", n))
            .unwrap_or_default();
        let place = format!(
            "{}{}: {}",
            self.place,
            shown,
            super::source::text(operation)
        );
        let index = self.site_texts.len();
        if index > u16::MAX as usize {
            return Err("more than 65,536 report sites".to_string());
        }
        let description = if self.line_reports {
            unsafe { LLVMConstNull(self.ptr()) }
        } else {
            self.program_text(&place)?
        };
        self.site_texts.push(place);
        unsafe {
            let i16 = LLVMInt16TypeInContext(self.context);
            let mut fields = [i16, i16, self.ptr()];
            let kind = LLVMStructTypeInContext(self.context, fields.as_mut_ptr(), 3, 0);
            let line = self.line.unwrap_or(0).min(u16::MAX as usize);
            let mut values = [
                LLVMConstInt(i16, index as u64, 0),
                LLVMConstInt(i16, line as u64, 0),
                description,
            ];
            let initial = LLVMConstStructInContext(self.context, values.as_mut_ptr(), 3, 0);
            let global =
                LLVMAddGlobal(self.module, kind, c(&format!("et.site.{}", index)).as_ptr());
            LLVMSetInitializer(global, initial);
            LLVMSetGlobalConstant(global, 1);
            LLVMSetLinkage(global, LLVMLinkage::LLVMInternalLinkage);
            if self.avr {
                LLVMSetSection(global, c(".progmem.data").as_ptr());
            }
            Ok(global)
        }
    }

    /// `artino_said`: one bit per site, set once the site has been reported.
    fn said(&mut self) {
        unsafe {
            let bytes = self.site_texts.len().div_ceil(8).max(1) as u32;
            let kind = LLVMArrayType2(self.i8(), bytes as u64);
            let global = LLVMAddGlobal(self.module, kind, c("artino_said").as_ptr());
            LLVMSetInitializer(global, LLVMConstNull(kind));
        }
    }

    /// A buffer for an expression, allocated in the entry block so a loop
    /// reuses it rather than growing the stack each time round.
    fn temp(&mut self, ty: Ty, label: &str) -> LLVMValueRef {
        let kind = self.storage(ty);
        self.temp_of(kind, label)
    }

    fn temp_of(&mut self, kind: LLVMTypeRef, label: &str) -> LLVMValueRef {
        unsafe {
            let here = LLVMGetInsertBlock(self.builder);
            let entry = LLVMGetEntryBasicBlock(self.current);
            let first = LLVMGetFirstInstruction(entry);
            if first.is_null() {
                LLVMPositionBuilderAtEnd(self.builder, entry);
            } else {
                LLVMPositionBuilderBefore(self.builder, first);
            }
            let slot = LLVMBuildAlloca(self.builder, kind, c(label).as_ptr());
            // Alive for the statement that made it, and no longer: LLVM can
            // then give one stack slot to temporaries of different statements.
            // Without this every buffer temporary in a function is its own
            // 49 bytes or more, and artino_loop outgrows an Uno's RAM.
            let before = self.scopes.last_mut().map(|(before, temps)| {
                temps.push(slot);
                *before
            });
            if let Some(before) = before {
                LLVMPositionBuilderBefore(self.builder, LLVMGetBasicBlockTerminator(before));
                self.lifetime("llvm.lifetime.start", slot);
            }
            LLVMPositionBuilderAtEnd(self.builder, here);
            slot
        }
    }

    /// Before a return, which leaves every statement being lowered: all
    /// their temporaries end. Otherwise, once LLVM inlines the function, they
    /// look alive for the rest of the caller and no slot can be shared.
    fn end_lifetimes(&mut self) {
        let slots: Vec<LLVMValueRef> = self
            .scopes
            .iter()
            .flat_map(|(_, temps)| temps.iter().copied())
            .collect();
        for slot in slots {
            self.lifetime("llvm.lifetime.end", slot);
        }
    }

    fn lifetime(&mut self, intrinsic: &str, slot: LLVMValueRef) {
        unsafe {
            let id = LLVMLookupIntrinsicID(intrinsic.as_ptr() as *const _, intrinsic.len());
            let mut types = [self.ptr()];
            let function = LLVMGetIntrinsicDeclaration(self.module, id, types.as_mut_ptr(), 1);
            let kind = LLVMIntrinsicGetType(self.context, id, types.as_mut_ptr(), 1);
            // -1: the whole object.
            let mut args = [LLVMConstInt(self.i64(), u64::MAX, 1), slot];
            LLVMBuildCall2(
                self.builder,
                kind,
                function,
                args.as_mut_ptr(),
                2,
                c("").as_ptr(),
            );
        }
    }

    /// `x = x & …`, with x text read nowhere else in the chain: appended to x
    /// where it is, which is what the VM's answer is, without a copy of x.
    fn appends_to(&self, name: &str, whole: &Expr) -> bool {
        let mut pieces: Vec<&Expr> = Vec::new();
        flatten(whole, &mut pieces);
        matches!(pieces.first(), Some(Expr::Variable(first)) if first == name)
            && !pieces[1..]
                .iter()
                .any(|piece| analyse::mentions(piece, name))
            && !self.in_flash(name)
            && self.target(name).is_ok_and(|(_, ty)| ty == Ty::Text)
    }

    /// Text known when compiling: a literal, or பலகை().
    fn known_text(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::String(text) => Some(text.clone()),
            Expr::Call { name, args }
                if args.is_empty() && analyse::builtin(name) == Some(Builtin::Board) =>
            {
                Some(self.board.to_string())
            }
            _ => None,
        }
    }

    /// Is `name`, here, one of the constants kept in flash on AVR.
    fn in_flash(&self, name: &str) -> bool {
        self.flash && !self.locals.contains_key(name) && self.flash_constants.contains(name)
    }

    fn copy_flash(&mut self, destination: LLVMValueRef, source: LLVMValueRef, ty: Ty) {
        unsafe {
            let bytes = LLVMABISizeOfType(self.layout, self.storage(ty));
            let size = LLVMConstInt(LLVMInt16TypeInContext(self.context), bytes, 0);
            self.call("artino_copy_flash", &mut [destination, source, size]);
        }
    }

    /// A `நிலை`'s value as an LLVM constant, when it is known now: a text
    /// literal, or an array of number or boolean literals.
    fn constant(&self, value: &Expr, ty: Ty) -> Option<LLVMValueRef> {
        unsafe {
            match (value, ty) {
                (Expr::String(text), Ty::Text)
                    if text.len() < TEXT_BYTES as usize && !text.contains('\0') =>
                {
                    let mut bytes = text.as_bytes().to_vec();
                    bytes.resize(TEXT_BYTES as usize, 0);
                    Some(LLVMConstStringInContext(
                        self.context,
                        bytes.as_ptr() as *const _,
                        bytes.len() as u32,
                        1,
                    ))
                }
                (Expr::ArrayLiteral(items), Ty::Array(Elem::Num, _)) => {
                    let mut values = items
                        .iter()
                        .map(|item| match item {
                            Expr::Number(n) => analyse::scaled(n).ok(),
                            other => folded(other),
                        })
                        .map(|n| n.map(|n| self.num(n)))
                        .collect::<Option<Vec<_>>>()?;
                    Some(LLVMConstArray2(
                        self.i64(),
                        values.as_mut_ptr(),
                        values.len() as u64,
                    ))
                }
                (Expr::ArrayLiteral(items), Ty::Array(Elem::Bool, _)) => {
                    let mut values = items
                        .iter()
                        .map(|item| match item {
                            Expr::Boolean(b) => Some(LLVMConstInt(self.i1(), u64::from(*b), 0)),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>()?;
                    Some(LLVMConstArray2(
                        self.i1(),
                        values.as_mut_ptr(),
                        values.len() as u64,
                    ))
                }
                _ => None,
            }
        }
    }

    fn copy(&mut self, ty: Ty, destination: LLVMValueRef, source: LLVMValueRef) {
        unsafe {
            match ty {
                Ty::Text => {
                    self.call("artino_text_copy", &mut [destination, source]);
                }
                Ty::Array(..) | Ty::Result(_) | Ty::Shape(_) => {
                    LLVMBuildMemMove(self.builder, destination, 1, source, 1, self.size_of(ty));
                }
                _ => {
                    LLVMBuildStore(self.builder, source, destination);
                }
            }
        }
    }

    // --- the program -----------------------------------------------------------------

    fn program(&mut self, program: &Program) -> Result<(), String> {
        self.declare_runtime();
        self.externs = program.externs.clone();

        // Named structs first, bodies after, so a field can be another வடிவம்.
        self.shapes = program.shapes.clone();
        for shape in &program.shapes {
            let kind = unsafe {
                LLVMStructCreateNamed(
                    self.context,
                    c(&format!("et.shape.{}", shape.name)).as_ptr(),
                )
            };
            self.shape_types.push(kind);
        }
        for (index, shape) in program.shapes.iter().enumerate() {
            let mut fields: Vec<LLVMTypeRef> = shape
                .fields
                .iter()
                .map(|(_, ty)| self.storage(*ty))
                .collect();
            unsafe {
                LLVMStructSetBody(
                    self.shape_types[index],
                    fields.as_mut_ptr(),
                    fields.len() as u32,
                    0,
                )
            };
        }

        let mut constants = HashMap::new();
        for statement in &program.setup {
            if let Stmt::Assign {
                name,
                value,
                immutable: true,
                ..
            } = statement
            {
                constants.insert(name.clone(), value);
            }
        }
        for (name, ty) in &program.globals {
            unsafe {
                let kind = self.storage(*ty);
                let global =
                    LLVMAddGlobal(self.module, kind, c(&format!("et.g.{}", name)).as_ptr());
                LLVMSetLinkage(global, LLVMLinkage::LLVMInternalLinkage);
                match constants
                    .get(name)
                    .and_then(|value| self.constant(value, *ty))
                {
                    Some(initial) => {
                        LLVMSetInitializer(global, initial);
                        LLVMSetGlobalConstant(global, 1);
                        if self.avr {
                            LLVMSetSection(global, c(".progmem.data").as_ptr());
                        }
                        self.flash_constants.insert(name.clone());
                    }
                    None => LLVMSetInitializer(global, LLVMConstNull(kind)),
                }
                self.globals.insert(name.clone(), (global, *ty));
            }
        }

        // Every function is declared before any body, so calls can go either way.
        for function in &program.functions {
            unsafe {
                let mut params: Vec<LLVMTypeRef> = Vec::new();
                let ret = if Self::returns_buffer(function.ret) {
                    params.push(self.ptr());
                    self.void()
                } else {
                    self.passed(function.ret)
                };
                params.extend(function.params.iter().map(|(_, t)| self.passed(*t)));
                let kind = LLVMFunctionType(ret, params.as_mut_ptr(), params.len() as u32, 0);
                let value = LLVMAddFunction(
                    self.module,
                    c(&format!("et.f.{}", function.name)).as_ptr(),
                    kind,
                );
                LLVMSetLinkage(value, LLVMLinkage::LLVMInternalLinkage);
                self.functions
                    .insert(function.name.clone(), (value, kind, function.ret));
            }
        }
        for function in &program.functions {
            self.function(function)?;
        }

        // Each இடைவெளி block becomes a function of its own, run from the loop.
        let mut every = Vec::new();
        for (index, schedule) in program.schedules.iter().enumerate() {
            let body = self.define_void(&format!("et.every.{}", index), false);
            self.place = format!("இடைவெளி {}", schedule.seconds);
            self.line = None;
            self.block_of(&schedule.body)?;
            self.finish_void();
            let next = unsafe {
                let next = LLVMAddGlobal(
                    self.module,
                    self.i32(),
                    c(&format!("et.next.{}", index)).as_ptr(),
                );
                LLVMSetInitializer(next, LLVMConstNull(self.i32()));
                LLVMSetLinkage(next, LLVMLinkage::LLVMInternalLinkage);
                next
            };
            every.push((body, next, schedule.ms));
        }

        self.define_void("artino_setup", true);
        self.place = "தொடக்கம்".to_string();
        self.line = None;
        self.block_of(&program.setup)?;
        self.finish_void();

        self.define_void("artino_loop", true);
        unsafe {
            let now = self.call("artino_millis", &mut []);
            for (body, next, ms) in every {
                let kind = LLVMFunctionType(self.void(), ptr::null_mut(), 0, 0);
                if ms == 0 {
                    LLVMBuildCall2(self.builder, kind, body, ptr::null_mut(), 0, c("").as_ptr());
                    continue;
                }
                let due_at = LLVMBuildLoad2(self.builder, self.i32(), next, c("due_at").as_ptr());
                let late = LLVMBuildSub(self.builder, now, due_at, c("late").as_ptr());
                // Signed, so the comparison survives millis() wrapping after 49 days.
                let due = LLVMBuildICmp(
                    self.builder,
                    LLVMIntPredicate::LLVMIntSGE,
                    late,
                    LLVMConstNull(self.i32()),
                    c("due").as_ptr(),
                );
                let run = self.block("run");
                let skip = self.block("skip");
                LLVMBuildCondBr(self.builder, due, run, skip);
                LLVMPositionBuilderAtEnd(self.builder, run);
                let following = LLVMBuildAdd(
                    self.builder,
                    now,
                    LLVMConstInt(self.i32(), ms as u64, 0),
                    c("following").as_ptr(),
                );
                LLVMBuildStore(self.builder, following, next);
                LLVMBuildCall2(self.builder, kind, body, ptr::null_mut(), 0, c("").as_ptr());
                LLVMBuildBr(self.builder, skip);
                LLVMPositionBuilderAtEnd(self.builder, skip);
            }
            if let Some(cycle) = &program.cycle {
                let (function, kind, _) = self.functions[cycle];
                LLVMBuildCall2(
                    self.builder,
                    kind,
                    function,
                    ptr::null_mut(),
                    0,
                    c("").as_ptr(),
                );
            }
        }
        self.finish_void();
        self.said();
        Ok(())
    }

    /// Start a `void f(void)`; exported for the sketch, internal otherwise.
    fn define_void(&mut self, name: &str, exported: bool) -> LLVMValueRef {
        unsafe {
            let kind = LLVMFunctionType(self.void(), ptr::null_mut(), 0, 0);
            let function = LLVMAddFunction(self.module, c(name).as_ptr(), kind);
            if !exported {
                LLVMSetLinkage(function, LLVMLinkage::LLVMInternalLinkage);
            }
            self.current = function;
            self.locals.clear();
            self.returns = None;
            self.out = None;
            let entry = LLVMAppendBasicBlockInContext(self.context, function, c("entry").as_ptr());
            LLVMPositionBuilderAtEnd(self.builder, entry);
            function
        }
    }

    fn finish_void(&mut self) {
        if !self.terminated() {
            unsafe {
                LLVMBuildRetVoid(self.builder);
            }
        }
    }

    fn function(&mut self, function: &analyse::Function) -> Result<(), String> {
        let (value, _, ret) = self.functions[&function.name];
        unsafe {
            self.current = value;
            self.returns = Some(ret);
            self.locals.clear();
            self.place = format!("செயல் {}", function.name);
            self.line = Some(function.line).filter(|n| *n > 0);
            let entry = LLVMAppendBasicBlockInContext(self.context, value, c("entry").as_ptr());
            LLVMPositionBuilderAtEnd(self.builder, entry);

            let mut offset = 0;
            self.out = None;
            if Self::returns_buffer(ret) {
                let out = LLVMGetParam(value, 0);
                // Empty text, or a தவறு with empty text: what falling off the end gives.
                LLVMBuildStore(self.builder, LLVMConstNull(self.storage(ret)), out);
                self.out = Some(out);
                offset = 1;
            }
            for (index, (name, ty)) in function.params.iter().enumerate() {
                let arrived = LLVMGetParam(value, (index + offset) as u32);
                // A buffer the body only reads is read where the caller holds
                // it: a function cannot change the program's variables, so
                // nothing can change it during the call. One the body writes
                // is a copy, as on the VM, so the caller's is never written.
                if Self::is_buffer(*ty) && !function.written.contains(name) {
                    self.locals.insert(name.clone(), (arrived, *ty));
                    continue;
                }
                let slot = LLVMBuildAlloca(self.builder, self.storage(*ty), c(name).as_ptr());
                self.copy(*ty, slot, arrived);
                self.locals.insert(name.clone(), (slot, *ty));
            }
            for (name, ty) in &function.locals {
                // A local that only ever holds one letter needs a letter's bytes.
                let kind = if function.letters.contains(name) {
                    LLVMArrayType2(self.i8(), LETTER_BYTES as u64)
                } else {
                    self.storage(*ty)
                };
                let slot = LLVMBuildAlloca(self.builder, kind, c(name).as_ptr());
                LLVMBuildStore(self.builder, LLVMConstNull(kind), slot);
                self.locals.insert(name.clone(), (slot, *ty));
            }
        }
        self.block_of(&function.body)?;
        if !self.terminated() {
            // Falling off the end: the VM gives இன்மை; a board has no nil, so
            // the type's zero (for text, the empty text already in `out`).
            unsafe {
                match ret {
                    ty if ty == Ty::Void || Self::returns_buffer(ty) => {
                        LLVMBuildRetVoid(self.builder)
                    }
                    ty => LLVMBuildRet(self.builder, LLVMConstNull(self.storage(ty))),
                };
            }
        }
        self.returns = None;
        self.out = None;
        Ok(())
    }

    // --- statements --------------------------------------------------------------------

    fn terminated(&self) -> bool {
        unsafe { !LLVMGetBasicBlockTerminator(LLVMGetInsertBlock(self.builder)).is_null() }
    }

    fn block(&self, label: &str) -> LLVMBasicBlockRef {
        unsafe { LLVMAppendBasicBlockInContext(self.context, self.current, c(label).as_ptr()) }
    }

    fn block_of(&mut self, body: &[Stmt]) -> Result<(), String> {
        for statement in body {
            if self.terminated() {
                // Code after a திரும்பு: unreachable, but still well-formed IR.
                let dead = self.block("after_return");
                unsafe { LLVMPositionBuilderAtEnd(self.builder, dead) };
            }
            self.stmt(statement)?;
        }
        Ok(())
    }

    fn slot(&self, name: &str) -> Option<(LLVMValueRef, Ty)> {
        self.locals
            .get(name)
            .or_else(|| self.globals.get(name))
            .copied()
    }

    /// Where an assignment to `name` goes: in a செயல், the function's own
    /// (as on the VM); at the top level and in இடைவெளி blocks, the program's.
    fn target(&self, name: &str) -> Result<(LLVMValueRef, Ty), String> {
        let found = if self.returns.is_some() {
            self.locals.get(name)
        } else {
            self.globals.get(name)
        };
        found
            .copied()
            .ok_or_else(|| format!("{} has no storage — an artino bug", name))
    }

    /// One statement, in a block of its own, so the lifetimes of the
    /// temporaries it makes start in the block before it and end after it.
    fn stmt(&mut self, statement: &Stmt) -> Result<(), String> {
        unsafe {
            let before = LLVMGetInsertBlock(self.builder);
            let body = self.block("stmt");
            LLVMBuildBr(self.builder, body);
            LLVMPositionBuilderAtEnd(self.builder, body);
            self.scopes.push((before, Vec::new()));
        }
        let lowered = self.statement(statement);
        let (_, temps) = self.scopes.pop().expect("pushed above");
        if lowered.is_ok() && !self.terminated() {
            for slot in temps {
                self.lifetime("llvm.lifetime.end", slot);
            }
        }
        lowered
    }

    fn statement(&mut self, statement: &Stmt) -> Result<(), String> {
        match statement {
            // Its value is the global's initialiser.
            Stmt::Assign { name, .. }
                if self.returns.is_none() && self.flash_constants.contains(name) => {}
            Stmt::Assign {
                name,
                value: whole @ Expr::Concat { .. },
                at,
                ..
            } if self.appends_to(name, whole) => {
                let outer = self.line;
                self.line = Some(at.line);
                let mut pieces: Vec<&Expr> = Vec::new();
                flatten(whole, &mut pieces);
                let (slot, _) = self.target(name)?;
                let lowered = self.join(whole, &pieces[1..], Some(slot), false);
                self.line = outer;
                lowered?;
            }
            Stmt::Assign {
                name,
                value: Expr::Call { name: called, args },
                at,
                ..
            } if self.returns_into(name, called, args) => {
                // A செயல்'s own variable, given a செயல்'s text or record: the
                // callee writes it directly. Nothing it can reach is the
                // variable — not its arguments, which do not name it, nor the
                // program's variables, which a local is not.
                let outer = self.line;
                self.line = Some(at.line);
                let mut values = Vec::new();
                for arg in args {
                    match self.expr(arg) {
                        Ok((value, _)) => values.push(value),
                        Err(e) => {
                            self.line = outer;
                            return Err(e);
                        }
                    }
                }
                let (slot, _) = self.target(name)?;
                let lowered = self.call_function_into(called, values, Some(slot));
                self.line = outer;
                lowered?;
            }
            Stmt::Assign {
                name, value, at, ..
            } => {
                let outer = self.line;
                self.line = Some(at.line);
                let lowered = self.expr(value);
                self.line = outer;
                let (value, _) = lowered?;
                let (slot, ty) = self.target(name)?;
                self.copy(ty, slot, value);
            }
            Stmt::Expression(expr) => {
                self.expr(expr)?;
            }
            Stmt::Print(expr) => {
                self.print(expr)?;
                self.call("artino_print_line", &mut []);
            }
            Stmt::Return(value) => unsafe {
                match (value, self.out) {
                    // திரும்பு g(…): g writes this function's result itself.
                    // Nothing in the program can name the result buffer, so
                    // no argument can be reading it.
                    (Some(Expr::Call { name, args }), Some(out))
                        if self
                            .functions
                            .get(name.as_str())
                            .is_some_and(|(_, _, ret)| Some(*ret) == self.returns)
                            && analyse::builtin(name).is_none() =>
                    {
                        let mut values = Vec::new();
                        for arg in args {
                            values.push(self.expr(arg)?.0);
                        }
                        self.call_function_into(name, values, Some(out))?;
                        self.end_lifetimes();
                        LLVMBuildRetVoid(self.builder);
                    }
                    (Some(value @ (Expr::Concat { .. } | Expr::String(_))), Some(out)) => {
                        self.text_into(value, Some(out))?;
                        self.end_lifetimes();
                        LLVMBuildRetVoid(self.builder);
                    }
                    (Some(value), Some(out)) => {
                        let (value, ty) = self.expr(value)?;
                        self.copy(ty, out, value);
                        self.end_lifetimes();
                        LLVMBuildRetVoid(self.builder);
                    }
                    (Some(value), None) => {
                        let (value, _) = self.expr(value)?;
                        self.end_lifetimes();
                        LLVMBuildRet(self.builder, value);
                    }
                    (None, _) => match self.returns {
                        Some(Ty::Void) | None => {
                            self.end_lifetimes();
                            LLVMBuildRetVoid(self.builder);
                        }
                        Some(ty) if Self::returns_buffer(ty) => {
                            self.end_lifetimes();
                            LLVMBuildRetVoid(self.builder);
                        }
                        Some(ty) => {
                            self.end_lifetimes();
                            LLVMBuildRet(self.builder, LLVMConstNull(self.storage(ty)));
                        }
                    },
                }
            },
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => unsafe {
                let (test, _) = self.expr(condition)?;
                let then_block = self.block("then");
                let else_block = self.block("else");
                let done = self.block("endif");
                LLVMBuildCondBr(self.builder, test, then_block, else_block);
                LLVMPositionBuilderAtEnd(self.builder, then_block);
                self.block_of(then_branch)?;
                if !self.terminated() {
                    LLVMBuildBr(self.builder, done);
                }
                LLVMPositionBuilderAtEnd(self.builder, else_block);
                if let Some(branch) = else_branch {
                    self.block_of(branch)?;
                }
                if !self.terminated() {
                    LLVMBuildBr(self.builder, done);
                }
                LLVMPositionBuilderAtEnd(self.builder, done);
            },
            Stmt::Loop { condition, body } => unsafe {
                let head = self.block("loop");
                let inside = self.block("body");
                let after = self.block("endloop");
                LLVMBuildBr(self.builder, head);
                LLVMPositionBuilderAtEnd(self.builder, head);
                let (test, _) = self.expr(condition)?;
                LLVMBuildCondBr(self.builder, test, inside, after);
                LLVMPositionBuilderAtEnd(self.builder, inside);
                self.block_of(body)?;
                if !self.terminated() {
                    LLVMBuildBr(self.builder, head);
                }
                LLVMPositionBuilderAtEnd(self.builder, after);
            },
            Stmt::ForEach {
                var,
                collection,
                body,
            } => self.for_each(var, collection, body)?,
            Stmt::SetIndex {
                name, index, value, ..
            } => {
                let (array, ty) = self
                    .slot(name)
                    .ok_or_else(|| format!("{} has no storage — an artino bug", name))?;
                let Ty::Array(elem, n) = ty else {
                    return Err(format!(
                        "{} is not an array — analyse should have refused it",
                        name
                    ));
                };
                let (position, _) = self.expr(index)?;
                let (value, _) = self.expr(value)?;
                let operation = Expr::Index {
                    base: Box::new(Expr::Variable(name.clone())),
                    index: Box::new(index.clone()),
                };
                let site = self.site(&operation)?;
                unsafe {
                    let (whole, inside) = self.bounds(position, n);
                    let write = self.block("index_ok");
                    let refuse = self.block("index_bad");
                    let done = self.block("index_done");
                    LLVMBuildCondBr(self.builder, inside, write, refuse);
                    LLVMPositionBuilderAtEnd(self.builder, write);
                    let element = self.element(array, elem, n, whole);
                    self.copy(elem.ty(), element, value);
                    LLVMBuildBr(self.builder, done);
                    LLVMPositionBuilderAtEnd(self.builder, refuse);
                    self.call("artino_report_index", &mut [site]);
                    LLVMBuildBr(self.builder, done);
                    LLVMPositionBuilderAtEnd(self.builder, done);
                }
            }
            Stmt::SetField {
                name, field, value, ..
            } => {
                let (record, ty) = self
                    .slot(name)
                    .ok_or_else(|| format!("{} has no storage — an artino bug", name))?;
                let Ty::Shape(id) = ty else {
                    return Err(format!(
                        "{} is not a record — analyse should have refused it",
                        name
                    ));
                };
                let (value, _) = self.expr(value)?;
                let (at, field_ty) = self.field_at(record, id, field)?;
                self.copy(field_ty, at, value);
            }
            Stmt::Import(_) => {}
            other => {
                return Err(format!(
                    "{} reached the emitter — analyse should have refused it",
                    crate::codegen::stmt_label(other)
                ));
            }
        }
        Ok(())
    }

    /// `ஒவ்வொரு x இல் xs`: over a copy of the array as it was when the loop
    /// began, as the VM iterates the value it was given.
    fn for_each(&mut self, var: &str, collection: &Expr, body: &[Stmt]) -> Result<(), String> {
        let (source, ty) = self.expr(collection)?;
        // The VM walks what the collection was when the loop began. A copy
        // keeps that only when the body could change it; otherwise, or when
        // the collection is already a temporary, it is walked where it is.
        let in_place = match collection {
            Expr::Variable(name) => {
                let mut changed = HashSet::new();
                analyse::written(body, &mut changed);
                !changed.contains(name)
            }
            Expr::ArrayLiteral(_) | Expr::String(_) | Expr::Concat { .. } | Expr::Call { .. } => {
                true
            }
            _ => false,
        };
        let items = if in_place {
            source
        } else {
            let items = self.temp(ty, "each_items");
            self.copy(ty, items, source);
            items
        };
        if ty == Ty::Text {
            return self.for_each_letter(var, collection, items, body);
        }
        let Ty::Array(elem, n) = ty else {
            return Err(
                "ஒவ்வொரு over something not an array — analyse should have refused it".to_string(),
            );
        };
        let counter = self.temp(Ty::Num, "each_index");
        let (slot, _) = self.target(var)?;
        unsafe {
            LLVMBuildStore(self.builder, self.num(0), counter);
            let head = self.block("each");
            let inside = self.block("each_body");
            let after = self.block("each_done");
            LLVMBuildBr(self.builder, head);
            LLVMPositionBuilderAtEnd(self.builder, head);
            let at = LLVMBuildLoad2(self.builder, self.i64(), counter, c("at").as_ptr());
            let more = LLVMBuildICmp(
                self.builder,
                LLVMIntPredicate::LLVMIntSLT,
                at,
                LLVMConstInt(self.i64(), n as u64, 0),
                c("more").as_ptr(),
            );
            LLVMBuildCondBr(self.builder, more, inside, after);
            LLVMPositionBuilderAtEnd(self.builder, inside);
            let element = self.element(items, elem, n, at);
            if Self::is_buffer(elem.ty()) {
                self.copy(elem.ty(), slot, element);
            } else {
                let item = LLVMBuildLoad2(
                    self.builder,
                    self.elem_type(elem),
                    element,
                    c("item").as_ptr(),
                );
                LLVMBuildStore(self.builder, item, slot);
            }
            self.block_of(body)?;
            if !self.terminated() {
                let at = LLVMBuildLoad2(self.builder, self.i64(), counter, c("at").as_ptr());
                let next = LLVMBuildAdd(
                    self.builder,
                    at,
                    LLVMConstInt(self.i64(), 1, 0),
                    c("next").as_ptr(),
                );
                LLVMBuildStore(self.builder, next, counter);
                LLVMBuildBr(self.builder, head);
            }
            LLVMPositionBuilderAtEnd(self.builder, after);
        }
        Ok(())
    }

    /// `ஒவ்வொரு எழுத்து இல் உரை`: each letter, as the VM segments letters.
    fn for_each_letter(
        &mut self,
        var: &str,
        collection: &Expr,
        text: LLVMValueRef,
        body: &[Stmt],
    ) -> Result<(), String> {
        let site = self.site(collection)?;
        let count = self.call("artino_text_letters", &mut [text, site]);
        let counter = self.temp(Ty::Num, "each_letter");
        let (slot, _) = self.target(var)?;
        unsafe {
            let count = LLVMBuildSExt(self.builder, count, self.i64(), c("letters").as_ptr());
            LLVMBuildStore(self.builder, self.num(0), counter);
            let head = self.block("letter");
            let inside = self.block("letter_body");
            let after = self.block("letter_done");
            LLVMBuildBr(self.builder, head);
            LLVMPositionBuilderAtEnd(self.builder, head);
            let at = LLVMBuildLoad2(self.builder, self.i64(), counter, c("at").as_ptr());
            let more = LLVMBuildICmp(
                self.builder,
                LLVMIntPredicate::LLVMIntSLT,
                at,
                count,
                c("more").as_ptr(),
            );
            LLVMBuildCondBr(self.builder, more, inside, after);
            LLVMPositionBuilderAtEnd(self.builder, inside);
            let index = LLVMBuildTrunc(self.builder, at, self.i32(), c("index").as_ptr());
            self.call("artino_text_letter", &mut [text, index, slot, site]);
            self.block_of(body)?;
            if !self.terminated() {
                let at = LLVMBuildLoad2(self.builder, self.i64(), counter, c("at").as_ptr());
                let next = LLVMBuildAdd(
                    self.builder,
                    at,
                    LLVMConstInt(self.i64(), 1, 0),
                    c("next").as_ptr(),
                );
                LLVMBuildStore(self.builder, next, counter);
                LLVMBuildBr(self.builder, head);
            }
            LLVMPositionBuilderAtEnd(self.builder, after);
        }
        Ok(())
    }

    // --- results ---------------------------------------------------------------------

    unsafe fn result_ok(&mut self, result: LLVMValueRef) -> LLVMValueRef {
        unsafe {
            let field = LLVMBuildStructGEP2(
                self.builder,
                self.result_type(),
                result,
                0,
                c("ok_at").as_ptr(),
            );
            LLVMBuildLoad2(self.builder, self.i1(), field, c("ok").as_ptr())
        }
    }

    unsafe fn result_payload(&mut self, result: LLVMValueRef) -> LLVMValueRef {
        unsafe {
            LLVMBuildStructGEP2(
                self.builder,
                self.result_type(),
                result,
                1,
                c("payload").as_ptr(),
            )
        }
    }

    /// A result made here: `ok`, and what goes in the payload.
    unsafe fn make_result(&mut self, ok: bool, value: LLVMValueRef, ty: Ty) -> LLVMValueRef {
        unsafe {
            let result = self.temp(Ty::Result(Inner::Unknown), "result");
            let field = LLVMBuildStructGEP2(
                self.builder,
                self.result_type(),
                result,
                0,
                c("ok_at").as_ptr(),
            );
            LLVMBuildStore(
                self.builder,
                LLVMConstInt(self.i1(), u64::from(ok), 0),
                field,
            );
            let payload = self.result_payload(result);
            match ty {
                Ty::Text => {
                    self.call("artino_text_copy", &mut [payload, value]);
                }
                Ty::Num | Ty::Bool => {
                    let store = LLVMBuildStore(self.builder, value, payload);
                    LLVMSetAlignment(store, 1);
                }
                _ => {}
            }
            result
        }
    }

    /// What a சரி holds, read out of a result; the type's zero when it is a
    /// தவறு, which is reported — the VM would stop there.
    unsafe fn unwrap(
        &mut self,
        result: LLVMValueRef,
        inner: Inner,
        site: LLVMValueRef,
    ) -> (LLVMValueRef, Ty) {
        unsafe {
            let ok = self.result_ok(result);
            let payload = self.result_payload(result);
            let refuse = self.block("unwrap_bad");
            let done = self.block("unwrap_done");
            LLVMBuildCondBr(self.builder, ok, done, refuse);
            LLVMPositionBuilderAtEnd(self.builder, refuse);
            self.call("artino_report_unwrap", &mut [site, payload]);
            LLVMBuildBr(self.builder, done);
            LLVMPositionBuilderAtEnd(self.builder, done);
            match inner {
                Inner::Num | Inner::Bool => {
                    let ty = inner.ty().unwrap();
                    let load =
                        LLVMBuildLoad2(self.builder, self.storage(ty), payload, c("held").as_ptr());
                    LLVMSetAlignment(load, 1);
                    let zero = LLVMConstNull(self.storage(ty));
                    (
                        LLVMBuildSelect(self.builder, ok, load, zero, c("unwrapped").as_ptr()),
                        ty,
                    )
                }
                Inner::Text => {
                    let empty = self.temp(Ty::Text, "empty");
                    self.call("artino_text_clear", &mut [empty]);
                    (
                        LLVMBuildSelect(self.builder, ok, payload, empty, c("unwrapped").as_ptr()),
                        Ty::Text,
                    )
                }
                Inner::Nothing | Inner::Unknown => (LLVMConstNull(self.i1()), Ty::Void),
            }
        }
    }

    /// `r?` in a செயல் that returns a result: a தவறு goes straight back to
    /// the caller, carrying its text; a சரி gives what it holds.
    fn try_expr(
        &mut self,
        operation: &Expr,
        inner_expr: &Expr,
    ) -> Result<(LLVMValueRef, Ty), String> {
        let (result, ty) = self.expr(inner_expr)?;
        let Ty::Result(inner) = ty else {
            return Err("? on something not a result — analyse should have refused it".to_string());
        };
        let out = self.out.ok_or_else(|| {
            "? outside a செயல் returning a result — analyse should have refused it".to_string()
        })?;
        let site = self.site(operation)?;
        unsafe {
            let ok = self.result_ok(result);
            let fail = self.block("try_fail");
            let go_on = self.block("try_ok");
            LLVMBuildCondBr(self.builder, ok, go_on, fail);
            LLVMPositionBuilderAtEnd(self.builder, fail);
            // The caller's result: a தவறு with the same text.
            LLVMBuildStore(self.builder, LLVMConstNull(self.result_type()), out);
            let from = self.result_payload(result);
            let into = self.result_payload(out);
            self.call("artino_text_copy", &mut [into, from]);
            self.end_lifetimes();
            LLVMBuildRetVoid(self.builder);
            LLVMPositionBuilderAtEnd(self.builder, go_on);
            Ok(self.unwrap(result, inner, site))
        }
    }

    /// An index as a whole number, and whether it is inside `0..n`.
    unsafe fn bounds(&mut self, position: LLVMValueRef, n: u16) -> (LLVMValueRef, LLVMValueRef) {
        unsafe {
            let whole = LLVMBuildSDiv(self.builder, position, self.num(1000), c("index").as_ptr());
            // Unsigned, so a negative index is also outside.
            let inside = LLVMBuildICmp(
                self.builder,
                LLVMIntPredicate::LLVMIntULT,
                whole,
                LLVMConstInt(self.i64(), n as u64, 0),
                c("inside").as_ptr(),
            );
            (whole, inside)
        }
    }

    unsafe fn element(
        &mut self,
        array: LLVMValueRef,
        elem: Elem,
        n: u16,
        index: LLVMValueRef,
    ) -> LLVMValueRef {
        unsafe {
            let mut indices = [LLVMConstInt(self.i64(), 0, 0), index];
            LLVMBuildInBoundsGEP2(
                self.builder,
                self.storage(Ty::Array(elem, n)),
                array,
                indices.as_mut_ptr(),
                2,
                c("element").as_ptr(),
            )
        }
    }

    /// அச்சு: every piece of the & chain is evaluated first, in order, and
    /// only then printed — as the VM builds the whole line before printing it,
    /// so a function called in the line that prints something itself comes out
    /// before the line, not in the middle of it.
    fn print(&mut self, expr: &Expr) -> Result<(), String> {
        let mut pieces: Vec<&Expr> = Vec::new();
        flatten(expr, &mut pieces);
        let mut ready = Vec::new();
        for piece in pieces {
            ready.push(match piece {
                Expr::String(text) => (self.program_text(text)?, None),
                other => {
                    let (value, ty) = self.expr(other)?;
                    (value, Some(ty))
                }
            });
        }
        for (value, ty) in ready {
            match ty {
                None => {
                    self.call("artino_print_text", &mut [value]);
                }
                Some(ty) => self.print_value(value, ty)?,
            }
        }
        Ok(())
    }

    /// One value, printed as the VM's Display writes it.
    fn print_value(&mut self, value: LLVMValueRef, ty: Ty) -> Result<(), String> {
        let i16 = unsafe { LLVMInt16TypeInContext(self.context) };
        match ty {
            Ty::Num => {
                self.call("artino_print_num", &mut [value]);
            }
            Ty::Bool => unsafe {
                let wide = LLVMBuildZExt(self.builder, value, self.i32(), c("flag").as_ptr());
                self.call("artino_print_bool", &mut [wide]);
            },
            Ty::Text => {
                self.call("artino_print_str", &mut [value]);
            }
            Ty::Array(Elem::Num, n) | Ty::Array(Elem::Bool, n) => unsafe {
                let count = LLVMConstInt(i16, n as u64, 0);
                let name = if ty == Ty::Array(Elem::Num, n) {
                    "artino_print_num_array"
                } else {
                    "artino_print_bool_array"
                };
                self.call(name, &mut [value, count]);
            },
            Ty::Array(elem, n) => {
                // [a, b, c], each item printed as itself.
                let open = self.program_text("[")?;
                let comma = self.program_text(", ")?;
                let close = self.program_text("]")?;
                self.call("artino_print_text", &mut [open]);
                for index in 0..n {
                    if index > 0 {
                        self.call("artino_print_text", &mut [comma]);
                    }
                    let element = unsafe {
                        self.element(value, elem, n, LLVMConstInt(self.i64(), index as u64, 0))
                    };
                    let item = if Self::is_buffer(elem.ty()) {
                        element
                    } else {
                        unsafe {
                            LLVMBuildLoad2(
                                self.builder,
                                self.elem_type(elem),
                                element,
                                c("item").as_ptr(),
                            )
                        }
                    };
                    self.print_value(item, elem.ty())?;
                }
                self.call("artino_print_text", &mut [close]);
            }
            Ty::Shape(id) => {
                // கடன்{அசல்: 1, பெயர்: x} — fields in sorted order, as the VM
                // walks a record, whatever order they were declared in.
                let shape = self.shapes[id as usize].clone();
                let mut order: Vec<usize> = (0..shape.fields.len()).collect();
                order.sort_by(|a, b| shape.fields[*a].0.cmp(&shape.fields[*b].0));
                let open = self.program_text(&format!("{}{{", shape.name))?;
                self.call("artino_print_text", &mut [open]);
                for (position, index) in order.into_iter().enumerate() {
                    let (name, field_ty) = shape.fields[index].clone();
                    let label = self.program_text(&format!(
                        "{}{}: ",
                        if position > 0 { ", " } else { "" },
                        name
                    ))?;
                    self.call("artino_print_text", &mut [label]);
                    let at = unsafe {
                        LLVMBuildStructGEP2(
                            self.builder,
                            self.shape_types[id as usize],
                            value,
                            index as u32,
                            c("field").as_ptr(),
                        )
                    };
                    let item = if Self::is_buffer(field_ty) {
                        at
                    } else {
                        unsafe {
                            LLVMBuildLoad2(
                                self.builder,
                                self.storage(field_ty),
                                at,
                                c("field_value").as_ptr(),
                            )
                        }
                    };
                    self.print_value(item, field_ty)?;
                }
                let close = self.program_text("}")?;
                self.call("artino_print_text", &mut [close]);
            }
            Ty::Result(inner) => {
                let kind = match inner {
                    Inner::Num => 0,
                    Inner::Bool => 1,
                    Inner::Text => 2,
                    Inner::Nothing | Inner::Unknown => 3,
                };
                let kind = unsafe { LLVMConstInt(self.i32(), kind, 0) };
                self.call("artino_print_result", &mut [value, kind]);
            }
            Ty::Void => return Err("printing nothing — analyse should have refused it".to_string()),
        }
        Ok(())
    }

    /// The address of a record's field, and its type.
    fn field_at(
        &mut self,
        record: LLVMValueRef,
        id: u16,
        field: &str,
    ) -> Result<(LLVMValueRef, Ty), String> {
        let (index, ty) = self.shapes[id as usize].field(field).ok_or_else(|| {
            format!(
                "{} has no field {} — analyse should have refused it",
                self.shapes[id as usize].name, field
            )
        })?;
        let at = unsafe {
            LLVMBuildStructGEP2(
                self.builder,
                self.shape_types[id as usize],
                record,
                index as u32,
                c(field).as_ptr(),
            )
        };
        Ok((at, ty))
    }

    // --- expressions -------------------------------------------------------------------

    fn expr(&mut self, expr: &Expr) -> Result<(LLVMValueRef, Ty), String> {
        unsafe {
            match expr {
                Expr::Number(n) => Ok((self.num(analyse::scaled(n)?), Ty::Num)),
                Expr::Boolean(b) => Ok((LLVMConstInt(self.i1(), u64::from(*b), 0), Ty::Bool)),
                Expr::String(_) | Expr::Concat { .. } => self.text(expr),
                Expr::Variable(name) => {
                    let (slot, ty) = self
                        .slot(name)
                        .ok_or_else(|| format!("{} has no storage — an artino bug", name))?;
                    if self.in_flash(name) {
                        // Flash is not RAM on AVR: read into a temporary first.
                        let copy = self.temp(ty, name);
                        self.copy_flash(copy, slot, ty);
                        return Ok((copy, ty));
                    }
                    match ty {
                        // A buffer is read through its address; assigning copies.
                        ty if Self::is_buffer(ty) => Ok((slot, ty)),
                        _ => Ok((
                            LLVMBuildLoad2(self.builder, self.storage(ty), slot, c(name).as_ptr()),
                            ty,
                        )),
                    }
                }
                Expr::BinaryOp { op, left, right } => {
                    if let Some(constant) = folded(expr) {
                        return Ok((self.num(constant), Ty::Num));
                    }
                    // By a whole number written in the source: exact, so only
                    // overflow to watch for, and the general multiply — 1.7 KB
                    // on AVR — is left out when nothing else needs it.
                    if op == "*" {
                        let by_whole = match (whole_constant(left), whole_constant(right)) {
                            (_, Some(k)) => Some((left, k)),
                            (Some(k), _) => Some((right, k)),
                            _ => None,
                        };
                        if let Some((other, k)) = by_whole {
                            let (a, _) = self.expr(other)?;
                            let site = self.site(expr)?;
                            let k = LLVMConstInt(self.i32(), k as u64, 1);
                            return Ok((
                                self.call("artino_num_mul_whole", &mut [a, k, site]),
                                Ty::Num,
                            ));
                        }
                    }
                    let runtime = match op.as_str() {
                        "+" => "artino_num_add",
                        "-" => "artino_num_sub",
                        "*" => "artino_num_mul",
                        "/" => "artino_num_div",
                        other => return Err(format!("the operator {} — not in artino yet", other)),
                    };
                    let (a, _) = self.expr(left)?;
                    let (b, _) = self.expr(right)?;
                    let site = self.site(expr)?;
                    Ok((self.call(runtime, &mut [a, b, site]), Ty::Num))
                }
                Expr::Comparison { op, left, right } => {
                    // பலகை() == "pico" is decided now, so the other board's
                    // branch is not in the firmware at all.
                    if let (Some(a), Some(b), "==" | "!=") =
                        (self.known_text(left), self.known_text(right), op.as_str())
                    {
                        let same = (a == b) == (op == "==");
                        return Ok((LLVMConstInt(self.i1(), u64::from(same), 0), Ty::Bool));
                    }
                    let (a, ty) = self.expr(left)?;
                    let (b, _) = self.expr(right)?;
                    if ty == Ty::Text {
                        let same = self.call("artino_text_equal", &mut [a, b]);
                        let predicate = if op == "==" {
                            LLVMIntPredicate::LLVMIntNE
                        } else {
                            LLVMIntPredicate::LLVMIntEQ
                        };
                        return Ok((
                            LLVMBuildICmp(
                                self.builder,
                                predicate,
                                same,
                                LLVMConstNull(self.i32()),
                                c("text_same").as_ptr(),
                            ),
                            Ty::Bool,
                        ));
                    }
                    let predicate = match (op.as_str(), ty) {
                        ("==", _) => LLVMIntPredicate::LLVMIntEQ,
                        ("!=", _) => LLVMIntPredicate::LLVMIntNE,
                        ("<", Ty::Num) => LLVMIntPredicate::LLVMIntSLT,
                        ("<=", Ty::Num) => LLVMIntPredicate::LLVMIntSLE,
                        (">", Ty::Num) => LLVMIntPredicate::LLVMIntSGT,
                        (">=", Ty::Num) => LLVMIntPredicate::LLVMIntSGE,
                        (other, _) => {
                            return Err(format!("the comparison {} — not in artino yet", other));
                        }
                    };
                    Ok((
                        LLVMBuildICmp(self.builder, predicate, a, b, c("compare").as_ptr()),
                        Ty::Bool,
                    ))
                }
                Expr::Logical { op, left, right } => self.logical(op, left, right),
                Expr::Not(inner) => {
                    let (value, _) = self.expr(inner)?;
                    Ok((
                        LLVMBuildNot(self.builder, value, c("not").as_ptr()),
                        Ty::Bool,
                    ))
                }
                Expr::ArrayLiteral(items) => {
                    let mut values = Vec::new();
                    for item in items {
                        values.push(self.expr(item)?);
                    }
                    let elem = elem_of(values[0].1)?;
                    let n = values.len() as u16;
                    let ty = Ty::Array(elem, n);
                    let array = self.temp(ty, "array");
                    for (index, (value, _)) in values.into_iter().enumerate() {
                        let element =
                            self.element(array, elem, n, LLVMConstInt(self.i64(), index as u64, 0));
                        self.copy(elem.ty(), element, value);
                    }
                    Ok((array, ty))
                }
                Expr::Index { base, index } => {
                    let from_flash = match base.as_ref() {
                        Expr::Variable(name) if self.in_flash(name) => self
                            .slot(name)
                            .filter(|(_, ty)| matches!(ty, Ty::Array(elem, _) if !Self::is_buffer(elem.ty()))),
                        _ => None,
                    };
                    let flash_text = match base.as_ref() {
                        Expr::Variable(name) if self.in_flash(name) => {
                            self.slot(name).filter(|(_, ty)| *ty == Ty::Text)
                        }
                        _ => None,
                    };
                    let (array, ty) = match from_flash.or(flash_text) {
                        Some(global) => global,
                        None => self.expr(base)?,
                    };
                    if ty == Ty::Text {
                        let (position, _) = self.expr(index)?;
                        let site = self.site(expr)?;
                        let whole = LLVMBuildSDiv(
                            self.builder,
                            position,
                            self.num(1000),
                            c("index").as_ptr(),
                        );
                        let whole = LLVMBuildTrunc(
                            self.builder,
                            whole,
                            self.i32(),
                            c("letter_at").as_ptr(),
                        );
                        // A letter, not a whole text: 16 bytes, not 49.
                        let letter =
                            self.temp_of(LLVMArrayType2(self.i8(), LETTER_BYTES as u64), "letter");
                        // From flash, the text is copied inside the call, so the
                        // copy is on the stack only while the letter is found.
                        let reader = if flash_text.is_some() {
                            "artino_text_letter_flash"
                        } else {
                            "artino_text_letter"
                        };
                        self.call(reader, &mut [array, whole, letter, site]);
                        return Ok((letter, Ty::Text));
                    }
                    let Ty::Array(elem, n) = ty else {
                        return Err(
                            "[…] on something not an array — analyse should have refused it"
                                .to_string(),
                        );
                    };
                    let (position, _) = self.expr(index)?;
                    let site = self.site(expr)?;
                    let (whole, inside) = self.bounds(position, n);
                    // Read at a safe place either way; report, and give zero, when outside.
                    let safe = LLVMBuildSelect(
                        self.builder,
                        inside,
                        whole,
                        LLVMConstInt(self.i64(), 0, 0),
                        c("safe").as_ptr(),
                    );
                    let element = self.element(array, elem, n, safe);
                    let loaded = if Self::is_buffer(elem.ty()) {
                        element
                    } else if from_flash.is_some() {
                        let item = self.temp(elem.ty(), "item");
                        self.copy_flash(item, element, elem.ty());
                        LLVMBuildLoad2(self.builder, self.elem_type(elem), item, c("item").as_ptr())
                    } else {
                        LLVMBuildLoad2(
                            self.builder,
                            self.elem_type(elem),
                            element,
                            c("item").as_ptr(),
                        )
                    };
                    let refuse = self.block("index_bad");
                    let done = self.block("index_done");
                    LLVMBuildCondBr(self.builder, inside, done, refuse);
                    LLVMPositionBuilderAtEnd(self.builder, refuse);
                    self.call("artino_report_index", &mut [site]);
                    LLVMBuildBr(self.builder, done);
                    LLVMPositionBuilderAtEnd(self.builder, done);
                    let zero = if Self::is_buffer(elem.ty()) {
                        // An empty buffer of the element's kind.
                        let empty = self.temp(elem.ty(), "outside");
                        LLVMBuildStore(self.builder, LLVMConstNull(self.elem_type(elem)), empty);
                        empty
                    } else {
                        LLVMConstNull(self.elem_type(elem))
                    };
                    Ok((
                        LLVMBuildSelect(self.builder, inside, loaded, zero, c("read").as_ptr()),
                        elem.ty(),
                    ))
                }
                Expr::ShapeLiteral {
                    shape,
                    fields,
                    base,
                    ..
                } => {
                    let id = self
                        .shapes
                        .iter()
                        .position(|s| &s.name == shape)
                        .ok_or_else(|| {
                            format!("{} is not a வடிவம் — analyse should have refused it", shape)
                        })? as u16;
                    let record = self.temp(Ty::Shape(id), "record");
                    match base {
                        Some(base) => {
                            let (source, _) = self.expr(base)?;
                            self.copy(Ty::Shape(id), record, source);
                        }
                        None => {
                            LLVMBuildStore(
                                self.builder,
                                LLVMConstNull(self.shape_types[id as usize]),
                                record,
                            );
                        }
                    }
                    for (field, value) in fields {
                        let (value, _) = self.expr(value)?;
                        let (at, field_ty) = self.field_at(record, id, field)?;
                        self.copy(field_ty, at, value);
                    }
                    Ok((record, Ty::Shape(id)))
                }
                Expr::Field { base, name, .. } => {
                    let (record, ty) = self.expr(base)?;
                    let Ty::Shape(id) = ty else {
                        return Err(format!(
                            ".{} on something not a record — analyse should have refused it",
                            name
                        ));
                    };
                    let (at, field_ty) = self.field_at(record, id, name)?;
                    if Self::is_buffer(field_ty) {
                        Ok((at, field_ty))
                    } else {
                        Ok((
                            LLVMBuildLoad2(
                                self.builder,
                                self.storage(field_ty),
                                at,
                                c(name).as_ptr(),
                            ),
                            field_ty,
                        ))
                    }
                }
                Expr::MethodCall {
                    receiver,
                    name,
                    args,
                    ..
                } => {
                    // கடன்.புதிது(…): the shape's own function.
                    if let Expr::Variable(shape) = receiver.as_ref() {
                        if self.slot(shape).is_none()
                            && self.shapes.iter().any(|s| &s.name == shape)
                        {
                            let full = crate::vm::shape::method_function(shape, name);
                            let mut values = Vec::new();
                            for arg in args {
                                values.push(self.expr(arg)?.0);
                            }
                            return self.call_function(&full, values);
                        }
                    }
                    let (record, ty) = self.expr(receiver)?;
                    let Ty::Shape(id) = ty else {
                        return Err(format!(
                            ".{}(…) on something not a record — analyse should have refused it",
                            name
                        ));
                    };
                    let full =
                        crate::vm::shape::method_function(&self.shapes[id as usize].name, name);
                    let mut values = vec![record];
                    for arg in args {
                        values.push(self.expr(arg)?.0);
                    }
                    self.call_function(&full, values)
                }
                Expr::Call { name, args } => self.call_expr(name, args),
                Expr::Try(inner) => self.try_expr(expr, inner),
                other => Err(format!(
                    "{:?} reached the emitter — analyse should have refused it",
                    other
                )),
            }
        }
    }

    /// Text as a value: a literal copied out of flash, or an & chain built
    /// piece by piece into a buffer. Cutting to fit is reported, once.
    fn text(&mut self, expr: &Expr) -> Result<(LLVMValueRef, Ty), String> {
        self.text_into(expr, None)
    }

    /// Text built into `into` when given — a buffer nothing in the chain can
    /// read, such as a function's result — rather than a temporary copied there.
    fn text_into(
        &mut self,
        expr: &Expr,
        into: Option<LLVMValueRef>,
    ) -> Result<(LLVMValueRef, Ty), String> {
        let mut pieces: Vec<&Expr> = Vec::new();
        flatten(expr, &mut pieces);
        let buffer = self.join(expr, &pieces, into, true)?;
        Ok((buffer, Ty::Text))
    }

    /// Append `pieces` to `into` (cleared first when `clear`), or to a new
    /// temporary. Every piece is worked out before any is appended.
    fn join(
        &mut self,
        whole: &Expr,
        pieces: &[&Expr],
        into: Option<LLVMValueRef>,
        clear: bool,
    ) -> Result<LLVMValueRef, String> {
        let mut ready = Vec::new();
        for piece in pieces {
            ready.push(match *piece {
                Expr::String(text) => (self.program_text(text)?, None),
                other => {
                    let (value, ty) = self.expr(other)?;
                    (value, Some(ty))
                }
            });
        }
        let site = self.site(whole)?;
        let buffer = match into {
            Some(buffer) => buffer,
            None => self.temp(Ty::Text, "text"),
        };
        if clear {
            self.call("artino_text_clear", &mut [buffer]);
        }
        for (value, ty) in ready {
            match ty {
                None => {
                    self.call("artino_text_append_flash", &mut [buffer, value, site]);
                }
                Some(Ty::Text) => {
                    self.call("artino_text_append", &mut [buffer, value, site]);
                }
                Some(Ty::Num) => {
                    self.call("artino_text_append_num", &mut [buffer, value, site]);
                }
                Some(Ty::Bool) => unsafe {
                    let wide = LLVMBuildZExt(self.builder, value, self.i32(), c("flag").as_ptr());
                    self.call("artino_text_append_bool", &mut [buffer, wide, site]);
                },
                Some(other) => {
                    return Err(format!(
                        "{} joined into text — analyse should have refused it",
                        other.name()
                    ));
                }
            }
        }
        Ok(buffer)
    }

    /// மற்றும் / அல்லது, short-circuiting: the right side runs only when the
    /// left has not already decided.
    fn logical(
        &mut self,
        op: &str,
        left: &Expr,
        right: &Expr,
    ) -> Result<(LLVMValueRef, Ty), String> {
        let stops_on = match op {
            "&&" => false,
            "||" => true,
            other => {
                return Err(format!(
                    "the logical operator {} — not in artino yet",
                    other
                ));
            }
        };
        unsafe {
            let (decided, _) = self.expr(left)?;
            let from = LLVMGetInsertBlock(self.builder);
            let right_block = self.block("logical_right");
            let done = self.block("logical_done");
            if stops_on {
                LLVMBuildCondBr(self.builder, decided, done, right_block);
            } else {
                LLVMBuildCondBr(self.builder, decided, right_block, done);
            }
            LLVMPositionBuilderAtEnd(self.builder, right_block);
            let (answer, _) = self.expr(right)?;
            let right_end = LLVMGetInsertBlock(self.builder);
            LLVMBuildBr(self.builder, done);
            LLVMPositionBuilderAtEnd(self.builder, done);
            let phi = LLVMBuildPhi(self.builder, self.i1(), c("logical").as_ptr());
            let mut values = [LLVMConstInt(self.i1(), u64::from(stops_on), 0), answer];
            let mut blocks = [from, right_end];
            LLVMAddIncoming(phi, values.as_mut_ptr(), blocks.as_mut_ptr(), 2);
            Ok((phi, Ty::Bool))
        }
    }

    fn call_expr(&mut self, name: &str, args: &[Expr]) -> Result<(LLVMValueRef, Ty), String> {
        if let Some(builtin) = analyse::builtin(name) {
            return self.builtin(builtin, name, args);
        }
        let mut values = Vec::new();
        for arg in args {
            values.push(self.expr(arg)?.0);
        }
        if let Some(index) = self.externs.iter().position(|e| e.etamil == name) {
            let call = Expr::Call {
                name: name.to_string(),
                args: args.to_vec(),
            };
            return self.call_extern(index, &call, values);
        }
        unsafe {
            if let Some(board) = analyse::intrinsic(name) {
                let mut converted: Vec<LLVMValueRef> = Vec::new();
                for (arg, value) in board.args.iter().zip(values) {
                    converted.push(match arg {
                        // A pin: the whole number, as int32.
                        Arg::Int => {
                            let whole = LLVMBuildSDiv(
                                self.builder,
                                value,
                                self.num(1000),
                                c("whole").as_ptr(),
                            );
                            LLVMBuildTrunc(self.builder, whole, self.i32(), c("pin").as_ptr())
                        }
                        Arg::Flag => {
                            LLVMBuildZExt(self.builder, value, self.i32(), c("flag").as_ptr())
                        }
                    });
                }
                let result = self.call(board.shim, &mut converted);
                return Ok(match board.ret {
                    Ret::Void => (result, Ty::Void),
                    Ret::Int => {
                        let wide =
                            LLVMBuildSExt(self.builder, result, self.i64(), c("wide").as_ptr());
                        (
                            LLVMBuildMul(self.builder, wide, self.num(1000), c("scaled").as_ptr()),
                            Ty::Num,
                        )
                    }
                    Ret::Millis => {
                        let wide =
                            LLVMBuildZExt(self.builder, result, self.i64(), c("wide").as_ptr());
                        (
                            LLVMBuildMul(self.builder, wide, self.num(1000), c("scaled").as_ptr()),
                            Ty::Num,
                        )
                    }
                    Ret::Flag => (
                        LLVMBuildICmp(
                            self.builder,
                            LLVMIntPredicate::LLVMIntNE,
                            result,
                            LLVMConstNull(self.i32()),
                            c("high").as_ptr(),
                        ),
                        Ty::Bool,
                    ),
                });
            }
            return self.call_function(name, values);
        }
    }

    /// A manifest function: its shim, with each value as C++ takes it.
    fn call_extern(
        &mut self,
        index: usize,
        call: &Expr,
        values: Vec<LLVMValueRef>,
    ) -> Result<(LLVMValueRef, Ty), String> {
        use super::manifest::{Extern, Kind};
        let function = self.externs[index].clone();
        unsafe {
            let c_type = |this: &Self, kind: Kind| match kind {
                Kind::Int | Kind::Bool => this.i32(),
                Kind::Num => this.i64(),
                Kind::Text => this.ptr(),
                Kind::Void => this.void(),
            };
            let (shim, kind) = match self.extern_shims.get(&index) {
                Some(found) => *found,
                None => {
                    let mut params: Vec<LLVMTypeRef> = Vec::new();
                    let ret = if function.returns == Kind::Text {
                        params.push(self.ptr());
                        self.void()
                    } else {
                        c_type(self, function.returns)
                    };
                    params.extend(function.args.iter().map(|k| c_type(self, *k)));
                    let kind = LLVMFunctionType(ret, params.as_mut_ptr(), params.len() as u32, 0);
                    let shim =
                        LLVMAddFunction(self.module, c(&Extern::symbol(index)).as_ptr(), kind);
                    self.extern_shims.insert(index, (shim, kind));
                    (shim, kind)
                }
            };
            let mut converted: Vec<LLVMValueRef> = Vec::new();
            let out = (function.returns == Kind::Text).then(|| self.temp(Ty::Text, "from_cpp"));
            if let Some(out) = out {
                converted.push(out);
            }
            let mut site = None;
            for (kind, value) in function.args.iter().zip(values) {
                converted.push(match kind {
                    // A fraction or a number past int32 is reported as it crosses.
                    Kind::Int => {
                        let at = match site {
                            Some(at) => at,
                            None => *site.insert(self.site(call)?),
                        };
                        self.call("artino_to_int", &mut [value, at])
                    }
                    Kind::Bool => {
                        LLVMBuildZExt(self.builder, value, self.i32(), c("flag").as_ptr())
                    }
                    _ => value,
                });
            }
            let result = LLVMBuildCall2(
                self.builder,
                kind,
                shim,
                converted.as_mut_ptr(),
                converted.len() as u32,
                c("").as_ptr(),
            );
            Ok(match function.returns {
                Kind::Void => (result, Ty::Void),
                Kind::Int => {
                    let wide = LLVMBuildSExt(self.builder, result, self.i64(), c("wide").as_ptr());
                    (
                        LLVMBuildMul(self.builder, wide, self.num(1000), c("scaled").as_ptr()),
                        Ty::Num,
                    )
                }
                Kind::Num => (result, Ty::Num),
                Kind::Bool => (
                    LLVMBuildICmp(
                        self.builder,
                        LLVMIntPredicate::LLVMIntNE,
                        result,
                        LLVMConstNull(self.i32()),
                        c("yes").as_ptr(),
                    ),
                    Ty::Bool,
                ),
                Kind::Text => (out.expect("made above"), Ty::Text),
            })
        }
    }

    /// Call a செயல் or a method: a buffer it returns comes back through a
    /// temporary the call passes first.
    fn call_function(
        &mut self,
        name: &str,
        values: Vec<LLVMValueRef>,
    ) -> Result<(LLVMValueRef, Ty), String> {
        self.call_function_into(name, values, None)
    }

    /// `x = f(…)` where f's result can be written straight into x: a local of
    /// f's type, in a செயல், that the arguments do not mention.
    fn returns_into(&self, target: &str, called: &str, args: &[Expr]) -> bool {
        let Some((_, _, ret)) = self.functions.get(called) else {
            return false;
        };
        self.returns.is_some()
            && Self::returns_buffer(*ret)
            && analyse::builtin(called).is_none()
            && self.locals.get(target).is_some_and(|(_, ty)| ty == ret)
            && !args.iter().any(|arg| analyse::mentions(arg, target))
    }

    fn call_function_into(
        &mut self,
        name: &str,
        mut values: Vec<LLVMValueRef>,
        into: Option<LLVMValueRef>,
    ) -> Result<(LLVMValueRef, Ty), String> {
        unsafe {
            let (function, kind, ret) = *self.functions.get(name).ok_or_else(|| {
                format!(
                    "{}(…) has no definition — analyse should have refused it",
                    name
                )
            })?;
            if Self::returns_buffer(ret) {
                let out = match into {
                    Some(slot) => slot,
                    None => self.temp(ret, "returned"),
                };
                values.insert(0, out);
                LLVMBuildCall2(
                    self.builder,
                    kind,
                    function,
                    values.as_mut_ptr(),
                    values.len() as u32,
                    c("").as_ptr(),
                );
                return Ok((out, ret));
            }
            let value = LLVMBuildCall2(
                self.builder,
                kind,
                function,
                values.as_mut_ptr(),
                values.len() as u32,
                c("").as_ptr(),
            );
            Ok((value, ret))
        }
    }

    fn builtin(
        &mut self,
        builtin: Builtin,
        name: &str,
        args: &[Expr],
    ) -> Result<(LLVMValueRef, Ty), String> {
        match builtin {
            Builtin::Length => {
                // An array's length is part of its type; the argument still runs.
                let (value, ty) = self.expr(&args[0])?;
                match ty {
                    Ty::Array(_, n) => Ok((self.num(n as i64 * 1000), Ty::Num)),
                    Ty::Text => {
                        let call = Expr::Call {
                            name: name.to_string(),
                            args: args.to_vec(),
                        };
                        let site = self.site(&call)?;
                        let count = self.call("artino_text_letters", &mut [value, site]);
                        unsafe {
                            let wide = LLVMBuildSExt(
                                self.builder,
                                count,
                                self.i64(),
                                c("letters").as_ptr(),
                            );
                            Ok((
                                LLVMBuildMul(
                                    self.builder,
                                    wide,
                                    self.num(1000),
                                    c("scaled").as_ptr(),
                                ),
                                Ty::Num,
                            ))
                        }
                    }
                    other => Err(format!(
                        "நீளம் of {} — analyse should have refused it",
                        other.name()
                    )),
                }
            }
            Builtin::ToText => {
                let call = Expr::Call {
                    name: name.to_string(),
                    args: args.to_vec(),
                };
                let (value, ty) = self.expr(&args[0])?;
                let site = self.site(&call)?;
                let buffer = self.temp(Ty::Text, "as_text");
                self.call("artino_text_clear", &mut [buffer]);
                match ty {
                    Ty::Num => {
                        self.call("artino_text_append_num", &mut [buffer, value, site]);
                    }
                    Ty::Bool => unsafe {
                        let wide =
                            LLVMBuildZExt(self.builder, value, self.i32(), c("flag").as_ptr());
                        self.call("artino_text_append_bool", &mut [buffer, wide, site]);
                    },
                    Ty::Text => {
                        self.call("artino_text_append", &mut [buffer, value, site]);
                    }
                    other => {
                        return Err(format!(
                            "{} of {} — analyse should have refused it",
                            name,
                            other.name()
                        ));
                    }
                }
                Ok((buffer, Ty::Text))
            }
            Builtin::Fill => {
                let (value, ty) = self.expr(&args[0])?;
                let count = analyse::fill_count(&args[1])?;
                let elem = elem_of(ty)?;
                let array_ty = Ty::Array(elem, count);
                let array = self.temp(array_ty, "filled");
                let counter = self.temp(Ty::Num, "fill_index");
                unsafe {
                    LLVMBuildStore(self.builder, self.num(0), counter);
                    let head = self.block("fill");
                    let inside = self.block("fill_body");
                    let after = self.block("fill_done");
                    LLVMBuildBr(self.builder, head);
                    LLVMPositionBuilderAtEnd(self.builder, head);
                    let at = LLVMBuildLoad2(self.builder, self.i64(), counter, c("at").as_ptr());
                    let more = LLVMBuildICmp(
                        self.builder,
                        LLVMIntPredicate::LLVMIntSLT,
                        at,
                        LLVMConstInt(self.i64(), count as u64, 0),
                        c("more").as_ptr(),
                    );
                    LLVMBuildCondBr(self.builder, more, inside, after);
                    LLVMPositionBuilderAtEnd(self.builder, inside);
                    let element = self.element(array, elem, count, at);
                    self.copy(elem.ty(), element, value);
                    let next = LLVMBuildAdd(
                        self.builder,
                        at,
                        LLVMConstInt(self.i64(), 1, 0),
                        c("next").as_ptr(),
                    );
                    LLVMBuildStore(self.builder, next, counter);
                    LLVMBuildBr(self.builder, head);
                    LLVMPositionBuilderAtEnd(self.builder, after);
                }
                Ok((array, array_ty))
            }
            _ => self.builtin_b32(builtin, name, args),
        }
    }

    fn builtin_b32(
        &mut self,
        builtin: Builtin,
        name: &str,
        args: &[Expr],
    ) -> Result<(LLVMValueRef, Ty), String> {
        let call = Expr::Call {
            name: name.to_string(),
            args: args.to_vec(),
        };
        unsafe {
            match builtin {
                Builtin::Ok => {
                    let (value, ty) = self.expr(&args[0])?;
                    let result = self.make_result(true, value, ty);
                    let inner = match ty {
                        Ty::Num => Inner::Num,
                        Ty::Bool => Inner::Bool,
                        _ => Inner::Text,
                    };
                    Ok((result, Ty::Result(inner)))
                }
                Builtin::Err => {
                    let (value, _) = self.expr(&args[0])?;
                    Ok((
                        self.make_result(false, value, Ty::Text),
                        Ty::Result(Inner::Unknown),
                    ))
                }
                Builtin::IsOk | Builtin::IsErr => {
                    let (result, _) = self.expr(&args[0])?;
                    let ok = self.result_ok(result);
                    if builtin == Builtin::IsOk {
                        Ok((ok, Ty::Bool))
                    } else {
                        Ok((
                            LLVMBuildNot(self.builder, ok, c("is_err").as_ptr()),
                            Ty::Bool,
                        ))
                    }
                }
                Builtin::Unwrap => {
                    let (result, ty) = self.expr(&args[0])?;
                    let Ty::Result(inner) = ty else {
                        return Err(format!(
                            "{} of something not a result — analyse should have refused it",
                            name
                        ));
                    };
                    let site = self.site(&call)?;
                    Ok(self.unwrap(result, inner, site))
                }
                Builtin::UnwrapErr => {
                    let (result, _) = self.expr(&args[0])?;
                    let site = self.site(&call)?;
                    let ok = self.result_ok(result);
                    let payload = self.result_payload(result);
                    let refuse = self.block("unwrap_err_bad");
                    let done = self.block("unwrap_err_done");
                    LLVMBuildCondBr(self.builder, ok, refuse, done);
                    LLVMPositionBuilderAtEnd(self.builder, refuse);
                    self.call("artino_report_unwrap_err", &mut [site]);
                    LLVMBuildBr(self.builder, done);
                    LLVMPositionBuilderAtEnd(self.builder, done);
                    let empty = self.temp(Ty::Text, "empty");
                    self.call("artino_text_clear", &mut [empty]);
                    Ok((
                        LLVMBuildSelect(self.builder, ok, empty, payload, c("error").as_ptr()),
                        Ty::Text,
                    ))
                }
                Builtin::UnwrapOr => {
                    let (result, ty) = self.expr(&args[0])?;
                    let (fallback, fallback_ty) = self.expr(&args[1])?;
                    let ok = self.result_ok(result);
                    let payload = self.result_payload(result);
                    let held = match (ty, fallback_ty) {
                        (_, Ty::Text) => payload,
                        (_, other) => {
                            let load = LLVMBuildLoad2(
                                self.builder,
                                self.storage(other),
                                payload,
                                c("held").as_ptr(),
                            );
                            LLVMSetAlignment(load, 1);
                            load
                        }
                    };
                    Ok((
                        LLVMBuildSelect(self.builder, ok, held, fallback, c("or").as_ptr()),
                        fallback_ty,
                    ))
                }
                Builtin::ToNumber => {
                    let (text, _) = self.expr(&args[0])?;
                    let site = self.site(&call)?;
                    let result = self.temp(Ty::Result(Inner::Num), "number");
                    self.call("artino_text_to_number", &mut [text, result, site]);
                    Ok((result, Ty::Result(Inner::Num)))
                }
                Builtin::Floor | Builtin::Ceil | Builtin::Round => {
                    let (mode, places) = match builtin {
                        Builtin::Floor => (0, 0),
                        Builtin::Ceil => (1, 0),
                        _ => (2, analyse::places(&args[1])?),
                    };
                    self.rounded(&args[0], places, mode)
                }
                Builtin::Board => {
                    let flash = self.program_text(self.board)?;
                    let site = self.site(&call)?;
                    let buffer = self.temp(Ty::Text, "board");
                    self.call("artino_text_clear", &mut [buffer]);
                    self.call("artino_text_append_flash", &mut [buffer, flash, site]);
                    Ok((buffer, Ty::Text))
                }
                Builtin::SerialOpen
                | Builtin::SerialReadLine
                | Builtin::SerialWrite
                | Builtin::SerialWriteLine
                | Builtin::SerialClose => {
                    let (port, _) = self.expr(&args[0])?;
                    let port = self.whole_i32(port);
                    let inner = match builtin {
                        Builtin::SerialReadLine => Inner::Text,
                        Builtin::SerialClose => Inner::Nothing,
                        _ => Inner::Num,
                    };
                    let result = self.temp(Ty::Result(inner), "serial");
                    match builtin {
                        Builtin::SerialOpen => {
                            let (baud, _) = self.expr(&args[1])?;
                            let baud = self.whole_i32(baud);
                            self.call("artino_serial_open", &mut [port, baud, result]);
                        }
                        Builtin::SerialReadLine => {
                            let (wait, _) = self.expr(&args[1])?;
                            let wait = self.whole_i32(wait);
                            let site = self.site(&call)?;
                            self.call("artino_serial_read_line", &mut [port, wait, result, site]);
                        }
                        Builtin::SerialWrite | Builtin::SerialWriteLine => {
                            let (text, _) = self.expr(&args[1])?;
                            let newline = LLVMConstInt(
                                self.i32(),
                                u64::from(builtin == Builtin::SerialWriteLine),
                                0,
                            );
                            self.call("artino_serial_write", &mut [port, text, newline, result]);
                        }
                        _ => {
                            self.call("artino_serial_close", &mut [port, result]);
                        }
                    }
                    Ok((result, Ty::Result(inner)))
                }
                other => Err(format!("{:?} reached builtin_b32 — an artino bug", other)),
            }
        }
    }

    /// A number's whole part, as int32: a port, a baud rate, a wait.
    fn whole_i32(&mut self, value: LLVMValueRef) -> LLVMValueRef {
        unsafe {
            let whole = LLVMBuildSDiv(self.builder, value, self.num(1000), c("whole").as_ptr());
            LLVMBuildTrunc(self.builder, whole, self.i32(), c("int").as_ptr())
        }
    }

    /// தரை, மேல், வட்டமிடு. The author wrote this rounding, so it is done
    /// exactly and not reported: `தரை(a / b)` divides straight to the answer
    /// rather than rounding to three decimals first.
    fn rounded(
        &mut self,
        arg: &Expr,
        places: u32,
        mode: u64,
    ) -> Result<(LLVMValueRef, Ty), String> {
        unsafe {
            let mode = LLVMConstInt(self.i32(), mode, 0);
            if let Expr::BinaryOp { op, left, right } = arg {
                if (op == "/" || op == "*") && places <= 3 {
                    let (a, _) = self.expr(left)?;
                    let (b, _) = self.expr(right)?;
                    let site = self.site(arg)?;
                    let places = LLVMConstInt(self.i32(), places as u64, 0);
                    let runtime = if op == "/" {
                        "artino_num_div_round"
                    } else {
                        "artino_num_mul_round"
                    };
                    return Ok((self.call(runtime, &mut [a, b, places, mode, site]), Ty::Num));
                }
            }
            let (value, _) = self.expr(arg)?;
            if places >= 3 {
                // A board number already has at most three decimals.
                return Ok((value, Ty::Num));
            }
            let places = LLVMConstInt(self.i32(), places as u64, 0);
            Ok((
                self.call("artino_num_round", &mut [value, places, mode]),
                Ty::Num,
            ))
        }
    }
}

/// `+` and `-` of literals, computed now: `-5` arrives as `0 - 5`, and a
/// constant should not cost a call or a site. Only when exact and in range;
/// anything else goes to the runtime, which reports.
fn folded(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Number(n) => analyse::scaled(n).ok(),
        Expr::BinaryOp { op, left, right } => {
            let (a, b) = (folded(left)?, folded(right)?);
            let value = match op.as_str() {
                "+" => a.checked_add(b)?,
                "-" => a.checked_sub(b)?,
                _ => return None,
            };
            (value != i64::MIN).then_some(value)
        }
        _ => None,
    }
}

/// A whole number known now, small enough for int32: 16 in `x * 16`.
fn whole_constant(expr: &Expr) -> Option<i32> {
    let value = match expr {
        Expr::Number(n) => analyse::scaled(n).ok()?,
        other => folded(other)?,
    };
    (value % 1000 == 0)
        .then(|| i32::try_from(value / 1000).ok())
        .flatten()
}

/// The pieces of an & chain, left to right.
fn flatten<'a>(expr: &'a Expr, into: &mut Vec<&'a Expr>) {
    match expr {
        Expr::Concat { left, right } => {
            flatten(left, into);
            flatten(right, into);
        }
        other => into.push(other),
    }
}

/// What an array of values of this type holds.
fn elem_of(ty: Ty) -> Result<Elem, String> {
    match ty {
        Ty::Num => Ok(Elem::Num),
        Ty::Bool => Ok(Elem::Bool),
        Ty::Text => Ok(Elem::Text),
        Ty::Shape(id) => Ok(Elem::Shape(id)),
        other => Err(format!(
            "an array of {} — analyse should have refused it",
            other.name()
        )),
    }
}
