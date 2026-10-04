// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! `etamil-lsp`: speaks the Language Server Protocol on stdin and stdout.

use lsp_server::Connection;

fn main() {
    // Anything printed to stdout would corrupt the protocol stream, so messages about
    // the server itself go to stderr.
    eprintln!("etamil-lsp {} starting", env!("CARGO_PKG_VERSION"));
    let (connection, _io_threads) = Connection::stdio();

    let result = connection
        .initialize(serde_json::to_value(etamil_lsp::capabilities()).expect("capabilities serialize"))
        .map_err(Into::into)
        .and_then(|_| etamil_lsp::run(&connection));
    drop(connection);

    // The protocol ends with the client's `exit` notification, after the shutdown reply
    // has been written, so there is nothing left to flush. The reader thread, though,
    // is blocked on stdin until the client closes it, and waiting for it (as joining the
    // io threads does) would keep this process alive after `exit` for as long as the
    // client holds the pipe open.
    match result {
        Ok(()) => {
            eprintln!("etamil-lsp stopped");
            std::process::exit(0);
        }
        Err(error) => {
            eprintln!("etamil-lsp: {error}");
            std::process::exit(1);
        }
    }
}
