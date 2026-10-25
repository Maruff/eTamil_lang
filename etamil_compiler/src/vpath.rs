// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Paths in a virtual file tree: no filesystem, so no canonicalizing.
//!
//! Both the built-in standard library (`crate::stdlib`) and a project handed to the
//! browser build (`crate::project`) are sets of named sources, not directories. An
//! import inside one is resolved by folding `.` and `..` out of the importing file's
//! folder plus the relative path, which is all this does. It touches nothing outside
//! the strings it is given, so it compiles for the browser too.

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
/// eleven times — so resolving inside a virtual tree needs real path
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
                // simply cannot name a module in the tree, and the caller falls
                // through to "module not found" like any other bad path.
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Fold `.`, `..` and duplicate or backslash separators into one plain form.
pub fn normalise(path: &str) -> String {
    join("", &path.replace('\\', "/"))
}
