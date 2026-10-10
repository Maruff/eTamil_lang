// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! `etamil-lsp`: speaks the Language Server Protocol on stdin and stdout.

use lsp_server::Connection;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Anything printed to stdout would corrupt the protocol stream, so
    // messages about the server itself go to stderr.
    eprintln!("etamil-lsp {} starting", env!("CARGO_PKG_VERSION"));
    let (connection, io_threads) = Connection::stdio();
    connection.initialize(serde_json::to_value(etamil_lsp::capabilities())?)?;
    etamil_lsp::run(&connection)?;
    io_threads.join()?;
    eprintln!("etamil-lsp stopped");
    Ok(())
}
