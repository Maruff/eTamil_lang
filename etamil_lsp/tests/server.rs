// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Drives the server the way an editor does, over an in-memory connection.

use std::thread;

use etamil_lsp::{
    char_to_utf16, completions, definition, diagnostics, hover, run, utf16_to_char,
};
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{DidOpenTextDocument, Notification as _, PublishDiagnostics};
use lsp_types::request::{Completion, GotoDefinition, HoverRequest, Request as _};
use lsp_types::{
    CompletionResponse, DiagnosticSeverity, DidOpenTextDocumentParams, GotoDefinitionResponse,
    HoverContents, Position, PublishDiagnosticsParams, TextDocumentIdentifier, TextDocumentItem,
    Url,
};

/// A function, a call to it, and a variable that holds the result.
const PROGRAM: &str = "செயல் கூட்டு(அ, ஆ) {\n    திரும்பு அ + ஆ;\n}\nமொத்தம் = கூட்டு(1, 2);\n";

fn uri() -> Url {
    Url::parse("file:///work/sample.qmz").unwrap()
}

// --- positions ---------------------------------------------------------------

#[test]
fn utf16_counts_astral_characters_as_two_units() {
    // One emoji is two UTF-16 units but one character.
    assert_eq!(char_to_utf16("a😀b", 2), 3);
    assert_eq!(char_to_utf16("a😀b", 3), 4);
    assert_eq!(utf16_to_char("a😀b", 3), 2);
    assert_eq!(utf16_to_char("a😀b", 4), 3);
}

#[test]
fn tamil_letters_are_one_unit_each() {
    // Every Tamil code point is in the BMP: one character, one UTF-16 unit.
    let text = "வணக்கம்";
    assert_eq!(char_to_utf16(text, 3), 3);
    assert_eq!(utf16_to_char(text, 3), 3);
}

// --- features ----------------------------------------------------------------

#[test]
fn a_valid_program_has_no_diagnostics() {
    assert!(diagnostics(PROGRAM).is_empty(), "{:?}", diagnostics(PROGRAM));
}

#[test]
fn a_lexical_error_is_reported_at_its_position_in_both_languages() {
    let found = diagnostics("x = 1;\ny = @;\n");
    assert!(!found.is_empty());
    let first = &found[0];
    assert_eq!(first.severity, Some(DiagnosticSeverity::ERROR));
    assert_eq!(first.range.start.line, 1, "the error is on the second line");
    assert_eq!(first.source.as_deref(), Some("etamil"));
    // The compiler's message carries both languages.
    assert!(first.message.contains("unrecognized input"));
    assert!(first.message.contains("வரி"));
}

#[test]
fn completion_offers_the_names_in_scope() {
    // Inside the function body: its parameters, plus the top-level names.
    let inside = completions(PROGRAM, Position::new(1, 12));
    let names: Vec<&str> = inside.iter().map(|c| c.label.as_str()).collect();
    assert!(names.contains(&"அ") && names.contains(&"ஆ"), "{names:?}");
    assert!(names.contains(&"கூட்டு") && names.contains(&"மொத்தம்"), "{names:?}");

    // At top level the parameters are out of scope.
    let outside = completions(PROGRAM, Position::new(3, 0));
    let names: Vec<&str> = outside.iter().map(|c| c.label.as_str()).collect();
    assert!(!names.contains(&"அ"), "{names:?}");
    assert!(names.contains(&"கூட்டு"), "{names:?}");
}

#[test]
fn hover_on_a_function_call_shows_its_signature() {
    // Line 3 is `மொத்தம் = கூட்டு(1, 2);`; column 12 is inside கூட்டு.
    let shown = hover(PROGRAM, Position::new(3, 12)).expect("hover on the call");
    let HoverContents::Markup(markup) = shown.contents else {
        panic!("expected markup");
    };
    assert!(markup.value.contains("function"), "{}", markup.value);
    assert!(markup.value.contains("கூட்டு(அ, ஆ)"), "{}", markup.value);
}

#[test]
fn definition_of_a_call_is_the_function_declaration() {
    let found = definition(PROGRAM, &uri(), Position::new(3, 12)).expect("definition");
    // `செயல் கூட்டு` is on the first line; the name starts after `செயல் `.
    assert_eq!(found.range.start.line, 0);
    assert_eq!(found.range.start.character, 6);
}

#[test]
fn nothing_is_offered_where_the_file_does_not_lex() {
    assert!(hover("x = @;", Position::new(0, 0)).is_none());
}

// --- the protocol ------------------------------------------------------------

fn send_request<P: serde::Serialize>(client: &Connection, id: i32, method: &str, params: P) {
    client
        .sender
        .send(Message::Request(Request::new(
            RequestId::from(id),
            method.to_string(),
            params,
        )))
        .unwrap();
}

fn next_response(client: &Connection) -> Response {
    loop {
        if let Message::Response(response) = client.receiver.recv().unwrap() {
            return response;
        }
    }
}

#[test]
fn a_session_opens_a_file_and_asks_questions() {
    let (server, client) = Connection::memory();
    let handle = thread::spawn(move || run(&server));

    // Open a file with an error: diagnostics are pushed without being asked for.
    client
        .sender
        .send(Message::Notification(Notification::new(
            DidOpenTextDocument::METHOD.to_string(),
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem::new(
                    uri(),
                    "etamil".to_string(),
                    1,
                    "y = @;\n".to_string(),
                ),
            },
        )))
        .unwrap();
    let Message::Notification(published) = client.receiver.recv().unwrap() else {
        panic!("expected a notification");
    };
    assert_eq!(published.method, PublishDiagnostics::METHOD);
    let params: PublishDiagnosticsParams = serde_json::from_value(published.params).unwrap();
    assert_eq!(params.uri, uri());
    assert!(!params.diagnostics.is_empty());

    // Re-open with a valid program (same URI replaces the text) and query it.
    client
        .sender
        .send(Message::Notification(Notification::new(
            DidOpenTextDocument::METHOD.to_string(),
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem::new(
                    uri(),
                    "etamil".to_string(),
                    2,
                    PROGRAM.to_string(),
                ),
            },
        )))
        .unwrap();
    let Message::Notification(published) = client.receiver.recv().unwrap() else {
        panic!("expected a notification");
    };
    let params: PublishDiagnosticsParams = serde_json::from_value(published.params).unwrap();
    assert!(params.diagnostics.is_empty(), "the valid program clears the errors");

    let at = |line, character| lsp_types::TextDocumentPositionParams {
        text_document: TextDocumentIdentifier::new(uri()),
        position: Position::new(line, character),
    };

    send_request(
        &client,
        1,
        Completion::METHOD,
        lsp_types::CompletionParams {
            text_document_position: at(3, 0),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        },
    );
    let response = next_response(&client);
    let items: Option<CompletionResponse> = serde_json::from_value(response.result.unwrap()).unwrap();
    let Some(CompletionResponse::List(list)) = items else {
        panic!("expected a completion list");
    };
    // Incomplete: the vocabulary is filtered by what was typed, so the editor must ask
    // again as typing continues.
    assert!(list.is_incomplete);
    let items = list.items;
    assert!(items.iter().any(|i| i.label == "கூட்டு"));
    assert!(items.iter().any(|i| i.label == "எனில்"), "keywords are offered too");

    send_request(
        &client,
        2,
        HoverRequest::METHOD,
        lsp_types::HoverParams {
            text_document_position_params: at(3, 12),
            work_done_progress_params: Default::default(),
        },
    );
    assert!(!next_response(&client).result.unwrap().is_null());

    send_request(
        &client,
        3,
        GotoDefinition::METHOD,
        lsp_types::GotoDefinitionParams {
            text_document_position_params: at(3, 12),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
    );
    let response = next_response(&client);
    let target: Option<GotoDefinitionResponse> =
        serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(matches!(target, Some(GotoDefinitionResponse::Scalar(_))));

    // An unknown method is answered with an error, not ignored.
    send_request(&client, 4, "etamil/nonsense", serde_json::Value::Null);
    assert!(next_response(&client).error.is_some());

    // Shut down the protocol's way: a request, then an exit notification.
    send_request(&client, 5, "shutdown", serde_json::Value::Null);
    let response = next_response(&client);
    assert!(response.error.is_none());
    client
        .sender
        .send(Message::Notification(Notification::new(
            "exit".to_string(),
            serde_json::Value::Null,
        )))
        .unwrap();
    handle.join().unwrap().unwrap();
}
