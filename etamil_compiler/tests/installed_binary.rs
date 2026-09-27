// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! What a user gets when the compiler is installed by a package manager.
//!
//! `cargo install`, `pip install`, `brew install` and a bare `winget` place an
//! executable and nothing else — no `nUlakam/` beside it, no `ETAMIL_PATH`.
//! Installed that way the compiler used to answer "cannot open module" to the
//! first line of the README's own money example, and nothing in the suite
//! noticed, because every existing test runs inside the repository where the
//! standard library is simply *there*.
//!
//! These tests run the real binary from a temporary directory with the
//! environment cleared, which is the only arrangement that can tell the
//! difference.

use std::process::{Command, Output};

/// Run a command, waiting out a copy that the kernel still holds open.
///
/// `fs::copy` closes the file it writes, but `cargo test` runs tests on
/// several threads, and a spawn from any of them can inherit that descriptor
/// between the open and the close. Linux then refuses to exec the file --
/// ETXTBSY, `ExecutableFileBusy` -- until the inherited copy is gone.
///
/// It is a race rather than a state: the same test passes on the next attempt.
/// CI hit it once in this file while four other tests copied and ran the same
/// binary successfully in the same job. Retrying is what the race deserves;
/// the alternative is serialising every test here for a window measured in
/// milliseconds.
fn run_when_not_busy(command: &mut Command) -> Output {
    let mut waited = std::time::Duration::ZERO;
    for attempt in 0..8 {
        match command.output() {
            Ok(output) => return output,
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                let pause = std::time::Duration::from_millis(20 << attempt);
                std::thread::sleep(pause);
                waited += pause;
            }
            Err(error) => panic!("run the compiler: {error:?}"),
        }
    }
    panic!("the copied compiler was still busy after {waited:?}");
}

/// Run the built binary on `source`, from a scratch directory, with every
/// route to an on-disk standard library removed.
fn run_isolated(source: &str) -> (bool, String) {
    let directory = std::env::temp_dir().join(format!(
        "etamil-installed-{}-{}",
        std::process::id(),
        source.len()
    ));
    std::fs::create_dir_all(&directory).expect("temp directory");
    let program = directory.join("program.qmz");
    std::fs::write(&program, source).expect("write program");

    // The binary is copied out of target/ so that "next to the compiler" finds
    // nothing either — in the repository that lookup would otherwise reach the
    // real library through the build directory's parents.
    let installed = directory.join(if cfg!(windows) {
        "etamil.exe"
    } else {
        "etamil"
    });
    std::fs::copy(env!("CARGO_BIN_EXE_etamil"), &installed).expect("copy the binary");

    let output = run_when_not_busy(
        Command::new(&installed)
            .arg("--vm")
            .arg(&program)
            .current_dir(&directory)
            .env_remove("ETAMIL_PATH"),
    );

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_dir_all(&directory);
    (output.status.success(), combined)
}

#[test]
fn the_standard_library_is_importable_with_nothing_on_disk() {
    // The README's money example, which is the first thing a new user runs.
    let (ok, output) = run_isolated("இறக்கு \"nUlakam/paNam/paNam.qmz\";\nஅச்சு ரூபாய்(12345678.5);\n");

    assert!(
        ok,
        "an installed binary could not run the money example:\n{output}"
    );
    assert!(
        output.contains("1,23,45,678.50"),
        "expected Indian digit grouping, got:\n{output}"
    );
}

#[test]
fn a_library_module_that_imports_its_neighbours_resolves() {
    // kaNakkiyal/ reaches one directory up — `../kaNiqam.qmz` and friends
    // appear throughout it — so this exercises relative resolution *inside*
    // the embedded tree, which has no filesystem to canonicalize against.
    let (ok, output) = run_isolated("இறக்கு \"nUlakam/kaNakkiyal/pErEtu.qmz\";\nஅச்சு \"ok\";\n");

    assert!(
        ok,
        "a relatively-importing library module failed:\n{output}"
    );
    assert!(output.contains("ok"), "{output}");
}

#[test]
fn a_missing_module_still_says_where_it_looked() {
    let (ok, output) = run_isolated("இறக்கு \"illY/kOppu.qmz\";\n");

    assert!(!ok, "a missing module should fail:\n{output}");
    assert!(
        output.contains("built-in"),
        "the error should mention the built-in library among the places searched:\n{output}"
    );
}

#[test]
fn a_file_on_disk_overrides_the_built_in_copy() {
    // Otherwise the standard library could not be worked on without
    // rebuilding the compiler, and a distribution package could not ship a
    // newer library than the binary it sits beside.
    let directory = std::env::temp_dir().join(format!("etamil-override-{}", std::process::id()));
    // The override has to sit where the import now points: paNam.qmz moved
    // into nUlakam/paNam/ when the library was given a folder structure.
    let library = directory.join("nUlakam").join("paNam");
    std::fs::create_dir_all(&library).expect("temp directory");
    std::fs::write(
        library.join("paNam.qmz"),
        "செயல் ரூபாய்(தொகை) { திரும்பு \"from disk\"; }\n",
    )
    .expect("write the override");
    std::fs::write(
        directory.join("program.qmz"),
        "இறக்கு \"nUlakam/paNam/paNam.qmz\";\nஅச்சு ரூபாய்(1);\n",
    )
    .expect("write program");

    let installed = directory.join(if cfg!(windows) {
        "etamil.exe"
    } else {
        "etamil"
    });
    std::fs::copy(env!("CARGO_BIN_EXE_etamil"), &installed).expect("copy the binary");

    let output = run_when_not_busy(
        Command::new(&installed)
            .arg("--vm")
            .arg(directory.join("program.qmz"))
            .current_dir(&directory)
            .env_remove("ETAMIL_PATH"),
    );

    let combined = String::from_utf8_lossy(&output.stdout).into_owned();
    let _ = std::fs::remove_dir_all(&directory);

    assert!(
        combined.contains("from disk"),
        "a local nUlakam/ must win over the embedded copy:\n{combined}"
    );
}

#[test]
fn the_binary_reports_how_many_modules_it_carries() {
    let expected = etamil_compiler::stdlib::count();
    assert!(
        expected >= 30,
        "only {expected} modules embedded; the library has around 41"
    );

    let mut paths: Vec<&str> = etamil_compiler::stdlib::paths().collect();
    paths.sort_unstable();
    let mut deduped = paths.clone();
    deduped.dedup();
    assert_eq!(paths.len(), deduped.len(), "duplicate module keys embedded");
}
