// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! artino — eTamil compiled for Arduino boards, through LLVM.
//!
//! ```text
//! etamil --artino --board uno prog.qmz          # a sketch folder, ready for arduino-cli
//! etamil --artino-gaps prog.qmz                 # what stops it building, on any machine
//! ```
//!
//! The pipeline is the desktop one up to the checked AST; after that artino
//! has its own lowering (`emit`), because the desktop backend holds every value
//! as a handle into the VM's runtime and a board has no room for that runtime.
//! Here a number is an `i64` holding the value × 1000 and a boolean is an `i1`,
//! decided when the program is compiled (`analyse`). See docs/artino.md.
//!
//! What the board runs is written out by `sketch`: the program as a
//! precompiled Arduino library, plus the MIT runtime (`artino/artino_rt.cpp`)
//! and a three-line `.ino`, so `arduino-cli` links and uploads it with no
//! custom build rules — the route the B0 spike proved on Uno, Nano, Mega and
//! Pico.

pub mod analyse;
#[cfg(feature = "llvm")]
pub mod emit;
pub mod manifest;
pub mod size;
pub mod sketch;
pub mod source;

/// A board artino builds for.
pub struct Board {
    pub name: &'static str,
    /// LLVM target triple and CPU.
    pub triple: &'static str,
    pub cpu: &'static str,
    /// What arduino-cli calls it.
    pub fqbn: &'static str,
    /// The core's `build.mcu`: the folder a precompiled library is looked for in.
    pub mcu: &'static str,
    /// A report names its line and number, not the operation's text: the
    /// text of every site would take a quarter of an Uno's flash. The full
    /// text of each is in the sketch's `artino_sites.txt`.
    pub line_reports: bool,
    /// Serial ports 0 up to this: the USB port and the board's hardware ones.
    pub serial_ports: u8,
    /// `நிலை` texts and arrays are kept in flash and copied out to be read,
    /// as AVR needs: its flash is not RAM.
    pub flash_constants: bool,
    /// Bytes of RAM, for the size report; 0 where there is nothing to report.
    pub ram: u32,
}

pub const BOARDS: &[Board] = &[
    Board { name: "uno", triple: "avr", cpu: "atmega328p", fqbn: "arduino:avr:uno", mcu: "atmega328p", line_reports: true, serial_ports: 1, flash_constants: true, ram: 2048 },
    Board { name: "nano", triple: "avr", cpu: "atmega328p", fqbn: "arduino:avr:nano", mcu: "atmega328p", line_reports: true, serial_ports: 1, flash_constants: true, ram: 2048 },
    Board { name: "mega", triple: "avr", cpu: "atmega2560", fqbn: "arduino:avr:mega", mcu: "atmega2560", line_reports: false, serial_ports: 4, flash_constants: true, ram: 8192 },
    Board {
        name: "pico",
        triple: "thumbv6m-none-eabi",
        cpu: "cortex-m0plus",
        fqbn: "rp2040:rp2040:rpipico",
        mcu: "cortex-m0plus",
        line_reports: false,
        serial_ports: 3,
        flash_constants: false,
        ram: 264 * 1024,
    },
    // Not a board: this machine. For the conformance suite, which runs the
    // static lowering against a stub of the Arduino API and compares its
    // output with the VM's. An empty triple means LLVM's default.
    Board {
        name: "host",
        triple: "",
        cpu: "generic",
        fqbn: "",
        mcu: "",
        line_reports: false,
        serial_ports: 1,
        flash_constants: false,
        ram: 0,
    },
    // This machine again, compiled as for an Uno: constants read as from
    // flash, reports by line. The conformance suite runs it too, so what only
    // a small board does is still checked against the VM.
    Board {
        name: "host-small",
        triple: "",
        cpu: "generic",
        fqbn: "",
        mcu: "",
        line_reports: true,
        serial_ports: 1,
        flash_constants: true,
        ram: 0,
    },
    Board {
        name: "pico2",
        triple: "thumbv8m.main-none-eabi",
        cpu: "cortex-m33",
        fqbn: "rp2040:rp2040:rpipico2",
        mcu: "cortex-m33",
        line_reports: false,
        serial_ports: 3,
        flash_constants: false,
        ram: 520 * 1024,
    },
];

pub fn board(name: &str) -> Option<&'static Board> {
    BOARDS.iter().find(|b| b.name == name)
}

/// What a `nUlakam/vaZporuL/` board file binds to the boards it describes:
/// `நிலை சொல் பலகைக்_கோப்பு = "pico,pico2";`.
pub const BOARD_FILE_MARK: &str = "பலகைக்_கோப்பு";

/// A board file imported for another board, named. yUnO.qmz built for a Pico
/// would drive the Uno's pin numbers, so it is refused rather than built. The
/// host stands in for every board, so it takes any board file.
pub fn board_file_mismatch(ast: &[crate::parser::Stmt], board: &str) -> Option<String> {
    use crate::parser::{Expr, Stmt};
    if board.starts_with("host") {
        return None;
    }
    ast.iter().find_map(|statement| match statement {
        Stmt::Assign { name, value: Expr::String(boards), .. }
            if name == BOARD_FILE_MARK && !boards.split(',').any(|b| b.trim() == board) =>
        {
            Some(format!(
                "the program imports the board file for {}, and is being built for {}: its pin names are the other board's. Import {}'s board file from nUlakam/vaZporuL/, or build with --board {}",
                boards,
                board,
                board,
                boards.split(',').next().unwrap_or(boards)
            ))
        }
        _ => None,
    })
}

/// The runtime written beside every sketch. MIT, unlike the compiler.
pub const RUNTIME_HEADER: &str = include_str!("../../artino/artino.h");
pub const RUNTIME_SOURCE: &str = include_str!("../../artino/artino_rt.cpp");

#[cfg(test)]
mod tests {
    use super::board_file_mismatch;

    fn load(source: &str) -> Vec<crate::parser::Stmt> {
        crate::module::load_source(source, std::path::Path::new(".")).expect("parses")
    }

    #[test]
    fn a_board_file_builds_only_for_its_boards() {
        let pico = load("இறக்கு \"nUlakam/vaZporuL/pIkO.qmz\";");
        assert!(board_file_mismatch(&pico, "pico").is_none());
        assert!(board_file_mismatch(&pico, "pico2").is_none());
        assert!(board_file_mismatch(&pico, "host").is_none());
        let refused = board_file_mismatch(&pico, "uno").expect("refused");
        assert!(refused.contains("pico,pico2") && refused.contains("uno"), "{}", refused);
        assert!(board_file_mismatch(&load("இறக்கு \"nUlakam/vaZporuL/rAspY.qmz\";"), "mega").is_some());
        assert!(board_file_mismatch(&load("இறக்கு \"nUlakam/vaZporuL/vaZporuL.qmz\";"), "uno").is_none());
    }
}
