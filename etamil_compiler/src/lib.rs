// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! The eTamil compiler.
//!
//! Pipeline: `lexer` -> `parser` -> `vm::bytecode::compiler` -> `vm::interpreter`.
//! `codegen` is an optional LLVM backend replacing the last two stages,
//! enabled with `--features llvm`.
//! `artino` compiles the same checked AST for Arduino boards (docs/artino.md);
//! its analysis runs anywhere, its LLVM lowering needs the same feature.
//!
//! # wasm builds
//!
//! `lexer`, `parser` and `check` depend on nothing but `logos`, `rust_decimal`
//! and `std` collections, so they compile for `wasm32-unknown-unknown` as they
//! are. **So does `vm`**, which is why the editor on etamil.in runs a program
//! rather than only checking one: it is the real front end and the real
//! bytecode interpreter, not a reimplementation of either.
//!
//! What is gated out is what needs an operating system -- a filesystem, a
//! socket, a terminal: `module`, `repl`, `db`, `redis`, `http`, `codegen`, `artino` and
//! the feature-gated drivers. A statement that reaches one of them refuses
//! through `wasm_stubs` with a message naming what is not available in the
//! browser, rather than quietly doing nothing. Several modules below the front end are portable and are
//! left ungated -- `crypt`, `signing`, `runtime`, `net`, `stdlib` -- and each
//! says so where it is declared.
//!
//! `wasm.rs` is the whole of the surface a browser sees. Its module comment
//! lists the exports.
//!
//! Build it with:
//!
//! ```text
//! cargo build --release --target wasm32-unknown-unknown --no-default-features
//! ```

// --- Front end: portable, and the whole of what a browser build exposes ---
pub mod analysis;
pub mod check;
pub mod lexer;
pub mod parser;

// --- Portable, but only a native build has a use for it ---

// The standard library, compiled into the binary by build.rs. This is what
// makes `cargo install` — and every other package manager, which places an
// executable and nothing else — produce a compiler whose first `இறக்கு`
// resolves rather than failing.
//
// It needs no operating system, so it is not gated — but nor is it of any use
// to a wasm build, and it once carried a comment claiming the opposite: that
// wasm needed it most, a browser having no directory to look in. `module`
// below holds every call site of this module and *is* gated out of wasm, so a
// wasm build has no `locate` to consult the embedded copy with and cannot
// resolve an import by either route. Nothing there reaches this module, only
// `#[wasm_bindgen]` exports are linker roots, and the whole of it —
// `include_str!` data included — is dropped: gating it by hand moved the wasm
// by 148 bytes, which is noise. So the cfg stays off, because adding one would
// buy nothing and would have to be kept in step with `module`'s.
//
// If browser imports are ever wanted, `module` is what has to reach wasm
// first; this is then what it would read.
pub mod stdlib;

// --- The rest. Gated out of a wasm build unless its own note says otherwise:
//     several of these are portable and a header claiming they all need an OS
//     was read, reasonably, as saying they do not reach the browser ---
// Reads imported files from disk.
#[cfg(not(target_family = "wasm"))]
pub mod module;
// Reads stdin, writes stdout.
#[cfg(not(target_family = "wasm"))]
pub mod repl;
// Authenticated encryption behind மறை and வெளிப்படு. Portable: XChaCha20-
// Poly1305 and Argon2id are pure Rust, and getrandom already has its wasm_js
// feature enabled below, so this works in the browser build too.
pub mod crypt;
// A command and a reply, so the host offers exactly one command generically
// and every Redis command works through it. Needs a socket.
#[cfg(not(target_family = "wasm"))]
pub mod redis;
// Portable: HMAC-SHA256 over hmac/sha2/subtle, all pure Rust.
pub mod signing;
// What the LLVM backend's emitted IR calls into: every eTamil value as a handle
// into an arena, and every operation on one as a C-ABI call. Not behind the
// `llvm` feature and not behind `cfg(not(wasm))` — it links no LLVM and touches
// no OS beyond stdout, and the `cdylib` has to export it on any machine that
// might link an `output.ll`.
pub mod runtime;
// Documents, behind a feature like the other non-bundled drivers.
#[cfg(all(feature = "mongodb", not(target_family = "wasm")))]
pub mod mongo;
// Client certificates are only meaningful when there is a client.
#[cfg(all(feature = "http-client", not(target_family = "wasm")))]
pub mod mtls;
// SQLite, PostgreSQL and MySQL behind தளம்_இணை. One connection at a time, and
// each driver needs a socket or a file.
#[cfg(not(target_family = "wasm"))]
pub mod db;
// Portable with `http-client` off, which a wasm build implies: `request` then
// takes the existing no-HTTP-client fallback, and sign/verify are pure Rust.
pub mod net;
// Internally `#[cfg(feature = "llvm")]` throughout, and llvm-sys has no wasm
// build; gated here too so a wasm build does not depend on that staying true.
#[cfg(not(target_family = "wasm"))]
pub mod codegen;
// eTamil for Arduino boards. The analysis and the sketch writer are portable
// Rust and the emitter is behind `llvm`, but it names statements with
// codegen's labels and writes files, so it leaves a wasm build with codegen.
#[cfg(not(target_family = "wasm"))]
pub mod artino;
// The server behind வழி and the routing around it. Needs to bind a socket.
#[cfg(not(target_family = "wasm"))]
pub mod http;
// Portable: a walk over compiled code that decides which functions a database
// may be given to call. It touches nothing outside the program it is given.
pub mod purity;
// Portable: the bytecode compiler and value layer touch no OS at all, and the
// interpreter's input and output go through vm::host, which has a browser
// implementation. The archive and subprocess helpers inside it are gated
// individually.
pub mod vm;

// --- Browser bindings ---

// Stand-ins for db, redis and http, re-exported under those names so the
// interpreter's `crate::db::...` paths resolve unchanged. See wasm_stubs.rs for
// why this is done as substitution rather than as gating.
#[cfg(target_family = "wasm")]
mod wasm_stubs;
#[cfg(target_family = "wasm")]
pub use wasm_stubs::{db, http, redis};

#[cfg(target_family = "wasm")]
pub mod wasm;
