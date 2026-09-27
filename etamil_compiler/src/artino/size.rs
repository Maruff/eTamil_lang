// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! What a linked AVR image needs of the board's RAM: its variables, and the
//! most the stack can reach.
//!
//! arduino-cli prints the variables and calls the rest "left for local
//! variables". On an Uno that rest is the stack, and when the stack meets the
//! variables nothing says so: a number changes, a text goes wrong, the board
//! restarts. So after every AVR build this reads the image back:
//!
//! - variables: the `.data` and `.bss` sections;
//! - stack: along the deepest chain of calls from `main`, each function's
//!   pushes and frame and two bytes a call, plus the deepest interrupt
//!   handler, which can arrive at that moment.
//!
//! Calls through a pointer (a Stream's virtual write) are not followed; they
//! are the core's, and shallow. The figure is the chain the code can take, not
//! one a test saw, so it is an upper bound on every path but those.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Default)]
struct Function {
    own: u32,
    calls: Vec<String>,
}

/// The deepest stack from `main`, plus the deepest interrupt, and the chain.
pub fn stack(disassembly: &str) -> (u32, Vec<String>) {
    let mut functions: HashMap<String, Function> = HashMap::new();
    let mut current: Option<String> = None;
    let mut lines_in = 0;
    let mut frame: u32 = 0;
    let finish = |functions: &mut HashMap<String, Function>, name: &Option<String>, frame: u32| {
        if let Some(name) = name {
            // A negative adjustment is an epilogue giving a frame back.
            let frame = if frame >= 0x8000 { 0 } else { frame };
            functions.entry(name.clone()).or_default().own += frame;
        }
    };
    for line in disassembly.lines() {
        if let Some(name) = line.strip_suffix(">:").and_then(|l| l.split_once(" <")).map(|(_, n)| n.to_string()) {
            finish(&mut functions, &current, frame);
            functions.entry(name.clone()).or_default();
            current = Some(name);
            lines_in = 0;
            frame = 0;
            continue;
        }
        let Some(name) = &current else { continue };
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        lines_in += 1;
        let op = parts[2].trim();
        let args = parts.get(3).map(|a| a.trim()).unwrap_or("");
        let f = functions.entry(name.clone()).or_default();
        match op {
            "push" => f.own += 1,
            "call" | "rcall" => {
                if let Some(target) = line.rsplit_once('<').and_then(|(_, t)| t.split(['>', '+']).next()) {
                    f.calls.push(target.to_string());
                }
            }
            _ => {}
        }
        if lines_in < 60 {
            let immediate = |prefix: &str| {
                args.strip_prefix(prefix)
                    .and_then(|v| v.split(|c: char| c.is_whitespace() || c == ';').next())
                    .and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok())
            };
            match op {
                "subi" if frame == 0 => frame = immediate("r28, ").unwrap_or(0),
                "sbci" if frame < 0x100 => frame += immediate("r29, ").unwrap_or(0) << 8,
                "sbiw" if frame == 0 => frame = immediate("r28, ").unwrap_or(0),
                _ => {}
            }
        }
    }
    finish(&mut functions, &current, frame);

    fn deepest(name: &str, functions: &HashMap<String, Function>, seen: &mut Vec<String>, memo: &mut HashMap<String, (u32, Vec<String>)>) -> (u32, Vec<String>) {
        if let Some(found) = memo.get(name) {
            return found.clone();
        }
        let Some(f) = functions.get(name) else { return (0, Vec::new()) };
        if seen.iter().any(|s| s == name) {
            return (0, vec![format!("{} (recursion)", name)]);
        }
        seen.push(name.to_string());
        let mut best = (0, Vec::new());
        for call in &f.calls {
            let (depth, chain) = deepest(call, functions, seen, memo);
            if depth + 2 > best.0 {
                best = (depth + 2, chain);
            }
        }
        seen.pop();
        let mut chain = vec![name.to_string()];
        chain.extend(best.1);
        let result = (f.own + best.0, chain);
        memo.insert(name.to_string(), result.clone());
        result
    }
    let mut memo = HashMap::new();
    let (main, chain) = deepest("main", &functions, &mut Vec::new(), &mut memo);
    let interrupt = functions
        .keys()
        .filter(|n| n.starts_with("__vector_"))
        .map(|n| deepest(n, &functions, &mut Vec::new(), &mut memo).0 + 2)
        .max()
        .unwrap_or(0);
    (main + interrupt, chain)
}

/// `.data` + `.bss`, from `objdump -h`.
pub fn variables(headers: &str) -> u32 {
    headers
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            match fields.as_slice() {
                [_, ".data" | ".bss", size, ..] => u32::from_str_radix(size, 16).ok(),
                _ => None,
            }
        })
        .sum()
}

/// avr-objdump, from the toolchain arduino-cli installed with the AVR core.
fn objdump(cli: &str) -> Option<PathBuf> {
    let output = Command::new(cli).args(["config", "get", "directories.data"]).output().ok()?;
    let data = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    let tools = data.join("packages").join("arduino").join("tools").join("avr-gcc");
    let mut versions: Vec<PathBuf> = std::fs::read_dir(tools).ok()?.flatten().map(|e| e.path()).collect();
    versions.sort();
    let name = if cfg!(windows) { "avr-objdump.exe" } else { "avr-objdump" };
    versions.into_iter().rev().map(|v| v.join("bin").join(name)).find(|p| p.is_file())
}

/// The RAM line for a linked AVR image, and whether it fits.
pub fn report(cli: &str, elf: &Path, ram: u32) -> Option<(String, bool)> {
    let tool = objdump(cli)?;
    let run = |flag: &str| {
        Command::new(&tool).arg(flag).arg(elf).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
    };
    let vars = variables(&run("-h")?);
    let (stack, chain) = stack(&run("-d")?);
    let total = vars + stack;
    let mut line = format!(
        "RAM: {} B of variables + {} B of stack at most = {} of {} B",
        vars, stack, total, ram
    );
    let fits = total <= ram;
    if fits {
        line.push_str(&format!(" ({} B to spare)", ram - total));
    } else {
        line.push_str(&format!(
            ". The stack can reach the variables, by {} B, along {}",
            total - ram,
            chain.join(" → ")
        ));
    }
    Some((line, fits))
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMAGE: &str = "
00000100 <main>:
 100:\t0e 94 00 02 \tcall\t0x400\t; 0x400 <artino_loop>
00000400 <artino_loop>:
 400:\tcf 93       \tpush\tr28
 402:\tdf 93       \tpush\tr29
 404:\tc5 57       \tsubi\tr28, 0x75\t; 117
 406:\td3 40       \tsbci\tr29, 0x01\t; 1
 408:\t0e 94 00 03 \tcall\t0x600\t; 0x600 <helper>
 40c:\tc5 58       \tsubi\tr28, 0x85\t; 133
 40e:\tdc 4f       \tsbci\tr29, 0xFE\t; 254
00000600 <helper>:
 600:\t0f 93       \tpush\tr16
 602:\t28 97       \tsbiw\tr28, 0x08\t; 8
00000800 <__vector_7>:
 800:\t1f 92       \tpush\tr1
 802:\t0f 92       \tpush\tr0
";

    #[test]
    fn the_deepest_chain_and_an_interrupt() {
        let (depth, chain) = stack(IMAGE);
        // loop 2 + 0x175, helper 1 + 8, two calls, an interrupt of 2 and its call.
        assert_eq!(depth, (2 + 0x175) + (1 + 8) + 2 + 2 + (2 + 2));
        assert_eq!(chain, vec!["main", "artino_loop", "helper"]);
    }

    #[test]
    fn variables_are_data_and_bss() {
        let headers = "  0 .data         0000017a  00800100  00007a4c  00007ae0  2**0\n  1 .text 00007a4c 00000000\n  2 .bss          000002f4  0080027a  0080027a  00007c5a  2**0\n";
        assert_eq!(variables(headers), 0x17a + 0x2f4);
    }
}
