// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! The eTamil language server.
//!
//! Every answer comes from the compiler's own front end, through
//! `etamil_compiler::analysis` (the same code the editor on etamil.in runs), so
//! a diagnostic here is the diagnostic `etamil --check` prints, in both
//! languages. Supported: diagnostics, completion of the names visible from the
//! cursor, hover, and go to definition.
//!
//! The compiler positions things 1-based and counts characters; LSP is 0-based
//! and counts UTF-16 code units. The conversions are at the edge, in
//! [`char_to_utf16`] and [`utf16_to_char`].

use std::collections::HashMap;
use std::error::Error;

use etamil_compiler::analysis;
use etamil_compiler::lexer::{self, Spanned, Token};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::request::{Completion, GotoDefinition, HoverRequest, Request as _};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse,
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents,
    HoverParams, HoverProviderCapability, Location, MarkupContent, MarkupKind, NumberOrString,
    OneOf, Position, PublishDiagnosticsParams, Range, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// What this server offers. Full-text sync: eTamil files are small, and
/// re-analysing the whole file on every change is what the compiler does anyway.
pub fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions::default()),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        ..Default::default()
    }
}

// --- Positions ---------------------------------------------------------------

fn line_text(source: &str, line: usize) -> &str {
    source.lines().nth(line).unwrap_or("")
}

/// UTF-16 offset of the character at 0-based `column` in `text`.
pub fn char_to_utf16(text: &str, column: usize) -> u32 {
    text.chars()
        .take(column)
        .map(|c| c.len_utf16() as u32)
        .sum()
}

/// 0-based character column for a UTF-16 offset in `text`. An offset inside a
/// surrogate pair rounds up to the next character.
pub fn utf16_to_char(text: &str, utf16: u32) -> usize {
    let mut units = 0u32;
    let mut characters = 0usize;
    for c in text.chars() {
        if units >= utf16 {
            break;
        }
        units += c.len_utf16() as u32;
        characters += 1;
    }
    characters
}

/// An LSP position as the compiler's 1-based (line, character column).
fn to_compiler(source: &str, position: Position) -> (usize, usize) {
    let text = line_text(source, position.line as usize);
    (
        position.line as usize + 1,
        utf16_to_char(text, position.character) + 1,
    )
}

/// The LSP range of `length` characters starting at a compiler position.
fn to_range(source: &str, line: usize, column: usize, length: usize) -> Range {
    let row = line.saturating_sub(1);
    let text = line_text(source, row);
    let start = column.saturating_sub(1);
    Range::new(
        Position::new(row as u32, char_to_utf16(text, start)),
        Position::new(row as u32, char_to_utf16(text, start + length)),
    )
}

// --- Features ----------------------------------------------------------------

/// The compiler's diagnostics for one file, in LSP form. The message is the
/// compiler's own bilingual text; `code` says which pass rejected the file.
pub fn diagnostics(source: &str) -> Vec<Diagnostic> {
    analysis::collect_diagnostics(source)
        .into_iter()
        .map(|d| Diagnostic {
            range: to_range(source, d.line, d.column, d.length),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String(d.stage.to_string())),
            source: Some("etamil".to_string()),
            message: d.message,
            ..Default::default()
        })
        .collect()
}

/// The names visible from the cursor. Keywords are not offered here: editors
/// already get them from the grammar's keyword table.
pub fn completions(source: &str, position: Position) -> Vec<CompletionItem> {
    let (line, column) = to_compiler(source, position);
    analysis::symbols_at(source, line, column)
        .into_iter()
        .map(|symbol| CompletionItem {
            kind: Some(match symbol.kind {
                "function" => CompletionItemKind::FUNCTION,
                _ => CompletionItemKind::VARIABLE,
            }),
            detail: (!symbol.detail.is_empty()).then_some(symbol.detail),
            label: symbol.name,
            ..Default::default()
        })
        .collect()
}

/// The identifier token under the cursor, with all the tokens, if the file lexes.
fn identifier_at(source: &str, position: Position) -> Option<(Spanned, Vec<Spanned>)> {
    let (line, column) = to_compiler(source, position);
    let tokens = lexer::tokenize(source).ok()?;
    let found = tokens
        .iter()
        .find(|t| {
            matches!(t.token, Token::Identifier(_))
                && t.line == line
                && t.column <= column
                && column <= t.column + t.text.chars().count()
        })?
        .clone();
    Some((found, tokens))
}

pub fn hover(source: &str, position: Position) -> Option<Hover> {
    let (token, _) = identifier_at(source, position)?;
    let (line, column) = to_compiler(source, position);
    let symbol = analysis::symbols_at(source, line, column)
        .into_iter()
        .find(|s| s.name == token.text)?;
    let shown = match (symbol.kind, symbol.detail.is_empty()) {
        ("function", _) => format!("{}{}", symbol.name, symbol.detail),
        (_, true) => symbol.name.clone(),
        (_, false) => format!("{}: {}", symbol.name, symbol.detail),
    };
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("**{}** `{}`", symbol.kind, shown),
        }),
        range: Some(to_range(
            source,
            token.line,
            token.column,
            token.text.chars().count(),
        )),
    })
}

/// Go to the first place the name is written. eTamil has no declaration
/// keyword for variables, so the first occurrence is where it is introduced;
/// for a function it is the function line, unless it is called before it is
/// defined, in which case this lands on the call.
pub fn definition(source: &str, uri: &Url, position: Position) -> Option<Location> {
    let (token, tokens) = identifier_at(source, position)?;
    let first = tokens
        .iter()
        .find(|t| matches!(t.token, Token::Identifier(_)) && t.text == token.text)?;
    Some(Location::new(
        uri.clone(),
        to_range(source, first.line, first.column, first.text.chars().count()),
    ))
}

// --- Server loop -------------------------------------------------------------

fn publish(connection: &Connection, uri: Url, source: &str) -> Result<()> {
    let params = PublishDiagnosticsParams::new(uri, diagnostics(source), None);
    connection
        .sender
        .send(Message::Notification(Notification::new(
            PublishDiagnostics::METHOD.to_string(),
            params,
        )))?;
    Ok(())
}

fn handle_notification(
    connection: &Connection,
    documents: &mut HashMap<Url, String>,
    notification: Notification,
) -> Result<()> {
    match notification.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(notification.params)?;
            let uri = params.text_document.uri;
            documents.insert(uri.clone(), params.text_document.text.clone());
            publish(connection, uri, &params.text_document.text)?;
        }
        DidChangeTextDocument::METHOD => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(notification.params)?;
            // Full sync: the last change carries the whole text.
            if let Some(change) = params.content_changes.into_iter().last() {
                let uri = params.text_document.uri;
                documents.insert(uri.clone(), change.text.clone());
                publish(connection, uri, &change.text)?;
            }
        }
        DidCloseTextDocument::METHOD => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(notification.params)?;
            documents.remove(&params.text_document.uri);
            // Clear what the editor is still showing for the closed file.
            let clear = PublishDiagnosticsParams::new(params.text_document.uri, Vec::new(), None);
            connection
                .sender
                .send(Message::Notification(Notification::new(
                    PublishDiagnostics::METHOD.to_string(),
                    clear,
                )))?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_request(documents: &HashMap<Url, String>, request: Request) -> Response {
    let id = request.id.clone();
    let invalid = |id, error: serde_json::Error| {
        Response::new_err(id, ErrorCode::InvalidParams as i32, error.to_string())
    };
    match request.method.as_str() {
        Completion::METHOD => match serde_json::from_value::<CompletionParams>(request.params) {
            Ok(p) => {
                let at = p.text_document_position;
                let result = documents
                    .get(&at.text_document.uri)
                    .map(|source| CompletionResponse::Array(completions(source, at.position)));
                Response::new_ok(id, result)
            }
            Err(error) => invalid(id, error),
        },
        HoverRequest::METHOD => match serde_json::from_value::<HoverParams>(request.params) {
            Ok(p) => {
                let at = p.text_document_position_params;
                let result = documents
                    .get(&at.text_document.uri)
                    .and_then(|source| hover(source, at.position));
                Response::new_ok(id, result)
            }
            Err(error) => invalid(id, error),
        },
        GotoDefinition::METHOD => {
            match serde_json::from_value::<GotoDefinitionParams>(request.params) {
                Ok(p) => {
                    let at = p.text_document_position_params;
                    let result = documents.get(&at.text_document.uri).and_then(|source| {
                        definition(source, &at.text_document.uri, at.position)
                            .map(GotoDefinitionResponse::Scalar)
                    });
                    Response::new_ok(id, result)
                }
                Err(error) => invalid(id, error),
            }
        }
        other => Response::new_err(
            id,
            ErrorCode::MethodNotFound as i32,
            format!("method not supported: {other}"),
        ),
    }
}

/// Serve one client until it asks to shut down. The caller has already done the
/// `initialize` handshake.
pub fn run(connection: &Connection) -> Result<()> {
    let mut documents: HashMap<Url, String> = HashMap::new();
    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    return Ok(());
                }
                let response = handle_request(&documents, request);
                connection.sender.send(Message::Response(response))?;
            }
            Message::Notification(notification) => {
                handle_notification(connection, &mut documents, notification)?;
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}
