// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! The language's own vocabulary: every keyword with each spelling, the host
//! builtins, and the standard library's functions.
//!
//! The data is `data/language-data.json`, written by
//! `scripts/generate_editor_support.py` from the compiler's lexer, interpreter and
//! `nUlakam/`, so it cannot drift from the language (`--check` fails when it does).
//! It is compiled into the binary and parsed once, on first use.

use std::sync::OnceLock;

use serde::Deserialize;

/// One keyword, with every spelling the lexer accepts.
#[derive(Debug, Deserialize)]
pub struct Keyword {
    /// The compiler's internal token name, such as `If`.
    pub token: String,
    /// Tamil script first, then romanized, then any English alias.
    pub forms: Vec<String>,
    /// The lexer section it belongs to, such as `Control Flow`.
    pub group: String,
    /// Reserved, but no statement consumes it: using it is always an error.
    #[serde(rename = "noSyntax")]
    pub no_syntax: bool,
    /// A statement template for Tamil-script authors; `{kw}` stands for the spelling.
    #[serde(rename = "snippetTamil")]
    pub snippet_tamil: Option<String>,
    /// The same template with romanized placeholders.
    #[serde(rename = "snippetLatin")]
    pub snippet_latin: Option<String>,
}

/// A host builtin, or a function defined in `nUlakam/`.
#[derive(Debug, Deserialize)]
pub struct Function {
    pub name: String,
    pub forms: Vec<String>,
    /// Parameter names, for standard library functions whose source can be read.
    pub params: Option<Vec<String>>,
    pub arity: Option<u32>,
    pub doc: String,
    /// `builtin` or `stdlib`.
    pub kind: String,
    /// Repository-relative path of the module, for standard library functions.
    pub module: Option<String>,
}

#[derive(Deserialize)]
struct Catalog {
    keywords: Vec<Keyword>,
    functions: Vec<Function>,
}

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../data/language-data.json"))
            .expect("data/language-data.json is generated, and parses")
    })
}

pub fn keywords() -> &'static [Keyword] {
    &catalog().keywords
}

pub fn functions() -> &'static [Function] {
    &catalog().functions
}

/// A character of an identifier: ASCII letters, digits and `_`, and the Tamil block.
pub fn is_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || ('\u{0B80}'..='\u{0BFF}').contains(&c)
}

/// Whether a spelling is written in Tamil script rather than Latin letters.
pub fn is_tamil(text: &str) -> bool {
    text.chars().any(|c| ('\u{0B80}'..='\u{0BFF}').contains(&c))
}

/// Whether `label` starts with `prefix`, ignoring case (which Tamil has none of).
pub fn matches_prefix(label: &str, prefix: &str) -> bool {
    let mut label = label.chars().flat_map(char::to_lowercase);
    prefix
        .chars()
        .flat_map(char::to_lowercase)
        .all(|wanted| label.next() == Some(wanted))
}

/// The keyword that has `text` as one of its spellings.
pub fn keyword_for(text: &str) -> Option<&'static Keyword> {
    keywords().iter().find(|k| k.forms.iter().any(|form| form == text))
}

/// The builtin or standard library function that has `text` as one of its spellings.
pub fn function_for(text: &str) -> Option<&'static Function> {
    functions().iter().find(|f| f.forms.iter().any(|form| form == text))
}

/// The spelling of the import keyword to use in a generated import, in Tamil script.
pub fn import_keyword() -> &'static str {
    keywords()
        .iter()
        .find(|k| k.token == "Import")
        .and_then(|k| k.forms.first())
        .map(String::as_str)
        .unwrap_or("இறக்கு")
}

/// The statement a keyword completion inserts, if it has a template. The template
/// whose script matches the spelling is chosen, so a Tamil spelling gets Tamil
/// placeholders and a romanized one gets Latin ones.
pub fn keyword_snippet(keyword: &Keyword, form: &str) -> Option<String> {
    let template = if is_tamil(form) {
        keyword.snippet_tamil.as_ref().or(keyword.snippet_latin.as_ref())
    } else {
        keyword.snippet_latin.as_ref().or(keyword.snippet_tamil.as_ref())
    }?;
    Some(template.replace("{kw}", form))
}

/// `name(a, b)`, or `name(2 arguments)` when only the count is known.
pub fn signature(function: &Function, form: &str) -> String {
    match (&function.params, function.arity) {
        (Some(params), _) => format!("{form}({})", params.join(", ")),
        (None, Some(0)) => format!("{form}()"),
        (None, Some(1)) => format!("{form}(1 argument)"),
        (None, Some(count)) => format!("{form}({count} arguments)"),
        (None, None) => form.to_string(),
    }
}

/// A snippet that inserts the call with its parameters as tab stops.
pub fn call_snippet(function: &Function, form: &str) -> String {
    match (&function.params, function.arity) {
        (Some(params), _) if !params.is_empty() => {
            let stops: Vec<String> = params
                .iter()
                .enumerate()
                .map(|(i, name)| format!("${{{}:{}}}", i + 1, name))
                .collect();
            format!("{form}({})$0", stops.join(", "))
        }
        (_, Some(0)) | (Some(_), _) => format!("{form}()$0"),
        _ => format!("{form}($1)$0"),
    }
}

/// Markdown for a function's documentation: its doc line, and where it comes from.
pub fn function_markdown(function: &Function, form: &str) -> String {
    let origin = match &function.module {
        Some(module) => format!("standard library function, from `{module}`"),
        None => "builtin function".to_string(),
    };
    let mut text = format!("**{}**\n\n`{}`", origin, signature(function, form));
    if !function.doc.is_empty() {
        text.push_str("\n\n");
        text.push_str(&function.doc);
    }
    text
}

/// Markdown for a keyword: what it is, and every way it can be written.
pub fn keyword_markdown(keyword: &Keyword, form: &str) -> String {
    format!(
        "**keyword** `{form}`\n\n{} ({}). Also written: {}.",
        keyword.group,
        keyword.token,
        keyword
            .forms
            .iter()
            .filter(|spelling| spelling.as_str() != form)
            .map(|spelling| format!("`{spelling}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
