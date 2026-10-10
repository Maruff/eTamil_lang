// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Browser bindings for the eTamil front end.
//!
//! The editor on etamil.in runs the real compiler front end rather than a
//! reimplementation of it, so a diagnostic in the browser is the same
//! diagnostic `etamil` prints on the command line -- including the bilingual
//! message text, which comes straight from each error type's `Display`.
//!
//! Two things are reachable from here: `lexer` -> `parser` -> `check`, for
//! diagnostics and the symbol queries, and `vm`, for `run`,
//! `run_with_input` and `run_project`. What the VM cannot do in a browser it refuses explicitly
//! rather than silently: the modules behind databases, sockets and `உள்ளிடு`
//! are gated out of a wasm build, so a program that reaches for one gets a
//! message saying it needs a machine of its own. See lib.rs.
//!
//! The whole of the surface a browser sees, and the only index of it:
//!
//! | export | what it answers |
//! |---|---|
//! | `diagnostics` | the compiler's own positioned, bilingual errors |
//! | `symbols` | every name a source defines |
//! | `symbols_at` | the names visible from one position |
//! | `script_spans` | which ASCII is eTamil script rather than English |
//! | `run` | the program, on the bytecode VM |
//! | `run_with_input` | the same, with stdin supplied |
//! | `run_project` | several sources that import each other, as JSON, with stdin supplied |
//! | `version` | the compiler version |
//!
//! `scripts/check_wasm_boundary.py` fails if an export is missing from that
//! table, because this comment went stale twice before anyone noticed.
//!
//! Every entry point returns a `String` rather than a `JsValue` -- JSON for
//! the seven that carry structure, plain text for `version`. That keeps the
//! dependency list at `wasm-bindgen` alone -- no `serde-wasm-bindgen`, no
//! `js-sys` -- and the payloads are small enough that one `JSON.parse` on the
//! JavaScript side costs nothing measurable.

use serde::Serialize;
use std::collections::HashSet;
use wasm_bindgen::prelude::*;

use crate::check;
use crate::lexer::{self, Spanned, Token};
use crate::parser::{Parser, Stmt};
use crate::vm;
use crate::vm::host;

/// One diagnostic, positioned the way the compiler positions errors: 1-based
/// line, 1-based column, both counting characters rather than bytes so Tamil
/// text reports sensible columns.
///
/// `length` is in characters too. The editor needs a range to underline, and
/// the length of the offending text is the closest thing the error types carry
/// to an end position.
#[derive(Serialize)]
struct Diagnostic {
    line: usize,
    column: usize,
    length: usize,
    /// Always "error" today. Present so warnings can be added without the
    /// JavaScript side having to change shape.
    severity: &'static str,
    /// Which pass rejected the input: "lex", "parse" or "type".
    stage: &'static str,
    /// The compiler's own bilingual message.
    message: String,
}

/// A name the editor can offer as a completion.
#[derive(Serialize)]
struct Symbol {
    name: String,
    /// "function", "parameter" or "variable".
    kind: &'static str,
    /// Shown beside the name: a parameter list for functions, the declared
    /// type for variables that have one.
    detail: String,
    /// Name of the function this was declared inside, or `None` for a top-level
    /// name. Used to decide visibility in `symbols_at` and never sent to the
    /// editor -- the editor asks "what can I see from here", not "who owns
    /// this".
    #[serde(skip)]
    owner: Option<String>,
}

/// One function body's extent, taken from the token stream.
///
/// The AST is the better source for what a name *is*, but it carries no span
/// for a `FunctionDef`, so it cannot say where a body begins and ends. The
/// token stream can: every token knows its line and column. Matching braces
/// over the tokens gives the ranges without touching parser.rs.
struct Scope {
    name: String,
    /// Position of the `{` that opens the body.
    start: (usize, usize),
    /// Position of the matching `}`, or the end of input when there is none
    /// yet -- which is the normal state while a function is being typed.
    end: (usize, usize),
}

/// Tuple ordering compares line first, then column, which is exactly the
/// document order these positions need.
fn within(position: (usize, usize), scope: &Scope) -> bool {
    position >= scope.start && position <= scope.end
}

/// Function body ranges, by matching braces from each `செயல்` token.
///
/// An unterminated body runs to the end of input rather than being discarded:
/// while you are still typing a function, the cursor is inside it, and that is
/// precisely when completions are wanted.
fn function_ranges(tokens: &[Spanned]) -> Vec<Scope> {
    let mut scopes = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if !matches!(tokens[index].token, Token::Function) {
            index += 1;
            continue;
        }

        // The name follows `செயல்`. Taking `.text` rather than matching on
        // Token::Identifier keeps a keyword used as a function name -- which
        // this language allows -- under the spelling the author wrote.
        let name = tokens
            .get(index + 1)
            .map(|t| t.text.clone())
            .unwrap_or_default();

        let Some(open) =
            (index + 1..tokens.len()).find(|&i| matches!(tokens[i].token, Token::LBrace))
        else {
            break;
        };

        let mut depth = 0usize;
        let mut close = None;
        for i in open..tokens.len() {
            match tokens[i].token {
                Token::LBrace => depth += 1,
                Token::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }

        scopes.push(Scope {
            name,
            start: (tokens[open].line, tokens[open].column),
            end: close
                .map(|i| (tokens[i].line, tokens[i].column))
                .unwrap_or((usize::MAX, usize::MAX)),
        });

        index = open + 1;
    }

    scopes
}

/// Character count, not byte count -- the compiler's columns are in characters.
fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Diagnostics for one source file, as a JSON array.
///
/// The passes are run in order and the first one to fail wins: a file that does
/// not lex cannot be parsed, and reporting invented parse errors on top of a
/// real lexical one buries the error the author needs to see.
#[wasm_bindgen]
pub fn diagnostics(source: &str) -> String {
    let out = collect_diagnostics(source);
    serde_json::to_string(&out).unwrap_or_else(|_| "[]".to_string())
}

fn collect_diagnostics(source: &str) -> Vec<Diagnostic> {
    let tokens = match lexer::tokenize(source) {
        Ok(tokens) => tokens,
        Err(errors) => {
            return errors
                .iter()
                .map(|e| Diagnostic {
                    line: e.line,
                    column: e.column,
                    length: char_len(&e.text).max(1),
                    severity: "error",
                    stage: "lex",
                    message: e.to_string(),
                })
                .collect();
        }
    };

    let statements = match Parser::new(tokens.iter()).parse() {
        Ok(statements) => statements,
        Err(e) => {
            return vec![Diagnostic {
                line: e.line,
                column: e.column,
                // `found` is empty at end of input, where there is nothing to
                // underline; one column keeps the marker visible.
                length: char_len(&e.found).max(1),
                severity: "error",
                stage: "parse",
                message: e.to_string(),
            }];
        }
    };

    match check::check(&statements) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .iter()
            .map(|e| Diagnostic {
                line: e.line,
                column: e.column,
                length: char_len(&e.name).max(1),
                severity: "error",
                stage: "type",
                message: e.to_string(),
            })
            .collect(),
    }
}

/// Completion candidates for one source file, as a JSON array.
///
/// Falls back to the identifiers in the token stream when the file does not
/// parse -- which is most of the time, because the moment you want a completion
/// is the moment you have typed half a statement. A completion list that
/// vanishes on the first syntax error is a completion list nobody can use.
#[wasm_bindgen]
pub fn symbols(source: &str) -> String {
    let (found, _) = analyse(source);
    serde_json::to_string(&found).unwrap_or_else(|_| "[]".to_string())
}

/// Completion candidates visible from one position, as a JSON array.
///
/// Same shape as `symbols`, minus the names that are out of scope: a parameter
/// or local of some other function is not something you can refer to from
/// here, and offering it is worse than offering nothing -- it invites code
/// that will not compile.
///
/// `line` and `column` are 1-based and count characters, matching the way the
/// compiler positions everything else.
#[wasm_bindgen]
pub fn symbols_at(source: &str, line: usize, column: usize) -> String {
    let (found, scopes) = analyse(source);
    let position = (line, column);

    // Every function body containing the cursor. More than one when a function
    // is nested, and none at top level.
    let enclosing: HashSet<&str> = scopes
        .iter()
        .filter(|scope| within(position, scope))
        .map(|scope| scope.name.as_str())
        .collect();

    let visible: Vec<&Symbol> = found
        .iter()
        .filter(|symbol| match &symbol.owner {
            None => true,
            Some(function) => enclosing.contains(function.as_str()),
        })
        .collect();

    serde_json::to_string(&visible).unwrap_or_else(|_| "[]".to_string())
}

fn analyse(source: &str) -> (Vec<Symbol>, Vec<Scope>) {
    let Ok(tokens) = lexer::tokenize(source) else {
        return (Vec::new(), Vec::new());
    };

    let scopes = function_ranges(&tokens);

    match Parser::new(tokens.iter()).parse() {
        Ok(statements) => {
            let mut found = Vec::new();
            let mut seen = HashSet::new();
            walk(&statements, None, &mut found, &mut seen);
            (found, scopes)
        }
        Err(_) => {
            let found = identifiers_from_tokens(&tokens, &scopes);
            (found, scopes)
        }
    }
}

/// Every distinct identifier in the token stream, in first-appearance order,
/// attributed to the function body it sits in.
///
/// This is the path taken whenever the file does not parse -- which, while
/// someone is typing, is most of the time. So it is worth scoping properly
/// rather than returning everything: the fallback is the common case, not the
/// exceptional one.
///
/// Keyword spellings never reach here: the lexer resolves those to their own
/// token variants, so `Token::Identifier` is exactly the set of author-chosen
/// names. The editor already offers keywords from its generated token table.
///
/// One imprecision: a name appearing both globally and inside a function is
/// recorded once, under whichever came first. Fixing that needs the parse this
/// path exists because we do not have.
fn identifiers_from_tokens(tokens: &[Spanned], scopes: &[Scope]) -> Vec<Symbol> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for spanned in tokens {
        let Token::Identifier(name) = &spanned.token else {
            continue;
        };
        if !seen.insert(name.clone()) {
            continue;
        }

        // Innermost enclosing body: the one that starts latest.
        let position = (spanned.line, spanned.column);
        let owner = scopes
            .iter()
            .filter(|scope| within(position, scope))
            .max_by_key(|scope| scope.start)
            .map(|scope| scope.name.clone());

        out.push(Symbol {
            name: name.clone(),
            kind: "variable",
            detail: String::new(),
            owner,
        });
    }

    out
}

/// Record one name, first occurrence winning.
///
/// A free function rather than a closure over `seen`: a closure capturing it
/// mutably cannot coexist with the recursive `walk` calls that also need it.
fn push(
    out: &mut Vec<Symbol>,
    seen: &mut HashSet<String>,
    owner: Option<&str>,
    name: &str,
    kind: &'static str,
    detail: String,
) {
    if name.is_empty() || !seen.insert(name.to_string()) {
        return;
    }
    out.push(Symbol {
        name: name.to_string(),
        kind,
        detail,
        owner: owner.map(str::to_string),
    });
}

/// Collect declared names, descending into every block that can hold them.
///
/// `owner` is the function whose body we are inside, or `None` at top level.
/// A function's *name* belongs to the scope that encloses it; its parameters
/// and everything declared in its body belong to the function. `symbols_at`
/// uses that to decide what a given cursor position can see.
///
/// `if`, `else` and loop bodies do not open a scope of their own here. eTamil
/// has no block-scoped binding form -- assignment is a bare `name = value` --
/// so a name first written inside an `if` is visible after it, and attributing
/// it to the enclosing function is the accurate answer rather than a shortcut.
fn walk(
    statements: &[Stmt],
    owner: Option<&str>,
    out: &mut Vec<Symbol>,
    seen: &mut HashSet<String>,
) {
    for statement in statements {
        match statement {
            Stmt::Assign { name, declared, .. } => {
                let detail = declared
                    .as_ref()
                    .map(|d| d.name().to_string())
                    .unwrap_or_default();
                push(out, seen, owner, name, "variable", detail);
            }
            Stmt::FunctionDef {
                name,
                params,
                returns,
                body,
                ..
            } => {
                let shown: Vec<String> = params
                    .iter()
                    .map(|param| match &param.declared {
                        Some(declared) => format!("{} {}", declared.name(), param.name),
                        None => param.name.clone(),
                    })
                    .collect();
                let signature = match returns {
                    Some(declared) => format!("({}) {}", shown.join(", "), declared.name()),
                    None => format!("({})", shown.join(", ")),
                };
                push(out, seen, owner, name, "function", signature);
                for param in params {
                    let detail = param
                        .declared
                        .as_ref()
                        .map(|d| d.name().to_string())
                        .unwrap_or_default();
                    push(out, seen, Some(name), &param.name, "parameter", detail);
                }
                walk(body, Some(name), out, seen);
            }
            Stmt::ForEach { var, body, .. } => {
                push(out, seen, owner, var, "variable", String::new());
                walk(body, owner, out, seen);
            }
            Stmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                walk(then_branch, owner, out, seen);
                if let Some(alternative) = else_branch {
                    walk(alternative, owner, out, seen);
                }
            }
            Stmt::Loop { body, .. } => walk(body, owner, out, seen),
            Stmt::SetIndex { name, .. } | Stmt::SetField { name, .. } => {
                push(out, seen, owner, name, "variable", String::new());
            }
            Stmt::FileRead { variable, .. } | Stmt::ReadCSV { variable, .. } => {
                push(out, seen, owner, variable, "variable", String::new());
            }
            Stmt::DBQuery { result_var, .. } => {
                push(out, seen, owner, result_var, "variable", String::new());
            }
            // Statements that declare nothing. Listed as a catch-all rather
            // than exhaustively so a new Stmt variant does not break the wasm
            // build -- it only means that variant declares no completions yet.
            _ => {}
        }
    }
}

/// What one run produced.
#[derive(Serialize)]
struct RunResult {
    /// False when any stage rejected the program or the VM raised an error.
    ok: bool,
    /// Everything `அச்சு` printed, kept even when the run then failed -- a
    /// program that prints three lines and dies on the fourth statement should
    /// still show its three lines.
    output: String,
    /// The failure, bilingual, from whichever stage produced it.
    error: Option<String>,
    /// Which stage failed: "lex", "parse", "type" or "run".
    stage: Option<&'static str>,
    /// Files the program wrote, so the editor can show that `கோப்பு_எழுது`
    /// did something even though the file only lived in memory.
    files: Vec<String>,
}

/// Instructions one run may retire before it is called an endless loop.
///
/// Ten million is far more than any example needs and still returns in well
/// under a second, so the ceiling is invisible unless something is wrong.
const STEP_LIMIT: u64 = 10_000_000;

/// Compile and run one source file, returning JSON.
///
/// The whole pipeline, in the browser, with no server: lex, parse, type-check,
/// compile to bytecode, interpret. Output and files go through `vm::host`,
/// whose browser implementation collects them in memory instead of touching a
/// console or a disk.
#[wasm_bindgen]
pub fn run(source: &str) -> String {
    let result = execute(source, "");
    serde_json::to_string(&result).unwrap_or_else(|_| {
        r#"{"ok":false,"output":"","error":"result could not be encoded","stage":"run","files":[]}"#
            .to_string()
    })
}

/// Compile and run one source file, with input for `உள்ளிடு`.
///
/// `input` is everything a person would have typed, newline-separated; the
/// program reads one line per `உள்ளிடு`. It arrives with the program rather
/// than during the run because a page has nowhere to type while the run is
/// happening — the VM would have to block, and a blocked page is a hung tab.
/// Asking for more lines than were supplied is the program's own error, the
/// same as reading past the end of a file.
#[wasm_bindgen]
pub fn run_with_input(source: &str, input: &str) -> String {
    let result = execute(source, input);
    serde_json::to_string(&result).unwrap_or_else(|_| {
        r#"{"ok":false,"output":"","error":"result could not be encoded","stage":"run","files":[]}"#
            .to_string()
    })
}

/// Compile and run a project: several sources that import each other, as JSON.
///
/// `files_json` is an object from path to source, for example
/// `{"main.qmz": "...", "vari.qmz": "..."}`; `entry` names the one to run. An
/// `இறக்கு "vari.qmz"` in a source finds `vari.qmz` among the others, relative to the
/// importing file, as on disk. The browser has no files to read, so everything an
/// import may reach has to be in the object: a page that wants the standard library
/// to work (`இறக்கு "nUlakam/paNam/paNam.qmz"`) puts its sources in too, under those
/// paths. `input` is as for `run_with_input`.
#[wasm_bindgen]
pub fn run_project(files_json: &str, entry: &str, input: &str) -> String {
    let result = execute_project(files_json, entry, input);
    serde_json::to_string(&result).unwrap_or_else(|_| {
        r#"{"ok":false,"output":"","error":"result could not be encoded","stage":"run","files":[]}"#
            .to_string()
    })
}

fn execute_project(files_json: &str, entry: &str, input: &str) -> RunResult {
    host::reset();
    for line in input.lines() {
        host::push_input(line);
    }

    let failed = |stage: &'static str, error: String| RunResult {
        ok: false,
        output: host::take_output(),
        error: Some(error),
        stage: Some(stage),
        files: host::file_names(),
    };

    let files: std::collections::HashMap<String, String> = match serde_json::from_str(files_json) {
        Ok(files) => files,
        Err(e) => {
            return failed(
                "parse",
                format!(
                    "the project's files could not be read (an object from path to source was expected): {e}"
                ),
            );
        }
    };

    // Lexing, parsing and resolving imports are one stage to a caller: either the
    // program could be assembled, or it could not.
    match crate::project::load(&files, entry) {
        Ok(statements) => run_statements(statements, &failed),
        Err(e) => failed("parse", e),
    }
}

fn execute(source: &str, input: &str) -> RunResult {
    // Nothing carries over between runs: last run's output and files are gone
    // before this one starts.
    host::reset();

    // After the reset, which clears the queue. `lines()` drops the trailing
    // newline, so one trailing newline does not add an empty line to read.
    for line in input.lines() {
        host::push_input(line);
    }

    let failed = |stage: &'static str, error: String| RunResult {
        ok: false,
        output: host::take_output(),
        error: Some(error),
        stage: Some(stage),
        files: host::file_names(),
    };

    let tokens = match lexer::tokenize(source) {
        Ok(tokens) => tokens,
        // Only the first is reported: a run either happens or does not, and the
        // editor is already underlining every one of them.
        Err(errors) => {
            let first = errors
                .first()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "lexical error".to_string());
            return failed("lex", first);
        }
    };

    let statements = match Parser::new(tokens.iter()).parse() {
        Ok(statements) => statements,
        Err(e) => return failed("parse", e.to_string()),
    };

    run_statements(statements, &failed)
}

/// Check, compile and run a program that has been parsed (and, for a project, had its
/// imports resolved). Shared by a single source and by a project.
fn run_statements(
    statements: Vec<Stmt>,
    failed: &dyn Fn(&'static str, String) -> RunResult,
) -> RunResult {
    // A program that does not type-check is not run, which is what the command
    // line does too -- running it anyway would produce a second, more confusing
    // error somewhere further along.
    if let Err(errors) = check::check(&statements) {
        let first = errors
            .first()
            .map(|e| e.to_string())
            .unwrap_or_else(|| "type error".to_string());
        return failed("type", first);
    }

    let bytecode = vm::compile_to_bytecode(statements);
    let mut machine = vm::VM::new();

    match machine.execute_limited(bytecode, STEP_LIMIT) {
        Ok(()) => RunResult {
            ok: true,
            output: host::take_output(),
            error: None,
            stage: None,
            files: host::file_names(),
        },
        Err(e) => failed("run", e),
    }
}

/// The compiler version this wasm module was built from, so the editor can say
/// which compiler produced a diagnostic.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ---------------------------------------------------------------------------
// Script spans
// ---------------------------------------------------------------------------

/// One run of eTamil-script ASCII on one line.
///
/// Offsets are UTF-16 code units from the start of the line, which is what
/// both editors count in: VS Code positions and CodeMirror document offsets
/// are both UTF-16. For Tamil it makes no difference -- the block is entirely
/// BMP -- but a stray emoji in a comment would shift every span after it if
/// this counted characters instead.
#[derive(Serialize)]
struct ScriptSpan {
    line: usize,
    start: usize,
    end: usize,
}

/// Every span of ASCII that should be drawn in the eTamil font.
///
/// This exists because the rule was being implemented twice -- once in
/// `eTamil_Code/src/marks.ts` for VS Code and once in
/// `eTamil_site/ide/src/etamil-font.js` for the browser -- with nothing making
/// the two agree. `docs/reference/SCRIPT_RULES.md` is normative and the
/// compiler is where it should be read from, beside the diagnostics and the
/// symbols that already come from here.
///
/// **What is returned is the eTamil-script ASCII, not the English.** An editor
/// keeps its ordinary ISO font as the base and paints only these spans. The
/// direction is deliberate: a span this misses renders eTamil as plain Latin,
/// which is the ordinary view of the file, where painting the other way round
/// would render English in Tamil glyphs, which is unreadable.
///
/// Left out, and so drawn in the ISO font:
///
///   - anything inside a string literal, which is data and carries no marks
///   - an identifier beginning with `_`, in whole (Rule 1)
///   - a name immediately preceded by `.` -- a field name, an extension
///   - comment text between `__` and `__` (Rule 2), across as many comment
///     lines as the sentence takes
///   - the licence header
///   - Unicode Tamil, which both fonts draw as Tamil, so the choice is moot
#[wasm_bindgen]
pub fn script_spans(source: &str) -> String {
    let mut spans: Vec<ScriptSpan> = Vec::new();
    // Both states outlive a line. A string may span lines because the lexer's
    // pattern does not exclude a newline, and an English comment may span them
    // because Rule 2's marks go at the ends of the sentence.
    let mut in_string = false;
    let mut in_english = false;

    for (number, raw) in source.split('\n').enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let chars: Vec<char> = line.chars().collect();

        // char index -> UTF-16 offset, one longer than `chars`.
        let mut utf16 = Vec::with_capacity(chars.len() + 1);
        let mut running = 0usize;
        utf16.push(0usize);
        for character in &chars {
            running += character.len_utf16();
            utf16.push(running);
        }

        // A block is contiguous `//` lines, so a line that is not a comment
        // ends any open region: one unclosed `__` cannot make the rest of the
        // file English.
        if !in_string && !starts_a_comment(&chars) {
            in_english = false;
        }

        let mut found: Vec<(usize, usize)> = Vec::new();
        let mut index = 0usize;

        while index < chars.len() {
            if in_string {
                if chars[index] == '\\' {
                    index += 2;
                    continue;
                }
                if chars[index] == '"' {
                    in_string = false;
                }
                index += 1;
                continue;
            }

            let character = chars[index];

            if character == '"' {
                in_string = true;
                index += 1;
                continue;
            }

            if character == '/' && chars.get(index + 1) == Some(&'/') {
                in_english = comment_spans(&mut found, &chars, index + 2, in_english);
                break;
            }

            if is_ident_start(character) {
                let mut end = index;
                while end < chars.len() && is_ident_part(chars[end]) {
                    end += 1;
                }
                // Rule 1 puts the mark on the identifier, so the whole of a
                // marked name stays ISO -- including a mixed one, where only
                // part of it is Latin. A name reached through `.` stays ISO
                // for the reason a string does: it is data.
                let after_dot = index > 0 && chars[index - 1] == '.';
                if character != '_' && !after_dot {
                    latin_runs(&mut found, &chars[index..end], index);
                }
                index = end;
                continue;
            }

            index += 1;
        }

        for (start, end) in found {
            spans.push(ScriptSpan {
                line: number,
                start: utf16[start],
                end: utf16[end],
            });
        }
    }

    serde_json::to_string(&spans).unwrap_or_else(|_| "[]".to_string())
}

/// Whether a line is a comment, ignoring leading blanks.
fn starts_a_comment(chars: &[char]) -> bool {
    let mut index = 0;
    while index < chars.len() && (chars[index] == ' ' || chars[index] == '\t') {
        index += 1;
    }
    chars.get(index) == Some(&'/') && chars.get(index + 1) == Some(&'/')
}

fn is_ident_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_' || is_tamil(character)
}

fn is_ident_part(character: char) -> bool {
    is_ident_start(character) || character.is_ascii_digit()
}

fn is_tamil(character: char) -> bool {
    ('\u{0B80}'..='\u{0BFF}').contains(&character)
}

/// The Latin letters inside one unmarked identifier. Digits are not remapped
/// by the font and are left alone.
fn latin_runs(found: &mut Vec<(usize, usize)>, token: &[char], offset: usize) {
    let mut index = 0;
    while index < token.len() {
        if token[index].is_ascii_alphabetic() {
            let start = index;
            while index < token.len() && token[index].is_ascii_alphabetic() {
                index += 1;
            }
            found.push((offset + start, offset + index));
            continue;
        }
        index += 1;
    }
}

/// The eTamil-script part of one comment, and whether English is still open.
///
/// `in_english` comes in as the state the previous comment line left and goes
/// out as the state this one leaves, so `__` opens a region that survives to
/// the line carrying the closing mark.
fn comment_spans(
    found: &mut Vec<(usize, usize)>,
    chars: &[char],
    from: usize,
    in_english: bool,
) -> bool {
    let body = &chars[from.min(chars.len())..];

    // Exempt, and deliberately state-neutral: the header sits above everything
    // and must not open or close a region for the code below it. A licence
    // scanner reads the SPDX expression to the end of the line, so a closing
    // `__` would become part of the licence name.
    if is_licence_header(body) {
        return in_english;
    }

    let mut english: Vec<(usize, usize)> = Vec::new();
    let mut inside = in_english;
    let mut opened_at = 0usize;
    let mut index = 0usize;

    while index < body.len() {
        if body[index] == '_' && body.get(index + 1) == Some(&'_') {
            if inside {
                english.push((opened_at, index + 2));
                inside = false;
            } else {
                opened_at = index;
                inside = true;
            }
            index += 2;
            continue;
        }
        index += 1;
    }
    if inside {
        english.push((opened_at, body.len()));
    }

    let mut index = 0usize;
    while index < body.len() {
        if body[index].is_ascii_alphabetic() {
            let start = index;
            while index < body.len() && body[index].is_ascii_alphabetic() {
                index += 1;
            }
            let is_english = english
                .iter()
                .any(|(open, close)| start >= *open && index <= *close);
            let after_dot = start > 0 && body[start - 1] == '.';
            if !is_english && !after_dot {
                found.push((from + start, from + index));
            }
            continue;
        }
        index += 1;
    }

    inside
}

/// `SPDX-…:` or `Copyright (C)`, the two lines Rule 2 exempts.
fn is_licence_header(body: &[char]) -> bool {
    let text: String = body.iter().collect();
    let text = text.trim();
    if let Some(rest) = text.strip_prefix("SPDX-") {
        let tag = rest.split(':').next().unwrap_or("");
        return rest.contains(':')
            && !tag.is_empty()
            && tag.chars().all(|c| c.is_ascii_alphabetic() || c == '-');
    }
    match text.strip_prefix("Copyright") {
        Some(rest) => rest.trim_start().starts_with("(C)"),
        None => false,
    }
}
