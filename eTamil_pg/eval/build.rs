// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//
// Writes the list of builtins a database function may call, in every spelling.
//
// The compiler keeps its builtins as the arms of one `match` in
// src/vm/interpreter.rs, each arm listing every spelling of one function
// (`"நீளம்" | "_length" | "length" => {`), and its own build script reads them the
// same way. This one does too, and keeps an arm only if its ASCII alias is on the
// list below. So a builtin the compiler gains later is NOT allowed until someone
// reads it and adds its alias here: the safe answer is the default.
//
// The list is the builtins that compute from their arguments and touch nothing
// else: strings, arithmetic, collections, JSON, result values, and reading the clock.
// Everything else is left out on purpose, in groups:
//
//   files and the web   _fileExists _fileInfo _fileSave _readDir _packageRead _packageWrite
//                       _respondFile _saveUpload _httpGet _httpPost _httpRequest _tokenHeader
//   other databases     _mongo* _redis* _tryQuery _tryExecute
//   the host            _env _exit _run _sleepMs
//   hardware            _analogRead _boardName _pin* _tone _noTone _serial* _sim* _watchdog*
//   keys and secrets    _encrypt _decrypt _encryptionKey _keyPair _publicKey _sign
//                       _verifySignature _ecSign _ecVerify _issueToken _readToken _verifyTokenRSA

use std::path::PathBuf;

const PURE: &[&str] = &[
    "_addDays", "_append", "_bytes", "_ceil", "_daysBetween", "_err", "_exp", "_fieldOr", "_floor",
    "_fromBytes", "_hasField", "_hashPassword", "_isErr", "_isOk", "_join", "_jsonParse",
    "_jsonStringify", "_length", "_ln", "_log10", "_lower", "_millis", "_nowSeconds", "_ok", "_pow",
    "_replace", "_round", "_sort", "_sortByField", "_split", "_sqrt", "_toNumber", "_toString",
    "_today", "_typeof", "_unwrap", "_unwrapErr", "_unwrapOr", "_upper", "_verifyPassword",
];

fn builtin_arm(line: &str) -> Option<Vec<String>> {
    let head = line.trim().strip_suffix('{')?.trim_end().strip_suffix("=>")?.trim_end();
    let mut forms = Vec::new();
    for part in head.split('|') {
        let inner = part.trim().strip_prefix('"')?.strip_suffix('"')?;
        if inner.is_empty() || inner.contains('"') {
            return None;
        }
        forms.push(inner.to_string());
    }
    let tamil = |c: char| ('\u{0B80}'..='\u{0BFF}').contains(&c);
    if forms.len() < 2 || !forms[0].chars().any(tamil) {
        return None;
    }
    Some(forms)
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let interpreter = manifest.join("../../etamil_compiler/src/vm/interpreter.rs");
    println!("cargo:rerun-if-changed={}", interpreter.display());
    let source = std::fs::read_to_string(&interpreter)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", interpreter.display()));

    let arms: Vec<Vec<String>> = source.lines().filter_map(builtin_arm).collect();

    // Every alias on the list must still exist: a rename in the compiler would
    // otherwise shrink the allowed set without anyone being told.
    let missing: Vec<&&str> = PURE
        .iter()
        .filter(|alias| !arms.iter().any(|forms| forms.iter().any(|f| f == **alias)))
        .collect();
    if !missing.is_empty() {
        panic!("these allowed builtins are not in the interpreter any more: {missing:?}");
    }

    let mut allowed: Vec<String> = arms
        .iter()
        .filter(|forms| forms.iter().any(|f| PURE.contains(&f.as_str())))
        .flatten()
        .cloned()
        .collect();
    allowed.sort();
    allowed.dedup();

    let mut out = String::from("pub static ALLOWED_BUILTINS: &[&str] = &[\n");
    for name in &allowed {
        out.push_str(&format!("    {:?},\n", name));
    }
    out.push_str("];\n");
    out.push_str(&format!("pub const ALLOWED_ALIASES: usize = {};\n", PURE.len()));
    let destination = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("allowed_builtins.rs");
    std::fs::write(destination, out).expect("could not write allowed_builtins.rs");
}
