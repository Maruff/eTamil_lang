// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Completion and hover for the language's own vocabulary: keywords, builtins and
//! the standard library.

use etamil_lsp::{catalog, completions, hover, word_prefix};
use lsp_types::{CompletionItem, CompletionItemKind, HoverContents, InsertTextFormat, Position};

/// The position just after the last character of `source` (which has no trailing newline).
fn end_of(source: &str) -> Position {
    let last = source.lines().count().saturating_sub(1);
    let text = source.lines().last().unwrap_or("");
    Position::new(last as u32, text.encode_utf16().count() as u32)
}

/// The position one character into the first occurrence of `needle`.
fn inside(source: &str, needle: &str) -> Position {
    for (row, line) in source.lines().enumerate() {
        if let Some(byte) = line.find(needle) {
            let before = line[..byte].encode_utf16().count() as u32;
            return Position::new(row as u32, before + 1);
        }
    }
    panic!("{needle} is not in the source");
}

fn find<'a>(items: &'a [CompletionItem], label: &str) -> &'a CompletionItem {
    items
        .iter()
        .find(|item| item.label == label)
        .unwrap_or_else(|| panic!("no completion labelled {label}"))
}

fn hover_text(source: &str, position: Position) -> String {
    match hover(source, position).expect("a hover").contents {
        HoverContents::Markup(markup) => markup.value,
        other => panic!("expected markup, got {other:?}"),
    }
}

/// A standard library function with parameters and a Tamil spelling, to build tests on.
fn library_function() -> (&'static catalog::Function, &'static str, &'static str) {
    let function = catalog::functions()
        .iter()
        .find(|f| f.kind == "stdlib" && f.module.is_some() && f.params.is_some() && !f.doc.is_empty())
        .expect("the catalog has standard library functions");
    let form = function.forms.iter().find(|form| catalog::is_tamil(form)).expect("a Tamil spelling");
    (function, form.as_str(), function.module.as_deref().unwrap())
}

#[test]
fn the_catalog_is_loaded_and_has_the_language_in_it() {
    assert!(catalog::keywords().len() > 150, "{} keywords", catalog::keywords().len());
    assert!(catalog::functions().len() > 900, "{} functions", catalog::functions().len());
    assert!(catalog::keywords().iter().all(|k| !k.forms.is_empty()));
}

#[test]
fn the_word_before_the_cursor_is_the_prefix() {
    assert_eq!(word_prefix("அச்சு எ", end_of("அச்சு எ")), "எ");
    assert_eq!(word_prefix("x = ceyal", end_of("x = ceyal")), "ceyal");
    assert_eq!(word_prefix("a_b1", end_of("a_b1")), "a_b1");
    assert_eq!(word_prefix("foo(", end_of("foo(")), "");
    assert_eq!(word_prefix("", Position::new(0, 0)), "");
}

#[test]
fn a_tamil_prefix_offers_keywords_in_tamil() {
    let items = completions("எ", end_of("எ"));
    let keyword = find(&items, "எனில்");
    assert_eq!(keyword.kind, Some(CompletionItemKind::KEYWORD));
    assert!(items.iter().all(|item| item.label != "eZil"), "a Latin spelling does not match a Tamil prefix");
}

#[test]
fn a_keyword_with_a_template_also_offers_the_whole_statement() {
    let items = completions("எ", end_of("எ"));
    let template = find(&items, "எனில் …");
    assert_eq!(template.kind, Some(CompletionItemKind::SNIPPET));
    assert_eq!(template.insert_text_format, Some(InsertTextFormat::SNIPPET));
    assert_eq!(template.filter_text.as_deref(), Some("எனில்"), "typing the keyword must match it");
    let text = template.insert_text.as_deref().unwrap();
    assert!(text.contains("எனில்") && !text.contains("{kw}"), "the spelling replaces the marker: {text}");
    // The plain keyword inserts only the word, so completing it in the middle of a
    // statement (eTamil writes the condition first) does not add a second statement.
    assert_eq!(find(&items, "எனில்").insert_text, None);
}

#[test]
fn a_latin_prefix_matches_ignoring_case() {
    let items = completions("EZ", end_of("EZ"));
    assert_eq!(find(&items, "eZil").kind, Some(CompletionItemKind::KEYWORD));
    let template = find(&items, "eZil …");
    assert!(template.insert_text.as_deref().unwrap().contains("condition"), "Latin spelling, Latin placeholders");
}

#[test]
fn with_nothing_typed_keywords_are_offered_but_not_the_thousand_functions() {
    let items = completions("", Position::new(0, 0));
    assert!(items.iter().any(|item| item.kind == Some(CompletionItemKind::KEYWORD)));
    assert!(
        items.iter().all(|item| item.kind != Some(CompletionItemKind::FUNCTION)),
        "functions wait for a first letter"
    );
    assert!(items.len() < 1000, "{} items", items.len());
}

#[test]
fn keywords_no_statement_uses_are_not_offered() {
    let items = completions("", Position::new(0, 0));
    for keyword in catalog::keywords().iter().filter(|k| k.no_syntax) {
        for form in &keyword.forms {
            assert!(
                items.iter().all(|item| &item.label != form),
                "{form} ({}) is reserved but unusable",
                keyword.token
            );
        }
    }
}

#[test]
fn a_library_function_arrives_with_its_signature_and_its_import() {
    let (function, form, module) = library_function();
    let prefix: String = form.chars().take(2).collect();
    let items = completions(&prefix, end_of(&prefix));
    let item = find(&items, form);

    assert_eq!(item.kind, Some(CompletionItemKind::FUNCTION));
    let params = function.params.as_ref().unwrap();
    assert!(item.detail.as_deref().unwrap().contains(&params.join(", ")), "{:?}", item.detail);
    assert!(item.insert_text.as_deref().unwrap().contains("${1:"), "parameters are tab stops");

    let edits = item.additional_text_edits.as_ref().expect("an import edit");
    assert_eq!(edits.len(), 1);
    assert!(edits[0].new_text.contains(module) && edits[0].new_text.contains(catalog::import_keyword()));
    assert!(edits[0].new_text.ends_with(";\n"));
}

#[test]
fn nothing_is_imported_twice() {
    let (_, form, module) = library_function();
    let prefix: String = form.chars().take(2).collect();
    let source = format!("{} \"{module}\";\n{prefix}", catalog::import_keyword());
    let items = completions(&source, end_of(&source));
    assert_eq!(find(&items, form).additional_text_edits, None);
}

#[test]
fn the_import_goes_after_the_comments_at_the_top() {
    let (_, form, _) = library_function();
    let prefix: String = form.chars().take(2).collect();
    let source = format!("// header\n// more\n\n{prefix}");
    let items = completions(&source, end_of(&source));
    let edit = &find(&items, form).additional_text_edits.as_ref().unwrap()[0];
    assert_eq!(edit.range.start, Position::new(3, 0), "the first line that is not a comment");
    assert_eq!(edit.range.start, edit.range.end);
}

#[test]
fn a_builtin_needs_no_import() {
    let function = catalog::functions().iter().find(|f| f.kind == "builtin").unwrap();
    let form = function.forms.iter().find(|form| catalog::is_tamil(form)).unwrap();
    let prefix: String = form.chars().take(2).collect();
    let items = completions(&prefix, end_of(&prefix));
    let item = find(&items, form);
    assert_eq!(item.additional_text_edits, None);
    assert_eq!(item.kind, Some(CompletionItemKind::FUNCTION));
}

#[test]
fn the_list_stays_small_for_a_short_prefix() {
    // A one-letter prefix is the worst case for a real session, since it is the first
    // thing typed. The whole vocabulary is over 2,000 spellings.
    let mut worst = 0;
    for prefix in ["ப", "க", "வ", "அ", "த", "c", "k", "v", "a"] {
        worst = worst.max(completions(prefix, end_of(prefix)).len());
    }
    assert!(worst < 1500, "{worst} items for the worst one-letter prefix");
}

#[test]
fn hover_on_a_standard_library_function_shows_its_documentation() {
    let (function, form, module) = library_function();
    let source = format!("r = {form}(1);\n");
    let text = hover_text(&source, inside(&source, form));
    assert!(text.contains("standard library function") && text.contains(module), "{text}");
    assert!(text.contains(&function.doc), "{text}");
}

#[test]
fn hover_on_a_keyword_lists_its_spellings() {
    let source = "(அ > 1) எனில் { அச்சு 1; }";
    let text = hover_text(source, inside(source, "எனில்"));
    assert!(text.contains("keyword") && text.contains("eZil") && text.contains("_if"), "{text}");
}

#[test]
fn a_name_defined_in_the_file_wins_over_the_library() {
    let (_, form, _) = library_function();
    let source = format!("ceyal {form}(a) {{ qirumpu a; }}\n{form}(1);\n");
    let text = hover_text(&source, Position::new(1, 1));
    assert!(text.contains("function") && !text.contains("standard library"), "{text}");
}
