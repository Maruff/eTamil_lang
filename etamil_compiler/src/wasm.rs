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
//! diagnostics and the symbol queries, and `vm`, for `run` and
//! `run_with_input`. What the VM cannot do in a browser it refuses explicitly
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
//! | `version` | the compiler version |
//!
//! `scripts/check_wasm_boundary.py` fails if an export is missing from that
//! table, because this comment went stale twice before anyone noticed.
//!
//! Every entry point returns a `String` rather than a `JsValue` -- JSON for
//! the six that carry structure, plain text for `version`. That keeps the
//! dependency list at `wasm-bindgen` alone -- no `serde-wasm-bindgen`, no
//! `js-sys` -- and the payloads are small enough that one `JSON.parse` on the
//! JavaScript side costs nothing measurable.

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::analysis;
use crate::check;
use crate::lexer;
use crate::parser::Parser;
use crate::vm;
use crate::vm::host;

/// Diagnostics for one source file, as a JSON array.
///
/// The passes are run in order and the first one to fail wins: a file that does
/// not lex cannot be parsed, and reporting invented parse errors on top of a
/// real lexical one buries the error the author needs to see.
#[wasm_bindgen]
pub fn diagnostics(source: &str) -> String {
    let out = analysis::collect_diagnostics(source);
    serde_json::to_string(&out).unwrap_or_else(|_| "[]".to_string())
}

/// Completion candidates for one source file, as a JSON array.
///
/// Falls back to the identifiers in the token stream when the file does not
/// parse -- which is most of the time, because the moment you want a completion
/// is the moment you have typed half a statement. A completion list that
/// vanishes on the first syntax error is a completion list nobody can use.
#[wasm_bindgen]
pub fn symbols(source: &str) -> String {
    let (found, _) = analysis::analyse(source);
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
    let visible = analysis::symbols_at(source, line, column);
    serde_json::to_string(&visible).unwrap_or_else(|_| "[]".to_string())
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
