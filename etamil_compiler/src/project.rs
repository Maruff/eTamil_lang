// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! A project: a set of named sources that import each other.
//!
//! The browser has no files to open, so `module.rs`, which reads them, is not part of
//! its build. A page can instead hand the compiler every source it has, by path, and
//! ask for one of them to be assembled into a program: `இறக்கு "kaNakku.qmz";` then
//! finds `kaNakku.qmz` among the sources given, resolved relative to the importing
//! file as on disk. The standard library works the same way: a page that wants
//! `இறக்கு "nUlakam/paNam/paNam.qmz";` to run puts those sources in the set.
//!
//! The rules are `module.rs`'s: each import's statements are spliced in ahead of the
//! importer's own, a file imported twice is included once, a cycle stops instead of
//! looping, and two modules defining the same name is an error (the importing program
//! may define whatever it likes).
//!
//! Pure: it reads nothing but what it is given, so it compiles for the browser.

use std::collections::{HashMap, HashSet};

use crate::lexer;
use crate::parser::{Parser, Stmt};
use crate::vpath;

/// Parse one source string into statements, with lexical errors reported.
pub(crate) fn parse_source(source: &str) -> Result<Vec<Stmt>, String> {
    let tokens = lexer::tokenize(source).map_err(|errors| {
        errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n  ")
    })?;
    let mut parser = Parser::new(tokens.iter());
    // Parse errors carry a line and column now, so the message a caller sees
    // says where to look rather than only what was wrong.
    parser.parse().map_err(|error| error.to_string())
}

/// The error for a name defined by two modules.
pub(crate) fn collision(name: &str, earlier: &str, label: &str) -> String {
    format!(
        "'{}' இரண்டு தொகுதிகளில் வரையறுக்கப்பட்டுள்ளது: '{}' மற்றும் '{}'  \
         ('{}' is defined in two modules, '{}' and '{}'. Imports are \
         flattened, so one would silently replace the other — rename one \
         of them.)",
        name, earlier, label, name, earlier, label
    )
}

fn module_not_found(relative: &str) -> String {
    format!(
        "தொகுதி '{}' கண்டுபிடிக்க முடியவில்லை  (cannot open module '{}'): \
         it is not one of the project's files",
        relative, relative
    )
}

/// Assemble the program that starts at `entry`, taking every import from `files`.
///
/// `files` maps a path to its source. Paths are folded the way imports are
/// (`.`, `..` and backslashes), so `src\a.qmz` and `./src/a.qmz` are one file.
pub fn load(files: &HashMap<String, String>, entry: &str) -> Result<Vec<Stmt>, String> {
    let mut loader = Loader {
        files: files
            .iter()
            .map(|(path, source)| (vpath::normalise(path), source.as_str()))
            .collect(),
        visited: HashSet::new(),
        defined: HashMap::new(),
    };
    let entry = vpath::normalise(entry);
    if !loader.files.contains_key(&entry) {
        return Err(format!(
            "திட்டத்தில் '{}' கோப்பு இல்லை  (there is no file '{}' in the project)",
            entry, entry
        ));
    }
    loader.module(&entry, true)
}

struct Loader<'a> {
    /// Normalised path to source.
    files: HashMap<String, &'a str>,
    visited: HashSet<String>,
    /// Which module wrote each function a module defines.
    defined: HashMap<String, String>,
}

impl Loader<'_> {
    fn module(&mut self, path: &str, is_entry: bool) -> Result<Vec<Stmt>, String> {
        let source = match self.files.get(path) {
            Some(source) => *source,
            None => return Err(module_not_found(path)),
        };
        if !self.visited.insert(path.to_string()) {
            return Ok(Vec::new()); // already imported
        }

        let directory = vpath::parent(path);
        let mut out = Vec::new();
        for statement in parse_source(source)? {
            // Only definitions written *in this file* are registered here. A nested
            // import registers its own under its own name when it is resolved, so
            // every name is attributed to the file that wrote it.
            if !is_entry
                && let Stmt::FunctionDef { name, .. } = &statement
                && let Some(earlier) = self.defined.insert(name.clone(), path.to_string())
                && earlier != path
            {
                return Err(collision(name, &earlier, path));
            }
            match statement {
                Stmt::Import(relative) => {
                    let target = vpath::join(&directory, &relative);
                    out.extend(self.module(&target, false)?);
                }
                other => out.push(other),
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{BytecodeCompiler, VM, host};

    fn project(files: &[(&str, &str)]) -> HashMap<String, String> {
        files
            .iter()
            .map(|(path, source)| (path.to_string(), source.to_string()))
            .collect()
    }

    fn definitions(statements: &[Stmt]) -> Vec<String> {
        statements
            .iter()
            .filter_map(|s| match s {
                Stmt::FunctionDef { name, .. } => Some(name.clone()),
                _ => None,
            })
            .collect()
    }

    /// Assemble, check, compile and run, collecting what the program printed.
    fn run(files: &HashMap<String, String>, entry: &str) -> Result<String, String> {
        let statements = load(files, entry)?;
        crate::check::check(&statements).map_err(|errors| {
            errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        })?;
        host::begin_capture();
        let outcome = VM::new().execute(BytecodeCompiler::compile_statements(statements));
        let printed = host::end_capture();
        outcome.map(|()| printed)
    }

    const ONE: &str = "செயல் moqqam(a) {\n  திரும்பு a + 1;\n}\n";

    #[test]
    fn a_single_file_loads_as_it_is() {
        let files = project(&[("main.qmz", ONE)]);
        assert_eq!(definitions(&load(&files, "main.qmz").unwrap()), ["moqqam"]);
    }

    #[test]
    fn an_import_is_spliced_in_ahead_of_the_importer() {
        let files = project(&[
            ("main.qmz", "இறக்கு \"vari.qmz\";\nஅச்சு vari(100);\n"),
            ("vari.qmz", "செயல் vari(a) {\n  திரும்பு a * 2;\n}\n"),
        ]);
        let statements = load(&files, "main.qmz").unwrap();
        assert_eq!(definitions(&statements), ["vari"]);
        assert!(matches!(statements[0], Stmt::FunctionDef { .. }));
    }

    #[test]
    fn imports_resolve_relative_to_the_importing_file() {
        let files = project(&[
            ("main.qmz", "இறக்கு \"lib/a.qmz\";\n"),
            (
                "lib/a.qmz",
                "இறக்கு \"../shared/b.qmz\";\nசெயல் a() {\n  திரும்பு 1;\n}\n",
            ),
            ("shared/b.qmz", "செயல் b() {\n  திரும்பு 2;\n}\n"),
        ]);
        let mut names = definitions(&load(&files, "main.qmz").unwrap());
        names.sort();
        assert_eq!(names, ["a", "b"]);
    }

    #[test]
    fn a_file_imported_twice_is_included_once() {
        let files = project(&[
            ("main.qmz", "இறக்கு \"a.qmz\";\nஇறக்கு \"b.qmz\";\n"),
            ("a.qmz", "இறக்கு \"c.qmz\";\n"),
            ("b.qmz", "இறக்கு \"c.qmz\";\n"),
            ("c.qmz", "செயல் c() {\n  திரும்பு 3;\n}\n"),
        ]);
        assert_eq!(definitions(&load(&files, "main.qmz").unwrap()), ["c"]);
    }

    #[test]
    fn a_cycle_stops_instead_of_looping() {
        let files = project(&[
            ("a.qmz", "இறக்கு \"b.qmz\";\nசெயல் a() {\n  திரும்பு 1;\n}\n"),
            ("b.qmz", "இறக்கு \"a.qmz\";\nசெயல் b() {\n  திரும்பு 2;\n}\n"),
        ]);
        let mut names = definitions(&load(&files, "a.qmz").unwrap());
        names.sort();
        assert_eq!(names, ["a", "b"]);
    }

    #[test]
    fn two_modules_defining_one_name_is_an_error_naming_both() {
        let files = project(&[
            ("main.qmz", "இறக்கு \"a.qmz\";\nஇறக்கு \"b.qmz\";\n"),
            ("a.qmz", ONE),
            ("b.qmz", ONE),
        ]);
        let error = load(&files, "main.qmz").unwrap_err();
        assert!(error.contains("moqqam"), "{error}");
        assert!(
            error.contains("a.qmz") && error.contains("b.qmz"),
            "{error}"
        );
    }

    #[test]
    fn the_program_itself_may_redefine_what_a_module_defines() {
        let files = project(&[
            (
                "main.qmz",
                "இறக்கு \"a.qmz\";\nசெயல் moqqam(a) {\n  திரும்பு 0;\n}\n",
            ),
            ("a.qmz", ONE),
        ]);
        assert!(load(&files, "main.qmz").is_ok());
    }

    #[test]
    fn a_module_that_is_not_in_the_project_is_named() {
        let files = project(&[("main.qmz", "இறக்கு \"missing.qmz\";\n")]);
        let error = load(&files, "main.qmz").unwrap_err();
        assert!(error.contains("missing.qmz"), "{error}");
        assert!(error.contains("not one of the project's files"), "{error}");
    }

    #[test]
    fn an_entry_that_is_not_in_the_project_is_an_error() {
        let files = project(&[("main.qmz", ONE)]);
        assert!(load(&files, "other.qmz").unwrap_err().contains("other.qmz"));
    }

    #[test]
    fn paths_are_folded_so_one_file_has_one_name() {
        let files = project(&[
            ("src\\main.qmz", "இறக்கு \"./a.qmz\";\n"),
            ("./src/a.qmz", ONE),
        ]);
        assert_eq!(
            definitions(&load(&files, "src/main.qmz").unwrap()),
            ["moqqam"]
        );
    }

    #[test]
    fn climbing_above_the_root_stays_at_the_root_and_names_only_what_was_given() {
        // `..` past the top folds to the top, as it does in the built-in library: it
        // widens nothing. A path that is not one of the files given is simply not found.
        let files = project(&[
            ("main.qmz", "இறக்கு \"../../shared.qmz\";\n"),
            ("shared.qmz", ONE),
        ]);
        assert_eq!(definitions(&load(&files, "main.qmz").unwrap()), ["moqqam"]);

        let alone = project(&[("main.qmz", "இறக்கு \"../../elsewhere.qmz\";\n")]);
        assert!(
            load(&alone, "main.qmz")
                .unwrap_err()
                .contains("elsewhere.qmz")
        );
    }

    #[test]
    fn a_syntax_error_in_an_imported_file_is_reported() {
        let files = project(&[
            ("main.qmz", "இறக்கு \"bad.qmz\";\n"),
            ("bad.qmz", "செயல் (\n"),
        ]);
        assert!(load(&files, "main.qmz").is_err());
    }

    #[test]
    fn a_project_runs_end_to_end_through_an_import() {
        let files = project(&[
            (
                "main.qmz",
                "இறக்கு \"vari.qmz\";\nஅச்சு moqqam_vari(1000, 18);\n",
            ),
            (
                "vari.qmz",
                "செயல் moqqam_vari(qokY, vikiqam) {\n  திரும்பு qokY * (1 + vikiqam / 100);\n}\n",
            ),
        ]);
        assert_eq!(run(&files, "main.qmz").unwrap(), "1180\n");
    }

    #[test]
    fn the_standard_library_runs_when_its_sources_are_in_the_set() {
        // What a page that wants `இறக்கு "nUlakam/paNam/paNam.qmz"` to work supplies:
        // the library's own sources, under the paths an author writes.
        let mut files = HashMap::new();
        for path in crate::stdlib::paths() {
            files.insert(
                path.to_string(),
                crate::stdlib::source(path).unwrap().to_string(),
            );
        }
        files.insert(
            "main.qmz".to_string(),
            "இறக்கு \"nUlakam/paNam/paNam.qmz\";\nஅச்சு ரூபாய்(1500);\n".to_string(),
        );
        let printed = run(&files, "main.qmz").unwrap();
        assert!(printed.contains("1,500"), "{printed}");
    }
}
