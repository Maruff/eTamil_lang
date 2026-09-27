// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! `artino.toml`: the C++ a program may call, and the serial ports it adds.
//!
//! ```toml
//! [[library]]                 # an Arduino library the sketch needs
//! name = "Servo"
//!
//! [[include]]
//! header = "Servo.h"
//!
//! [[object]]                  # a global the shims use
//! cpp = "Servo kaqavu;"
//!
//! [[function]]                # an eTamil name for a C++ call
//! etamil = "கதவு_இணை"
//! cpp = "kaqavu.attach"      # called with the arguments, or a template: "f({0}, {1})"
//! args = ["int"]              # int | num | bool | text
//! returns = "void"            # void | int | num | bool | text
//!
//! [[port]]                    # a Stream as serial port 1–3, for the தொடர்_* functions
//! number = 1
//! cpp = "bus"                 # an object with begin(baud), declared above
//! boards = ["uno", "nano"]    # any entry: only on these boards
//! ```
//!
//! Each function becomes an `extern "C"` shim in the sketch's
//! `artino_shims.cpp`, which the program calls by number. A number reaches C++
//! as `int` (its whole part, a fraction reported) or `num` (a double), and
//! comes back the same way.
//!
//! The file is read as the subset of TOML above: tables of arrays, strings in
//! either quote, whole numbers, booleans and arrays of strings. Anything else is
//! refused with its line.

use std::path::{Path, PathBuf};

/// What a value is on the C++ side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Void,
    Int,
    Num,
    Bool,
    Text,
}

impl Kind {
    fn parse(word: &str, returns: bool) -> Option<Kind> {
        match word {
            "void" if returns => Some(Kind::Void),
            "int" => Some(Kind::Int),
            "num" => Some(Kind::Num),
            "bool" => Some(Kind::Bool),
            "text" => Some(Kind::Text),
            _ => None,
        }
    }

    /// The C++ type a shim takes or gives.
    fn cpp(self) -> &'static str {
        match self {
            Kind::Void => "void",
            Kind::Int | Kind::Bool => "int32_t",
            Kind::Num => "int64_t",
            Kind::Text => "const char *",
        }
    }
}

/// One `[[function]]`: an eTamil name for a C++ call.
#[derive(Debug, Clone, PartialEq)]
pub struct Extern {
    pub etamil: String,
    pub cpp: String,
    pub args: Vec<Kind>,
    pub returns: Kind,
}

impl Extern {
    /// The shim's symbol: by position, since an eTamil name is not a C one.
    pub fn symbol(index: usize) -> String {
        format!("artino_x_{}", index)
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Manifest {
    pub libraries: Vec<String>,
    pub includes: Vec<String>,
    pub objects: Vec<String>,
    pub functions: Vec<Extern>,
    /// `(port, object)`.
    pub ports: Vec<(u8, String)>,
}

/// The manifest beside a program: `<name>.artino.toml`, else `artino.toml`.
pub fn find(program: &Path) -> Option<PathBuf> {
    let dir = program.parent().unwrap_or_else(|| Path::new("."));
    let stem = program.file_stem()?.to_string_lossy().into_owned();
    [
        dir.join(format!("{}.artino.toml", stem)),
        dir.join("artino.toml"),
    ]
    .into_iter()
    .find(|p| p.is_file())
}

/// Read a manifest, keeping the entries for `board` (every entry when `None`).
pub fn load(path: &Path, board: Option<&str>) -> Result<Manifest, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    parse(&text, board).map_err(|e| format!("{}: {}", path.display(), e))
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Text(String),
    Whole(i64),
    Flag(bool),
    List(Vec<String>),
}

pub fn parse(text: &str, board: Option<&str>) -> Result<Manifest, String> {
    let mut manifest = Manifest::default();
    for (table, line, keys) in tables(text)? {
        let get = |key: &str| {
            keys.iter()
                .find(|(k, _, _)| k == key)
                .map(|(_, v, l)| (v, *l))
        };
        let text_of = |key: &str| -> Result<String, String> {
            match get(key) {
                Some((Value::Text(t), _)) => Ok(t.clone()),
                Some((_, l)) => Err(format!("line {}: {} must be a string", l, key)),
                None => Err(format!("line {}: [[{}]] needs {}", line, table, key)),
            }
        };
        let allowed: &[&str] = match table.as_str() {
            "library" => &["name", "boards"],
            "include" => &["header", "boards"],
            "object" => &["cpp", "boards"],
            "function" => &["etamil", "cpp", "args", "returns", "boards"],
            "port" => &["number", "cpp", "boards"],
            other => {
                return Err(format!(
                    "line {}: [[{}]] is not an artino.toml table",
                    line, other
                ));
            }
        };
        if let Some((key, _, l)) = keys.iter().find(|(k, _, _)| !allowed.contains(&k.as_str())) {
            return Err(format!("line {}: [[{}]] has no key {}", l, table, key));
        }
        match get("boards") {
            None => {}
            Some((Value::List(boards), _)) => {
                if let Some(board) = board
                    && !boards.iter().any(|b| b == board)
                {
                    continue;
                }
            }
            Some((_, l)) => {
                return Err(format!("line {}: boards must be a list of board names", l));
            }
        }
        match table.as_str() {
            "library" => manifest.libraries.push(text_of("name")?),
            "include" => manifest.includes.push(text_of("header")?),
            "object" => manifest.objects.push(text_of("cpp")?),
            "function" => {
                let etamil = text_of("etamil")?;
                let kinds = |key: &str, returns: bool| -> Result<Vec<Kind>, String> {
                    let words = match get(key) {
                        Some((Value::List(words), _)) => words.clone(),
                        Some((Value::Text(word), _)) if returns => vec![word.clone()],
                        Some((_, l)) => {
                            return Err(format!("line {}: {} has the wrong form", l, key));
                        }
                        None if returns => vec!["void".to_string()],
                        None => Vec::new(),
                    };
                    words
                        .iter()
                        .map(|w| {
                            Kind::parse(w, returns).ok_or_else(|| {
                                format!(
                                    "{}: {} is not {}",
                                    etamil,
                                    w,
                                    if returns {
                                        "void, int, num, bool or text"
                                    } else {
                                        "int, num, bool or text"
                                    }
                                )
                            })
                        })
                        .collect()
                };
                let args = kinds("args", false)?;
                let returns = kinds("returns", true)?[0];
                if manifest.functions.iter().any(|f| f.etamil == etamil) {
                    return Err(format!("line {}: {} is declared twice", line, etamil));
                }
                manifest.functions.push(Extern {
                    etamil,
                    cpp: text_of("cpp")?,
                    args,
                    returns,
                });
            }
            _ => {
                let number = match get("number") {
                    Some((Value::Whole(n @ 1..=3), _)) => *n as u8,
                    Some((_, l)) => {
                        return Err(format!("line {}: a port's number is 1, 2 or 3", l));
                    }
                    None => return Err(format!("line {}: [[port]] needs number", line)),
                };
                let object = text_of("cpp")?;
                if !object
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    return Err(format!(
                        "line {}: a port's cpp names its object, such as bus",
                        line
                    ));
                }
                if manifest.ports.iter().any(|(n, _)| *n == number) {
                    return Err(format!("line {}: port {} is declared twice", line, number));
                }
                manifest.ports.push((number, object));
            }
        }
    }
    Ok(manifest)
}

type Table = (String, usize, Vec<(String, Value, usize)>);

/// `[[name]]` tables and their `key = value` lines.
fn tables(text: &str) -> Result<Vec<Table>, String> {
    let mut out: Vec<Table> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let content = strip_comment(raw).trim();
        if content.is_empty() {
            continue;
        }
        if let Some(name) = content
            .strip_prefix("[[")
            .and_then(|r| r.strip_suffix("]]"))
        {
            out.push((name.trim().to_string(), line, Vec::new()));
            continue;
        }
        if content.starts_with('[') {
            return Err(format!(
                "line {}: artino.toml has only [[tables]]: {}",
                line, content
            ));
        }
        let (key, value) = content
            .split_once('=')
            .ok_or_else(|| format!("line {}: expected key = value: {}", line, content))?;
        let Some(current) = out.last_mut() else {
            return Err(format!(
                "line {}: {} is outside any [[table]]",
                line,
                key.trim()
            ));
        };
        current
            .2
            .push((key.trim().to_string(), value_of(value.trim(), line)?, line));
    }
    Ok(out)
}

/// The line without a `#` comment, a `#` inside a string kept.
fn strip_comment(line: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (at, ch) in line.char_indices() {
        match quote {
            Some(q) => {
                if escaped {
                    escaped = false;
                } else if ch == '\\' && q == '"' {
                    escaped = true;
                } else if ch == q {
                    quote = None;
                }
            }
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch == '#' => return &line[..at],
            None => {}
        }
    }
    line
}

fn value_of(text: &str, line: usize) -> Result<Value, String> {
    if let Some(inner) = text.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        let mut items = Vec::new();
        let mut rest = inner.trim();
        while !rest.is_empty() {
            let (item, after) = string_at(rest, line)?;
            items.push(item);
            rest = after.trim_start();
            rest = rest.strip_prefix(',').unwrap_or(rest).trim_start();
        }
        return Ok(Value::List(items));
    }
    if text.starts_with('"') || text.starts_with('\'') {
        let (item, after) = string_at(text, line)?;
        if !after.trim().is_empty() {
            return Err(format!(
                "line {}: something after the string: {}",
                line,
                after.trim()
            ));
        }
        return Ok(Value::Text(item));
    }
    match text {
        "true" => return Ok(Value::Flag(true)),
        "false" => return Ok(Value::Flag(false)),
        _ => {}
    }
    text.parse::<i64>().map(Value::Whole).map_err(|_| {
        format!(
            "line {}: {} is not a string, whole number, boolean or list of strings",
            line, text
        )
    })
}

/// A string at the start of `text`, and what follows it.
fn string_at(text: &str, line: usize) -> Result<(String, &str), String> {
    let mut chars = text.char_indices();
    let quote = match chars.next() {
        Some((_, q @ ('"' | '\''))) => q,
        _ => return Err(format!("line {}: expected a string: {}", line, text)),
    };
    let mut out = String::new();
    let mut escaped = false;
    for (at, ch) in chars {
        if escaped {
            out.push(match ch {
                'n' => '\n',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if ch == '\\' && quote == '"' {
            escaped = true;
        } else if ch == quote {
            return Ok((out, &text[at + 1..]));
        } else {
            out.push(ch);
        }
    }
    Err(format!("line {}: a string is not closed", line))
}

// --- the shims ---------------------------------------------------------------------

/// `artino_shims.cpp`: the includes, the objects, one `extern "C"` function
/// for each `[[function]]`, and the extra serial ports.
pub fn shims(manifest: &Manifest, source_name: &str) -> String {
    let mut out = format!(
        "// Generated by etamil --artino from {}'s manifest. Each function is one\n\
         // [[function]] of it, called by the compiled program by number.\n\
         #include <Arduino.h>\n",
        source_name
    );
    for header in &manifest.includes {
        out.push_str(&format!("#include <{}>\n", header));
    }
    out.push_str("#include \"artino.h\"\n\n");
    for object in &manifest.objects {
        out.push_str(object);
        out.push('\n');
    }
    out.push_str(
        "\n// Text C++ gives back, as const char * or String, copied into the program's buffer.\n\
         __attribute__((unused)) static void artino_give(char *out, const char *text) { artino_text_set(out, text); }\n\
         __attribute__((unused)) static void artino_give(char *out, const String &text) { artino_text_set(out, text.c_str()); }\n\n\
         // A double from C++ as a board number: x 1000, half away from zero, held\n\
         // at the limit. avr-libc has no llround.\n\
         __attribute__((unused)) static int64_t artino_num_of(double value) {\n\
         \x20 double scaled = value * 1000.0;\n\
         \x20 if (scaled >= 9.2e18) return INT64_MAX;\n\
         \x20 if (scaled <= -9.2e18) return -INT64_MAX;\n\
         \x20 return (int64_t)(scaled < 0 ? scaled - 0.5 : scaled + 0.5);\n\
         }\n\n\
         extern \"C\" {\n",
    );
    for (index, function) in manifest.functions.iter().enumerate() {
        out.push_str(&shim(index, function));
    }
    if !manifest.ports.is_empty() {
        out.push_str("\nStream *artino_extra_port(int32_t port) {\n  switch (port) {\n");
        for (number, object) in &manifest.ports {
            out.push_str(&format!(
                "    case {}:\n      return &{};\n",
                number, object
            ));
        }
        out.push_str("    default:\n      return nullptr;\n  }\n}\n\n");
        out.push_str("void artino_extra_begin(int32_t port, int32_t baud) {\n  switch (port) {\n");
        for (number, object) in &manifest.ports {
            out.push_str(&format!(
                "    case {}:\n      {}.begin(baud);\n      break;\n",
                number, object
            ));
        }
        out.push_str("    default:\n      break;\n  }\n}\n");
    }
    out.push_str("}  // extern \"C\"\n");
    out
}

fn shim(index: usize, function: &Extern) -> String {
    // Arguments as C++ wants them: a number is ×1000 on the board.
    let values: Vec<String> = function
        .args
        .iter()
        .enumerate()
        .map(|(i, kind)| match kind {
            Kind::Num => format!("(double)a{} / 1000.0", i),
            Kind::Bool => format!("a{} != 0", i),
            _ => format!("a{}", i),
        })
        .collect();
    let call =
        if function.cpp.contains("{0}") || function.args.is_empty() && function.cpp.contains('(') {
            let mut call = function.cpp.clone();
            for (i, value) in values.iter().enumerate() {
                call = call.replace(&format!("{{{}}}", i), &format!("({})", value));
            }
            call
        } else {
            format!("{}({})", function.cpp, values.join(", "))
        };
    let mut params: Vec<String> = Vec::new();
    if function.returns == Kind::Text {
        params.push("char *out".to_string());
    }
    for (i, kind) in function.args.iter().enumerate() {
        params.push(format!("{} a{}", kind.cpp(), i));
    }
    let returns = if function.returns == Kind::Text {
        "void"
    } else {
        function.returns.cpp()
    };
    let body = match function.returns {
        Kind::Void => format!("{};", call),
        Kind::Int => format!("return (int32_t)({});", call),
        Kind::Bool => format!("return ({}) ? 1 : 0;", call),
        Kind::Num => format!("return artino_num_of((double)({}));", call),
        Kind::Text => format!("artino_give(out, {});", call),
    };
    format!(
        "// {}\n{} {}({}) {{ {} }}\n",
        function.etamil,
        returns,
        Extern::symbol(index),
        if params.is_empty() {
            "void".to_string()
        } else {
            params.join(", ")
        },
        body
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVO: &str = r#"
# a door on a servo
[[library]]
name = "Servo"

[[include]]
header = "Servo.h"

[[object]]
cpp = 'Servo kaqavu;'

[[function]]
etamil = "கதவு_இணை"
cpp = "kaqavu.attach"
args = ["int"]

[[function]]
etamil = "கோணம்"
cpp = "kaqavu.read"
returns = "int"

[[port]]
number = 1
cpp = "bus"
boards = ["uno", "nano"]
"#;

    #[test]
    fn a_manifest_is_read_for_its_board() {
        let uno = parse(SERVO, Some("uno")).expect("parses");
        assert_eq!(uno.libraries, vec!["Servo"]);
        assert_eq!(uno.objects, vec!["Servo kaqavu;"]);
        assert_eq!(uno.functions[0].args, vec![Kind::Int]);
        assert_eq!(uno.functions[0].returns, Kind::Void);
        assert_eq!(uno.functions[1].returns, Kind::Int);
        assert_eq!(uno.ports, vec![(1, "bus".to_string())]);
        // Not a Pico's: the port is for the Uno and Nano only.
        assert!(parse(SERVO, Some("pico")).expect("parses").ports.is_empty());
        assert_eq!(parse(SERVO, None).expect("parses").ports.len(), 1);
    }

    #[test]
    fn mistakes_name_their_line() {
        let wrong = |text: &str| parse(text, None).expect_err("refused");
        assert!(
            wrong("[[function]]\netamil = \"அ\"\ncpp = \"f\"\nargs = [\"float\"]")
                .contains("float")
        );
        assert!(wrong("[[servo]]\nname = \"x\"").contains("line 1"));
        assert!(wrong("[[port]]\nnumber = 4\ncpp = \"bus\"").contains("line 2"));
        assert!(wrong("name = \"x\"").contains("outside"));
        assert!(wrong("[[library]]\nname = \"x\"\nversion = 2").contains("no key version"));
    }

    #[test]
    fn shims_convert_at_the_boundary() {
        let manifest = Manifest {
            functions: vec![
                Extern {
                    etamil: "அ".into(),
                    cpp: "servo.write".into(),
                    args: vec![Kind::Int],
                    returns: Kind::Void,
                },
                Extern {
                    etamil: "ஆ".into(),
                    cpp: "sensors.getTempCByIndex".into(),
                    args: vec![Kind::Int],
                    returns: Kind::Num,
                },
                Extern {
                    etamil: "இ".into(),
                    cpp: "strip.setPixelColor({0}, strip.Color({1}, {2}, {3}))".into(),
                    args: vec![Kind::Int; 4],
                    returns: Kind::Void,
                },
            ],
            ..Manifest::default()
        };
        let text = shims(&manifest, "p.qmz");
        assert!(
            text.contains("void artino_x_0(int32_t a0) { servo.write(a0); }"),
            "{}",
            text
        );
        assert!(text.contains("int64_t artino_x_1(int32_t a0) { return artino_num_of((double)(sensors.getTempCByIndex(a0))); }"), "{}", text);
        assert!(
            text.contains("strip.setPixelColor((a0), strip.Color((a1), (a2), (a3)))"),
            "{}",
            text
        );
    }
}
