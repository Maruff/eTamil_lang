//! This crate provides eTamil language support for the [tree-sitter] parsing library.
//!
//! Typically, you will use the [`LANGUAGE`] constant to add this language to a
//! tree-sitter [`Parser`], and then use the parser to parse some code:
//!
//! ```
//! let code = r#"
//! ceyal add(a, b) { qirumpu a + b; }
//! total = add(1, 2);
//! "#;
//! let mut parser = tree_sitter::Parser::new();
//! let language = tree_sitter_etamil::LANGUAGE;
//! parser
//!     .set_language(&language.into())
//!     .expect("Error loading eTamil parser");
//! let tree = parser.parse(code, None).unwrap();
//! assert!(!tree.root_node().has_error());
//! ```
//!
//! [`Parser`]: https://docs.rs/tree-sitter/0.25.10/tree_sitter/struct.Parser.html
//! [tree-sitter]: https://tree-sitter.github.io/

use tree_sitter_language::LanguageFn;

extern "C" {
    fn tree_sitter_etamil() -> *const ();
}

/// The tree-sitter [`LanguageFn`] for this grammar.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_etamil) };

/// The content of the [`node-types.json`] file for this grammar.
///
/// [`node-types.json`]: https://tree-sitter.github.io/tree-sitter/using-parsers/6-static-node-types
pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

/// Syntax highlighting.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/highlights.scm");
/// Other languages inside eTamil: the SQL given to a database statement, and comments.
pub const INJECTIONS_QUERY: &str = include_str!("../../queries/injections.scm");
/// Scopes and bindings, for rename and reference highlighting.
pub const LOCALS_QUERY: &str = include_str!("../../queries/locals.scm");
/// Function and shape definitions, and calls, for symbol outlines.
pub const TAGS_QUERY: &str = include_str!("../../queries/tags.scm");

#[cfg(test)]
mod tests {
    fn parser() -> tree_sitter::Parser {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("Error loading eTamil parser");
        parser
    }

    #[test]
    fn test_can_load_grammar() {
        parser();
    }

    #[test]
    fn test_parses_a_program_without_errors() {
        let code = "ceyal add(a, b) { qirumpu a + b; }
total = add(1, 2);
";
        let tree = parser().parse(code, None).unwrap();
        assert!(!tree.root_node().has_error(), "{}", tree.root_node().to_sexp());
    }

    #[test]
    fn test_every_query_compiles_against_the_grammar() {
        // A query that names a node or field the grammar lacks fails to compile, and
        // an editor then loses the whole language, not just that capture.
        let language = super::LANGUAGE.into();
        for (name, source) in [
            ("highlights", super::HIGHLIGHTS_QUERY),
            ("injections", super::INJECTIONS_QUERY),
            ("locals", super::LOCALS_QUERY),
            ("tags", super::TAGS_QUERY),
        ] {
            if let Err(error) = tree_sitter::Query::new(&language, source) {
                panic!("{name}.scm does not compile: {error}");
            }
        }
    }
}
