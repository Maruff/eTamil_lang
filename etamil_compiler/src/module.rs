// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Module loading: `இறக்கு "kOppu.qmz";`
//!
//! A program is assembled by splicing each imported file's statements in
//! ahead of the importer's own. Paths resolve relative to the importing
//! file, a file imported twice is included once, and an import cycle stops
//! rather than looping — the same guarantees `#pragma once` gives, without
//! needing the author to think about it.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::parser::Stmt;
use crate::project::parse_source;

/// Load a program from disk, resolving its imports.
pub fn load_file(path: &Path) -> Result<Vec<Stmt>, String> {
    let mut visited = HashSet::new();
    let mut defined = HashMap::new();
    load_inner(path, &mut visited, &mut defined, true)
}

/// Load a program held in memory. Imports resolve relative to `base_dir`.
pub fn load_source(source: &str, base_dir: &Path) -> Result<Vec<Stmt>, String> {
    let mut visited = HashSet::new();
    let mut defined = HashMap::new();
    let statements = parse_source(source)?;
    resolve(statements, base_dir, &mut visited, &mut defined, true)
}

/// Where a module's imports resolve from.
///
/// A module read from disk resolves its own imports against its directory. One
/// answered from the built-in library has no directory, so it resolves against
/// its position in the embedded tree instead — which matters because the
/// library imports its neighbours relatively (`../kaNiqam.qmz`).
enum Origin {
    Disk(PathBuf),
    Embedded(String),
}

/// Parse an embedded module and resolve whatever it imports.
fn load_embedded(
    virtual_path: &str,
    visited: &mut HashSet<PathBuf>,
    defined: &mut HashMap<String, String>,
) -> Result<Vec<Stmt>, String> {
    let source = crate::stdlib::source(virtual_path).ok_or_else(|| {
        format!(
            "உள்ளமைந்த தொகுதி '{}' இல்லை  (no built-in module '{}')",
            virtual_path, virtual_path
        )
    })?;

    // Keyed apart from any real path so a file on disk and the built-in copy
    // of the same module are never mistaken for one another.
    let key = PathBuf::from(format!(
        "<built-in>/{}",
        crate::stdlib::parent(virtual_path)
    ))
    .join(virtual_path);
    if !visited.insert(key) {
        return Ok(Vec::new()); // already imported
    }

    let statements = parse_source(source)?;
    resolve_from(
        statements,
        &Origin::Embedded(crate::stdlib::parent(virtual_path)),
        visited,
        defined,
        virtual_path,
        false,
    )
}

fn load_inner(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    defined: &mut HashMap<String, String>,
    is_entry: bool,
) -> Result<Vec<Stmt>, String> {
    // Canonicalize so the same file reached by two different paths is still
    // recognised as already imported.
    let canonical = path.canonicalize().map_err(|e| {
        format!(
            "கோப்பு '{}' திறக்க முடியவில்லை  (cannot open '{}'): {}",
            path.display(),
            path.display(),
            e
        )
    })?;

    if !visited.insert(canonical.clone()) {
        return Ok(Vec::new()); // already imported
    }

    let source = std::fs::read_to_string(&canonical).map_err(|e| {
        format!(
            "கோப்பு '{}' படிக்க முடியவில்லை  (cannot read '{}'): {}",
            canonical.display(),
            canonical.display(),
            e
        )
    })?;

    let statements = parse_source(&source)?;
    let base_dir = canonical
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    // The file's own name, for the collision message. The whole path
    // would be accurate and unreadable; the file name is what someone
    // recognises and what they will grep for.
    let label = canonical
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| canonical.display().to_string());
    resolve_from(
        statements,
        &Origin::Disk(base_dir),
        visited,
        defined,
        &label,
        is_entry,
    )
}

/// Find an imported file: next to the importer first, then along
/// `ETAMIL_PATH`, then in a `nUlakam` directory beside the executable. That
/// last one is what lets `இறக்கு "nUlakam/paNam/paNam.qmz";` work from anywhere
/// once the compiler is installed.
fn locate(relative: &str, base_dir: &Path) -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = vec![base_dir.to_path_buf()];

    if let Ok(search_path) = std::env::var("ETAMIL_PATH") {
        roots.extend(std::env::split_paths(&search_path));
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        roots.push(dir.to_path_buf());
    }

    // Native packages keep the standard library in the platform data
    // directory rather than beside the executable in /usr/bin.
    roots.push(PathBuf::from("/usr/share/etamil"));
    roots.push(PathBuf::from("/usr/local/share/etamil"));

    for root in &roots {
        let candidate = root.join(relative);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    one_level_deeper(relative, &roots)
}

/// The library was one flat directory until its files were grouped into folders
/// by subject. Every program published before that move says
/// `இறக்கு "nUlakam/paNam.qmz"`, so when the literal path finds nothing, look
/// one directory deeper for the same file name. Module file names are unique
/// across the library, so there is never a choice to make.
///
/// Only one level, and never through `..`: this widens where a name may be
/// found, and it should not widen what a name may reach.
fn one_level_deeper(relative: &str, roots: &[PathBuf]) -> Option<PathBuf> {
    if relative.split(['/', '\\']).any(|part| part == "..") {
        return None;
    }

    let asked = Path::new(relative);
    let file = asked.file_name()?;
    let holder = asked.parent().unwrap_or_else(|| Path::new(""));

    for root in roots {
        let Ok(entries) = std::fs::read_dir(root.join(holder)) else {
            continue;
        };
        let mut folders: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        // Sorted, so which file answers an import never depends on the order
        // the file system happens to hand back.
        folders.sort();
        for folder in folders {
            let candidate = folder.join(file);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

fn resolve(
    statements: Vec<Stmt>,
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    defined: &mut HashMap<String, String>,
    is_entry: bool,
) -> Result<Vec<Stmt>, String> {
    resolve_from(
        statements,
        &Origin::Disk(base_dir.to_path_buf()),
        visited,
        defined,
        "",
        is_entry,
    )
}

/// `label` names the module these statements were read from, for the error
/// message; `is_entry` marks the program itself, which may define whatever it
/// likes.
fn resolve_from(
    statements: Vec<Stmt>,
    origin: &Origin,
    visited: &mut HashSet<PathBuf>,
    defined: &mut HashMap<String, String>,
    label: &str,
    is_entry: bool,
) -> Result<Vec<Stmt>, String> {
    let mut out = Vec::new();
    for statement in statements {
        // Only definitions written *in this file* are registered here. A
        // nested import registers its own under its own name when it is
        // resolved, so every name is attributed to the file that wrote it.
        if !is_entry
            && let Stmt::FunctionDef { name, .. } = &statement
            && let Some(earlier) = defined.insert(name.clone(), label.to_string())
            && earlier != label
        {
            return Err(crate::project::collision(name, &earlier, label));
        }
        match statement {
            Stmt::Import(relative) => {
                let imported = match origin {
                    // On disk, the filesystem is tried first and the built-in
                    // library last, so a checkout, ETAMIL_PATH or a distribution
                    // package always overrides the copy inside the binary. That
                    // ordering is what lets the library be edited without
                    // rebuilding the compiler.
                    Origin::Disk(base_dir) => match locate(&relative, base_dir) {
                        Some(found) => load_inner(&found, visited, defined, false)?,
                        None if crate::stdlib::contains(&relative) => {
                            load_embedded(&relative, visited, defined)?
                        }
                        None => return Err(not_found(&relative)),
                    },
                    // Inside the built-in library, imports stay inside it. A
                    // module answered from the binary must not start reading
                    // the invoking user's working directory.
                    Origin::Embedded(virtual_dir) => {
                        let target = crate::stdlib::join(virtual_dir, &relative);
                        if crate::stdlib::contains(&target) {
                            load_embedded(&target, visited, defined)?
                        } else {
                            return Err(not_found(&relative));
                        }
                    }
                };
                out.extend(imported);
            }
            other => out.push(other),
        }
    }
    Ok(out)
}

fn not_found(relative: &str) -> String {
    format!(
        "தொகுதி '{}' கண்டுபிடிக்க முடியவில்லை  (cannot open module '{}'): \
         looked beside the importing file, along ETAMIL_PATH, next to the compiler, \
         and in the built-in standard library",
        relative, relative
    )
}
