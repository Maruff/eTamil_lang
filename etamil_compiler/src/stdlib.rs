// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! The standard library, carried inside the binary.
//!
//! `nUlakam/` is eTamil source, so it can travel as text. `build.rs` compiles
//! it into the table below; this module addresses that table the way an author
//! addresses the real directory, so `இறக்கு "nUlakam/paNam/paNam.qmz"` resolves with
//! or without a filesystem copy.
//!
//! This is the *last* thing `module::locate` tries. Anything on disk wins, so
//! a checkout, `ETAMIL_PATH`, a distribution package or a local edit all still
//! override the built-in copy — which is what makes it possible to work on the
//! library itself without rebuilding the compiler.

include!(concat!(env!("OUT_DIR"), "/embedded_stdlib.rs"));

/// The source of an embedded module, addressed as it would be on disk.
pub fn source(virtual_path: &str) -> Option<&'static str> {
    let key = normalise(virtual_path);
    EMBEDDED_STDLIB
        .binary_search_by(|(candidate, _)| candidate.cmp(&key.as_str()))
        .ok()
        .map(|index| EMBEDDED_STDLIB[index].1)
}

/// Is this path one the embedded library carries?
pub fn contains(virtual_path: &str) -> bool {
    source(virtual_path).is_some()
}

/// The virtual directory holding `virtual_path`, for resolving its own imports.
pub fn parent(virtual_path: &str) -> String {
    let normalised = normalise(virtual_path);
    match normalised.rfind('/') {
        Some(cut) => normalised[..cut].to_string(),
        None => String::new(),
    }
}

/// Join a relative import onto a virtual directory.
///
/// The library imports its neighbours relatively — `../kaNiqam.qmz` appears
/// eleven times — so resolving inside the embedded tree needs real path
/// arithmetic, not string concatenation. There is no filesystem here to
/// canonicalize against, so `.` and `..` are folded here.
pub fn join(base: &str, relative: &str) -> String {
    if relative.starts_with('/') {
        return normalise(relative);
    }

    let mut parts: Vec<&str> = Vec::new();
    for segment in base.split('/').chain(relative.split('/')) {
        match segment {
            "" | "." => {}
            ".." => {
                // Climbing above the root is not an error to report here; it
                // simply cannot name an embedded module, and the caller falls
                // through to "module not found" like any other bad path.
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Fold `.`, `..` and duplicate or backslash separators into one plain form.
fn normalise(path: &str) -> String {
    join("", &path.replace('\\', "/"))
}

/// How many modules are built in. Used by the tests and by `--version`.
pub fn count() -> usize {
    EMBEDDED_STDLIB.len()
}

/// Every embedded module path, in sorted order.
pub fn paths() -> impl Iterator<Item = &'static str> {
    EMBEDDED_STDLIB.iter().map(|(path, _)| *path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_is_actually_embedded() {
        // If build.rs silently produced nothing, every other test here would
        // still pass by vacuous truth, so assert the table is populated first.
        assert!(
            count() >= 30,
            "only {} modules embedded; the standard library has ~41",
            count()
        );
    }

    #[test]
    fn a_module_resolves_the_way_an_author_writes_it() {
        assert!(contains("nUlakam/paNam/paNam.qmz"));
        assert!(contains("nUlakam/atippatY/col.qmz"));
        assert!(contains("nUlakam/kaNakkiyal/pErEtu.qmz"));
    }

    #[test]
    fn an_embedded_module_carries_its_source() {
        let source = source("nUlakam/paNam/paNam.qmz").expect("paNam.qmz");
        assert!(source.contains("ரூபாய்"), "paNam.qmz should define ரூபாய்");
    }

    #[test]
    fn relative_imports_inside_the_library_resolve() {
        // The shape that actually occurs: kaNakkiyal/ reaching one level up.
        let base = parent("nUlakam/kaNakkiyal/pErEtu.qmz");
        assert_eq!(base, "nUlakam/kaNakkiyal");
        assert_eq!(
            join(&base, "../atippatY/kaNiqam.qmz"),
            "nUlakam/atippatY/kaNiqam.qmz"
        );
        assert_eq!(
            join(&base, "kaNakkukaL.qmz"),
            "nUlakam/kaNakkiyal/kaNakkukaL.qmz"
        );
        assert!(contains(&join(&base, "../atippatY/kaNiqam.qmz")));
    }

    #[test]
    fn separators_and_dot_segments_are_folded() {
        assert_eq!(
            normalise("nUlakam//atippatY/col.qmz"),
            "nUlakam/atippatY/col.qmz"
        );
        assert_eq!(
            normalise("nUlakam/./atippatY/col.qmz"),
            "nUlakam/atippatY/col.qmz"
        );
        assert_eq!(
            normalise("nUlakam\\atippatY\\col.qmz"),
            "nUlakam/atippatY/col.qmz"
        );
        assert!(
            contains("nUlakam\\atippatY\\col.qmz"),
            "a Windows separator should still resolve"
        );
    }

    #[test]
    fn climbing_past_the_root_finds_nothing_rather_than_panicking() {
        assert_eq!(join("nUlakam", "../../../etc/passwd"), "etc/passwd");
        assert!(!contains("../../../etc/passwd"));
    }

    #[test]
    fn every_embedded_module_parses() {
        // The library is compiled in as text, so a syntax error in it would
        // otherwise only appear when a user imported that particular file.
        for path in paths() {
            let text = source(path).expect(path);
            crate::lexer::tokenize(text)
                .unwrap_or_else(|errors| panic!("{path} does not lex: {errors:?}"));
        }
    }
}
