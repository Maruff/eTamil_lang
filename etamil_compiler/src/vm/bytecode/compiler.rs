// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
// Bytecode compiler: Converts AST to bytecode instructions
use crate::parser::{Expr, Param, Stmt};
use crate::vm::Value;
use crate::vm::bytecode::{Bytecode, FunctionInfo, Instruction, LambdaSource};
use rust_decimal::Decimal;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Numbers every anonymous செயல் this process compiles, across compilations.
///
/// Per-compilation numbering would reuse `#செயல்_0` in every program, and the
/// REPL compiles each line as a new program over a VM that keeps its variables:
/// a function value made on one line would then name a *different* body on the
/// next. A name that is never reused can at worst name nothing, which the VM
/// reports, rather than name the wrong thing, which it could not.
static NEXT_LAMBDA: AtomicUsize = AtomicUsize::new(0);

pub struct BytecodeCompiler {
    bytecode: Bytecode,
    /// Makes each ஒவ்வொரு loop's hidden variables unique.
    loop_id: usize,
    /// The locals of the function being compiled, as far as it has got:
    /// parameters, captures, and every name bound so far. `None` at the top
    /// level, where a name is a global and a செயல் reads it live rather than
    /// capturing it. This is what an anonymous செயல் captures from, and the
    /// LLVM backend keeps the same set, in the same order, for the same use.
    locals: Option<HashSet<String>>,
    /// The program's வடிவம்s, known before anything is compiled, because a
    /// shape may be used above the line that declares it.
    shapes: HashSet<String>,
    /// False when the program defines its own `இணை`, in which case the
    /// in-place append below would call something the author replaced.
    append_in_place: bool,
}

impl Default for BytecodeCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl BytecodeCompiler {
    pub fn new() -> Self {
        BytecodeCompiler {
            bytecode: Bytecode::new(),
            loop_id: 0,
            locals: None,
            shapes: HashSet::new(),
            append_in_place: true,
        }
    }

    /// The three spellings of the append builtin.
    const APPEND: [&'static str; 3] = ["இணை", "iNY", "_append"];

    /// Does this program define its own function called `இணை`?
    ///
    /// A user-defined function shadows a builtin at call time, so if one
    /// exists the peephole below would silently call the builtin instead of
    /// the author's function. Whole-program, because a definition anywhere —
    /// including inside another function — shadows every call.
    fn defines_own_append(statements: &[Stmt]) -> bool {
        statements.iter().any(|stmt| match stmt {
            Stmt::FunctionDef { name, body, .. } => {
                Self::APPEND.contains(&name.as_str()) || Self::defines_own_append(body)
            }
            Stmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                Self::defines_own_append(then_branch)
                    || else_branch
                        .as_ref()
                        .is_some_and(|stmts| Self::defines_own_append(stmts))
            }
            Stmt::Loop { body, .. } | Stmt::ForEach { body, .. } => Self::defines_own_append(body),
            _ => false,
        })
    }

    pub fn compile_statements(statements: Vec<Stmt>) -> Bytecode {
        Self::compile_with_lambdas(statements, &[])
    }

    /// Compile a program that must also hold these anonymous செயல்s, under
    /// the names they were first compiled with — the REPL's case, where a
    /// function value made on an earlier line is still held in a variable.
    pub fn compile_with_lambdas(statements: Vec<Stmt>, carried: &[LambdaSource]) -> Bytecode {
        let mut compiler = BytecodeCompiler::new();
        compiler.append_in_place = !Self::defines_own_append(&statements);
        compiler.bytecode.shapes = crate::vm::shape::from_program(&statements);
        compiler.shapes = compiler.bytecode.shapes.keys().cloned().collect();
        for lambda in carried {
            compiler.compile_function(
                lambda.name.clone(),
                lambda.captures.clone(),
                lambda.params.clone(),
                lambda.body.clone(),
            );
        }
        for stmt in statements {
            compiler.compile_stmt(stmt);
        }
        compiler.bytecode.push(Instruction::Halt);
        compiler.bytecode
    }

    fn compile_stmt(&mut self, stmt: Stmt) {
        // What this statement binds becomes a local once it has run, which is
        // when a செயல் written after it can capture it.
        let bound: Vec<String> = crate::parser::bound_names(&stmt)
            .into_iter()
            .map(str::to_string)
            .collect();
        self.compile_stmt_inner(stmt);
        self.note_locals(bound);
    }

    fn note_locals(&mut self, names: impl IntoIterator<Item = String>) {
        if let Some(locals) = self.locals.as_mut() {
            locals.extend(names);
        }
    }

    /// A function body, emitted inline and jumped over.
    ///
    /// `captures` are bound before `params` when it is called, from the values
    /// the function value carries.
    fn compile_function(
        &mut self,
        name: String,
        captures: Vec<String>,
        params: Vec<Param>,
        body: Vec<Stmt>,
    ) {
        let params: Vec<String> = params.into_iter().map(|param| param.name).collect();

        let jump_idx = self.bytecode.len();
        self.bytecode.push(Instruction::Jump(0)); // patched below

        let own: HashSet<String> = captures.iter().chain(params.iter()).cloned().collect();
        let outer_locals = self.locals.replace(own);

        let start = self.bytecode.len();
        for stmt in body {
            self.compile_stmt(stmt);
        }
        // Falling off the end returns nil.
        self.bytecode.push(Instruction::Push(Value::Null));
        self.bytecode.push(Instruction::Return);

        self.locals = outer_locals;

        let end = self.bytecode.len();
        self.bytecode.instructions[jump_idx] = Instruction::Jump(end);
        self.bytecode.functions.insert(
            name,
            FunctionInfo {
                start,
                params,
                captures,
            },
        );
    }

    fn compile_stmt_inner(&mut self, stmt: Stmt) {
        match stmt {
            // The declared type and நிலை are the checker's business, not the
            // VM's: by the time bytecode is emitted the program has already
            // been accepted, so there is nothing left to enforce here.
            Stmt::Assign { name, value, .. } => {
                // `x = இணை(x, v)` appends to x in place. Anything else —
                // a different destination, a longer expression around the
                // call, an author's own இணை — takes the copying path.
                if self.append_in_place
                    && let Expr::Call { name: called, args } = &value
                    && Self::APPEND.contains(&called.as_str())
                    && args.len() == 2
                    && matches!(&args[0], Expr::Variable(source) if *source == name)
                {
                    let mut args = args.clone();
                    let item = args.pop().expect("two arguments were just checked");
                    self.compile_expr(item);
                    self.bytecode.push(Instruction::AppendVar(name));
                    return;
                }
                // `x = x & expr` appends to x's string in place, for the
                // same reason and under the same conditions. Anything else —
                // a different destination, or the variable on the right —
                // takes the copying path.
                if let Expr::Concat { left, right } = &value
                    && matches!(&**left, Expr::Variable(source) if *source == name)
                {
                    self.compile_expr((**right).clone());
                    self.bytecode.push(Instruction::ConcatVar(name));
                    return;
                }
                self.compile_expr(value);
                self.bytecode.push(Instruction::StoreVar(name));
            }
            // The VM binds parameters by name; the declared types are the
            // checker's business and are already enforced by now. A named
            // செயல் captures nothing: it reads globals live, as it always has.
            Stmt::FunctionDef {
                name, params, body, ..
            } => self.compile_function(name, Vec::new(), params, body),
            // The shape itself was registered before compiling began. Its
            // methods are functions named `வடிவம்.முறை`, இது first when they
            // take one.
            Stmt::ShapeDef { name, methods, .. } => {
                for method in methods {
                    let function = crate::vm::shape::method_function(&name, &method.name);
                    self.compile_function(function, Vec::new(), method.params, method.body);
                }
            }
            Stmt::Return(value) => {
                match value {
                    Some(expr) => self.compile_expr(expr),
                    None => self.bytecode.push(Instruction::Push(Value::Null)),
                }
                self.bytecode.push(Instruction::Return);
            }
            Stmt::Expression(expr) => {
                // Evaluated for its effect; discard whatever it left behind.
                self.compile_expr(expr);
                self.bytecode.push(Instruction::Pop);
            }
            Stmt::SetIndex {
                name, index, value, ..
            } => {
                self.compile_expr(index);
                self.compile_expr(value);
                self.bytecode.push(Instruction::SetIndex(name));
            }
            Stmt::SetField {
                name, field, value, ..
            } => {
                self.compile_expr(value);
                self.bytecode.push(Instruction::SetField(name, field));
            }
            Stmt::Print(expr) => {
                self.compile_expr(expr);
                self.bytecode.push(Instruction::Print);
            }
            Stmt::Input(expr) => {
                self.bytecode.push(Instruction::Input);
                if let Expr::Variable(name) = expr {
                    self.bytecode.push(Instruction::StoreVar(name));
                }
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.compile_expr(condition);

                let jump_false_idx = self.bytecode.len();
                self.bytecode.push(Instruction::JumpIfFalse(0)); // Placeholder

                for stmt in then_branch {
                    self.compile_stmt(stmt);
                }

                match else_branch {
                    Some(else_stmts) => {
                        let jump_idx = self.bytecode.len();
                        self.bytecode.push(Instruction::Jump(0)); // Placeholder

                        // Patch jump_if_false to skip to else
                        let else_start = self.bytecode.len();
                        if let Instruction::JumpIfFalse(_) =
                            &mut self.bytecode.instructions[jump_false_idx]
                        {
                            self.bytecode.instructions[jump_false_idx] =
                                Instruction::JumpIfFalse(else_start);
                        }

                        for stmt in else_stmts {
                            self.compile_stmt(stmt);
                        }

                        // Patch final jump
                        let end = self.bytecode.len();
                        if let Instruction::Jump(_) = &mut self.bytecode.instructions[jump_idx] {
                            self.bytecode.instructions[jump_idx] = Instruction::Jump(end);
                        }
                    }
                    None => {
                        let end = self.bytecode.len();
                        if let Instruction::JumpIfFalse(_) =
                            &mut self.bytecode.instructions[jump_false_idx]
                        {
                            self.bytecode.instructions[jump_false_idx] =
                                Instruction::JumpIfFalse(end);
                        }
                    }
                }
            }
            Stmt::Loop { condition, body } => {
                let loop_start = self.bytecode.len();

                self.compile_expr(condition);
                let jump_false_idx = self.bytecode.len();
                self.bytecode.push(Instruction::JumpIfFalse(0)); // Placeholder

                for stmt in body {
                    self.compile_stmt(stmt);
                }

                self.bytecode.push(Instruction::Jump(loop_start));

                let end = self.bytecode.len();
                if let Instruction::JumpIfFalse(_) = &mut self.bytecode.instructions[jump_false_idx]
                {
                    self.bytecode.instructions[jump_false_idx] = Instruction::JumpIfFalse(end);
                }
            }
            // ovvoru item il collection { ... } is desugared into an index
            // loop over hidden variables. The '#' prefix cannot appear in a
            // user identifier, so these can never collide with a real name.
            Stmt::ForEach {
                var,
                collection,
                body,
            } => {
                let id = self.loop_id;
                self.loop_id += 1;
                let items = format!("#each_items_{}", id);
                let index = format!("#each_i_{}", id);

                self.compile_expr(collection);
                // Bound before the body runs, so a செயல் in the body can
                // capture the item it was made for.
                self.note_locals([var.clone()]);
                self.bytecode.push(Instruction::StoreVar(items.clone()));
                self.bytecode
                    .push(Instruction::Push(Value::Number(Decimal::ZERO)));
                self.bytecode.push(Instruction::StoreVar(index.clone()));

                let start = self.bytecode.len();
                self.bytecode.push(Instruction::LoadVar(index.clone()));
                self.bytecode.push(Instruction::LoadVar(items.clone()));
                self.bytecode.push(Instruction::Length);
                self.bytecode.push(Instruction::LessThan);
                let jump_false_idx = self.bytecode.len();
                self.bytecode.push(Instruction::JumpIfFalse(0)); // patched below

                self.bytecode.push(Instruction::LoadVar(items.clone()));
                self.bytecode.push(Instruction::LoadVar(index.clone()));
                self.bytecode.push(Instruction::NthOrKey);
                self.bytecode.push(Instruction::StoreVar(var));

                for stmt in body {
                    self.compile_stmt(stmt);
                }

                self.bytecode.push(Instruction::LoadVar(index.clone()));
                self.bytecode
                    .push(Instruction::Push(Value::Number(Decimal::ONE)));
                self.bytecode.push(Instruction::Add);
                self.bytecode.push(Instruction::StoreVar(index));
                self.bytecode.push(Instruction::Jump(start));

                let end = self.bytecode.len();
                self.bytecode.instructions[jump_false_idx] = Instruction::JumpIfFalse(end);
            }
            Stmt::FileOpen { filename, mode } => {
                self.compile_expr(filename);
                self.bytecode.push(Instruction::FileOpen(mode));
            }
            Stmt::FileClose { filename } => {
                self.compile_expr(filename);
                self.bytecode.push(Instruction::FileClose);
            }
            Stmt::FileWrite { filename, data } => {
                self.compile_expr(filename);
                self.compile_expr(data);
                self.bytecode.push(Instruction::FileWrite);
            }
            Stmt::FileRead { filename, variable } => {
                self.compile_expr(filename);
                self.bytecode.push(Instruction::FileRead);
                self.bytecode.push(Instruction::StoreVar(variable));
            }
            Stmt::ReadCSV { filename, variable } => {
                self.compile_expr(filename);
                self.bytecode.push(Instruction::ReadCSV);
                self.bytecode.push(Instruction::StoreVar(variable));
            }
            Stmt::WriteCSV { filename, data } => {
                self.compile_expr(filename);
                self.compile_expr(data);
                self.bytecode.push(Instruction::WriteCSV);
            }
            Stmt::SendResponse {
                status_code,
                body,
                headers,
            } => {
                self.compile_expr(status_code);
                self.compile_expr(body);
                // A response always leaves three values for the instruction,
                // so the stack shape does not depend on whether the author
                // supplied headers.
                match headers {
                    Some(expr) => self.compile_expr(expr),
                    None => self.bytecode.push(Instruction::Push(Value::Null)),
                }
                self.bytecode.push(Instruction::SendResponse);
            }
            Stmt::SendJSON { data, status_code } => {
                // The status is optional in the grammar; a JSON body with
                // nothing said about it is a 200.
                match status_code {
                    Some(expr) => self.compile_expr(expr),
                    None => self
                        .bytecode
                        .push(Instruction::Push(Value::Number(Decimal::from(200)))),
                }
                self.compile_expr(data);
                self.bytecode.push(Instruction::SendJSON);
            }
            Stmt::DBConnect {
                db_type,
                connection_string,
                handle,
            } => {
                self.compile_expr(connection_string);
                // Unnamed, and the driver name is the handle — which is what it
                // has always been, so an existing program is unchanged.
                let handle = handle.unwrap_or_else(|| db_type.clone());
                self.bytecode.push(Instruction::DBConnect(db_type, handle));
            }
            Stmt::DBDisconnect { db_type } => {
                self.bytecode.push(Instruction::DBDisconnect(db_type));
            }
            Stmt::DBExecute {
                command,
                params,
                handle,
            } => {
                self.compile_expr(command);
                self.compile_expr(params);
                self.bytecode.push(Instruction::DBExecute(handle));
            }
            Stmt::DBQuery {
                query,
                params,
                result_var,
                handle,
            } => {
                self.compile_expr(query);
                self.compile_expr(params);
                self.bytecode.push(Instruction::DBQuery(handle));
                self.bytecode.push(Instruction::StoreVar(result_var));
            }
            // The remaining database and server statements parse, but the VM
            // has no runtime for them yet. Emit an instruction that fails
            // loudly rather than silently doing nothing.
            other => {
                let label = Self::stmt_label(&other);
                self.bytecode.push(Instruction::Unsupported(label));
            }
        }
    }

    /// Human-readable name for a statement the VM cannot execute.
    fn stmt_label(stmt: &Stmt) -> String {
        match stmt {
            // Imports are resolved by module.rs before compilation; reaching
            // here means the program was compiled without resolving them.
            Stmt::Import(_) => "இறக்கு (import)",
            Stmt::DBConnect { .. } => "தளம்_இணை (database connect)",
            Stmt::DBDisconnect { .. } => "தளம்_பிரி (database disconnect)",
            Stmt::DBQuery { .. } => "தளம்_வினா (database query)",
            Stmt::DBExecute { .. } => "தளம்_செய் (database execute)",
            Stmt::DBInsert { .. } => "தளம்_செருக (database insert)",
            Stmt::DBUpdate { .. } => "தளம்_புதுப்பி (database update)",
            Stmt::DBDelete { .. } => "தளம்_நீக்கு (database delete)",
            Stmt::CreateTable { .. } => "அட்டை_ஆக்கு (create table)",
            Stmt::Select { .. } => "தேர்வெடு (select)",
            Stmt::DefineRoute { .. } => "வழி (route)",
            Stmt::Schedule { .. } => "இடைவெளி (schedule)",
            Stmt::StartServer { .. } => "வழங்கி_தொடங்கு (start server)",
            Stmt::StopServer => "வழங்கி_நிறுத்து (stop server)",
            Stmt::SendResponse { .. } => "பதில் (response)",
            Stmt::GetRequestBody { .. } => "உடல் (request body)",
            Stmt::GetRequestParam { .. } => "அளவுரு (request param)",
            Stmt::GetHeader { .. } | Stmt::SetHeader { .. } => "தலைப்பு (header)",
            _ => "this statement",
        }
        .to_string()
    }

    fn compile_expr(&mut self, expr: Expr) {
        match expr {
            Expr::Number(n) => {
                self.bytecode.push(Instruction::Push(Value::Number(n)));
            }
            Expr::String(s) => {
                self.bytecode.push(Instruction::Push(Value::String(s)));
            }
            Expr::Boolean(b) => {
                self.bytecode.push(Instruction::Push(Value::Boolean(b)));
            }
            Expr::Null => {
                self.bytecode.push(Instruction::Push(Value::Null));
            }
            Expr::Variable(name) => {
                self.bytecode.push(Instruction::LoadVar(name));
            }
            Expr::BinaryOp { op, left, right } => {
                self.compile_expr(*left);
                self.compile_expr(*right);

                match op.as_str() {
                    "+" => self.bytecode.push(Instruction::Add),
                    "-" => self.bytecode.push(Instruction::Subtract),
                    "*" => self.bytecode.push(Instruction::Multiply),
                    "/" => self.bytecode.push(Instruction::Divide),
                    "%" => self.bytecode.push(Instruction::Modulo),
                    _ => {}
                }
            }
            Expr::Comparison { left, op, right } => {
                self.compile_expr(*left);
                self.compile_expr(*right);

                match op.as_str() {
                    // The parser emits "==" for equality; matching "=" here
                    // meant no instruction was emitted and both operands were
                    // left on the stack, so every == comparison silently
                    // tested the truthiness of its right-hand side instead.
                    "==" => self.bytecode.push(Instruction::Equal),
                    "!=" => self.bytecode.push(Instruction::NotEqual),
                    "<" => self.bytecode.push(Instruction::LessThan),
                    "<=" => self.bytecode.push(Instruction::LessOrEqual),
                    ">" => self.bytecode.push(Instruction::GreaterThan),
                    ">=" => self.bytecode.push(Instruction::GreaterOrEqual),
                    other => panic!("Unknown comparison operator: {}", other),
                }
            }
            Expr::Logical { op, left, right } => {
                // Short-circuiting: the right side is not evaluated — and so
                // cannot fail — once the left has decided the answer.
                //
                // Both sides used to run always. That was survivable while
                // expressions could only compute, and stopped being so once
                // they could fail: a guard could not guard, because
                //
                //     (நீளம்(அ) > 0 மற்றும் அ[0] == 1)
                //
                // indexed an empty array on the very step that had just proved
                // it was empty. col.qmz's எழுத்து and AvaNam.qmz's
                // பகுதியை_எடு exist only to work around that.
                //
                // The answer is still a Boolean, as it was when both sides ran.
                // `Not` twice turns a value into its own truthiness, which is
                // how that is kept without a new instruction.
                self.compile_expr(*left);

                let (short_value, invert_left) = match op.as_str() {
                    // false and we are done; otherwise the right side decides.
                    "&&" => (crate::vm::Value::Boolean(false), false),
                    // true and we are done. JumpIfFalse is the only conditional
                    // jump there is, so the left side is inverted to use it.
                    "||" => (crate::vm::Value::Boolean(true), true),
                    other => panic!("Unknown logical operator: {}", other),
                };

                if invert_left {
                    self.bytecode.push(Instruction::Not);
                }

                let decided = self.bytecode.len();
                self.bytecode.push(Instruction::JumpIfFalse(0)); // patched below

                self.compile_expr(*right);
                self.bytecode.push(Instruction::Not);
                self.bytecode.push(Instruction::Not);

                let over = self.bytecode.len();
                self.bytecode.push(Instruction::Jump(0)); // patched below

                let short = self.bytecode.len();
                self.bytecode.push(Instruction::Push(short_value));

                let end = self.bytecode.len();
                self.bytecode.instructions[decided] = Instruction::JumpIfFalse(short);
                self.bytecode.instructions[over] = Instruction::Jump(end);
            }
            Expr::Not(inner) => {
                self.compile_expr(*inner);
                self.bytecode.push(Instruction::Not);
            }
            Expr::Call { name, args } => {
                let argc = args.len();
                for arg in args {
                    self.compile_expr(arg);
                }
                self.bytecode.push(Instruction::Call(name, argc));
            }
            Expr::ArrayLiteral(items) => {
                let count = items.len();
                for item in items {
                    self.compile_expr(item);
                }
                self.bytecode.push(Instruction::MakeArray(count));
            }
            Expr::RecordLiteral(fields) => {
                let mut keys = Vec::with_capacity(fields.len());
                for (key, value) in fields {
                    keys.push(key);
                    self.compile_expr(value);
                }
                self.bytecode.push(Instruction::MakeRecord(keys));
            }
            Expr::Index { base, index } => {
                self.compile_expr(*base);
                self.compile_expr(*index);
                self.bytecode.push(Instruction::Index);
            }
            Expr::Field { base, name, .. } => {
                self.compile_expr(*base);
                self.bytecode.push(Instruction::Field(name));
            }
            Expr::Try(inner) => {
                self.compile_expr(*inner);
                self.bytecode.push(Instruction::TryUnwrap);
            }
            Expr::Concat { left, right } => {
                self.compile_expr(*left);
                self.compile_expr(*right);
                self.bytecode.push(Instruction::Concat);
            }
            Expr::Lambda { params, body, .. } => {
                let captures = match &self.locals {
                    Some(locals) => {
                        crate::parser::captures(&params, &body, |name| locals.contains(name))
                    }
                    None => Vec::new(),
                };
                let name = format!("#செயல்_{}", NEXT_LAMBDA.fetch_add(1, Ordering::Relaxed));
                self.bytecode.lambdas.push(LambdaSource {
                    name: name.clone(),
                    captures: captures.clone(),
                    params: params.clone(),
                    body: body.clone(),
                });
                self.compile_function(name.clone(), captures.clone(), params, body);
                self.bytecode
                    .push(Instruction::MakeFunction(name, captures));
            }
            // `கடன்.புதிது(…)` names the shape: its function, called directly.
            // Anything else is a record whose method is found when it runs.
            Expr::MethodCall {
                receiver,
                name,
                args,
                ..
            } => {
                let argc = args.len();
                let associated = match receiver.as_ref() {
                    Expr::Variable(shape) if self.shapes.contains(shape) => Some(shape.clone()),
                    _ => None,
                };
                match associated {
                    Some(shape) => {
                        for arg in args {
                            self.compile_expr(arg);
                        }
                        let function = crate::vm::shape::method_function(&shape, &name);
                        self.bytecode.push(Instruction::Call(function, argc));
                    }
                    None => {
                        self.compile_expr(*receiver);
                        for arg in args {
                            self.compile_expr(arg);
                        }
                        self.bytecode.push(Instruction::CallMethod(name, argc));
                    }
                }
            }
            Expr::ShapeLiteral {
                shape,
                fields,
                base,
                ..
            } => {
                let with_base = base.is_some();
                if let Some(base) = base {
                    self.compile_expr(*base);
                }
                let mut keys = Vec::with_capacity(fields.len());
                for (key, value) in fields {
                    keys.push(key);
                    self.compile_expr(value);
                }
                self.bytecode.push(Instruction::MakeShaped {
                    shape,
                    keys,
                    with_base,
                });
            }
            Expr::CallValue { callee, args } => {
                let argc = args.len();
                self.compile_expr(*callee);
                for arg in args {
                    self.compile_expr(arg);
                }
                self.bytecode.push(Instruction::CallValue(argc));
            }
        }
    }
}
