// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Source analysis for editors: diagnostics and the names visible from a
//! position.
//!
//! This was written inside `wasm.rs` for the editor on etamil.in and moved here,
//! unchanged, so the language server (`etamil_lsp`) can answer the same
//! questions with the same code. It touches no operating system, so it is
//! compiled on every target, wasm included. `wasm.rs` keeps the JSON wrappers
//! that a browser sees; positions here are 1-based and count characters.

use serde::Serialize;
use std::collections::HashSet;

use crate::check;
use crate::lexer::{self, Spanned, Token};
use crate::parser::{Parser, Stmt};

/// One diagnostic, positioned the way the compiler positions errors: 1-based
/// line, 1-based column, both counting characters rather than bytes so Tamil
/// text reports sensible columns.
///
/// `length` is in characters too. The editor needs a range to underline, and
/// the length of the offending text is the closest thing the error types carry
/// to an end position.
#[derive(Serialize)]
pub struct Diagnostic {
    pub line: usize,
    pub column: usize,
    pub length: usize,
    /// Always "error" today. Present so warnings can be added without the
    /// JavaScript side having to change shape.
    pub severity: &'static str,
    /// Which pass rejected the input: "lex", "parse" or "type".
    pub stage: &'static str,
    /// The compiler's own bilingual message.
    pub message: String,
}

/// A name the editor can offer as a completion.
#[derive(Serialize)]
pub struct Symbol {
    pub name: String,
    /// "function", "parameter" or "variable".
    pub kind: &'static str,
    /// Shown beside the name: a parameter list for functions, the declared
    /// type for variables that have one.
    pub detail: String,
    /// Name of the function this was declared inside, or `None` for a top-level
    /// name. Used to decide visibility in `symbols_at` and never sent to the
    /// editor -- the editor asks "what can I see from here", not "who owns
    /// this".
    #[serde(skip)]
    pub owner: Option<String>,
}

/// One function body's extent, taken from the token stream.
///
/// The AST is the better source for what a name *is*, but it carries no span
/// for a `FunctionDef`, so it cannot say where a body begins and ends. The
/// token stream can: every token knows its line and column. Matching braces
/// over the tokens gives the ranges without touching parser.rs.
pub struct Scope {
    pub name: String,
    /// Position of the `{` that opens the body.
    pub start: (usize, usize),
    /// Position of the matching `}`, or the end of input when there is none
    /// yet -- which is the normal state while a function is being typed.
    pub end: (usize, usize),
}

/// Tuple ordering compares line first, then column, which is exactly the
/// document order these positions need.
pub fn within(position: (usize, usize), scope: &Scope) -> bool {
    position >= scope.start && position <= scope.end
}

/// Function body ranges, by matching braces from each `செயல்` token.
///
/// An unterminated body runs to the end of input rather than being discarded:
/// while you are still typing a function, the cursor is inside it, and that is
/// precisely when completions are wanted.
pub fn function_ranges(tokens: &[Spanned]) -> Vec<Scope> {
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
        for (i, token) in tokens.iter().enumerate().skip(open) {
            match token.token {
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
pub fn char_len(s: &str) -> usize {
    s.chars().count()
}

pub fn collect_diagnostics(source: &str) -> Vec<Diagnostic> {
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

pub fn analyse(source: &str) -> (Vec<Symbol>, Vec<Scope>) {
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

/// Names visible from one position: every top-level name, and the parameters
/// and locals of each function whose body contains the position.
///
/// `line` and `column` are 1-based and count characters.
pub fn symbols_at(source: &str, line: usize, column: usize) -> Vec<Symbol> {
    let (found, scopes) = analyse(source);
    let position = (line, column);

    // Every function body containing the cursor. More than one when a function
    // is nested, and none at top level.
    let enclosing: HashSet<&str> = scopes
        .iter()
        .filter(|scope| within(position, scope))
        .map(|scope| scope.name.as_str())
        .collect();

    found
        .into_iter()
        .filter(|symbol| match &symbol.owner {
            None => true,
            Some(function) => enclosing.contains(function.as_str()),
        })
        .collect()
}
