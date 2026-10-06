// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Runs the real `etamil-lsp` binary over its standard streams, the way an editor does.
//! The in-process tests in server.rs cannot see how the process ends.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn send(stdin: &mut impl Write, message: &str) {
    write!(stdin, "Content-Length: {}\r\n\r\n{}", message.len(), message).unwrap();
    stdin.flush().unwrap();
}

fn receive(stdout: &mut BufReader<impl Read>) -> String {
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse().unwrap();
        }
    }
    let mut body = vec![0u8; length];
    stdout.read_exact(&mut body).unwrap();
    String::from_utf8(body).unwrap()
}

#[test]
fn the_process_ends_after_exit_even_though_the_client_keeps_stdin_open() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_etamil-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the server starts");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    send(&mut stdin, r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"capabilities":{},"rootUri":null}}"#);
    assert!(receive(&mut stdout).contains("\"capabilities\""));
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#);

    send(&mut stdin, r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}"#);
    assert!(receive(&mut stdout).contains("\"id\":2"));
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit","params":null}"#);

    // stdin stays open here, as it does for a client that has not yet torn its pipes
    // down: the server must still stop on its own.
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("the server was still running 10 seconds after exit");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(status.success(), "a clean shutdown exits with 0, got {status}");
}

#[test]
fn the_process_ends_when_the_client_closes_stdin_without_asking() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_etamil-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the server starts");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    send(&mut stdin, r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"capabilities":{},"rootUri":null}}"#);
    receive(&mut stdout);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#);
    drop(stdin); // an editor that crashed

    let deadline = Instant::now() + Duration::from_secs(10);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("the server outlived its client");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
