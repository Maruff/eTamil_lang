// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
// The modules live in the library target (src/lib.rs); this binary uses them
// from there rather than re-declaring them, which previously compiled the
// whole crate twice.
use std::env;
use std::io::{self, Read};
use std::path::Path;

#[cfg(feature = "llvm")]
use etamil_compiler::codegen;
use etamil_compiler::http::{AsyncHttpServer, HttpServer};
use etamil_compiler::{module, parser, vm};

// main is deliberately *not* async.
//
// It used to be #[tokio::main], on the grounds that the server paths needed a
// runtime — but nothing here ever did: the VM is synchronous and the HTTP
// server is thread-per-request over std::net. The only tokio users in the
// crate are http/cache.rs and http/resilience.rs, neither of which is
// reachable from this binary, and whose tests carry their own runtime.
//
// It was not harmless. Being inside a runtime makes every blocking database
// driver panic — `Cannot start a runtime from within a runtime` — because
// they call block_on internally. That is exactly the arrangement
// docs/ARCHITECTURE.md chose blocking drivers *for*.
//
// Note on numbering: "backend milestone N" here refers to this repository's
// own HTTP work (1 sync server, 2 async, 3 logging, 4 auth). It is unrelated
// to the Phase 1-5 in the eTamil paper, which numbers compiler core, domain
// modules, tooling, pilots and policy. See docs/ROADMAP.md.

fn print_help() {
    println!("etamil {} — the eTamil compiler", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE:");
    println!("    etamil [OPTIONS] <FILE>");
    println!("    cat program.qmz | etamil [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --vm               Run on the bytecode VM (default)");
    println!("    --check            Lex, parse and type check only — never runs the program");
    println!("    --repl             Interactive shell: type an expression, see what it comes to");
    println!("    --server           Start the synchronous HTTP server");
    println!("    --async            Concurrent server: async accept, blocking handlers");
    println!("                       இடைவெளி blocks run on a timer under either server");
    println!("    --llvm             LLVM backend (requires --features llvm; Linux/macOS)");
    println!("    --llvm-gaps        List what the LLVM backend would refuse — no LLVM needed");
    println!("    --artino           Compile for an Arduino board (requires --features llvm)");
    println!("    --board <BOARD>    With --artino: uno (default), nano, mega, pico, pico2;");
    println!("                       host builds an object for this machine, for testing");
    println!("    --out <DIR>        With --artino: where the sketch goes (default: artino-build)");
    println!("    --upload <PORT>    With --artino: build and upload through arduino-cli");
    println!("    --artino-gaps      List what --artino would refuse — no LLVM needed");
    println!("    --host <HOST>      Server bind address (default: 127.0.0.1)");
    println!("    --port <PORT>      Server port (default: 8080)");
    println!("    -h, --help         Show this message");
    println!("    -V, --version      Show the version");
    println!();
    println!("EXAMPLES:");
    println!("    etamil --vm program.qmz");
    println!("    echo \"950000\" | etamil --vm examples/basic_samples/example.qmz");
    println!("    etamil --server --port 8080 examples/backend/hello_server.qmz");
    println!("    cat program.qmz | etamil --check     # errors only, nothing runs");
    println!("    etamil --repl                        # try something without a file");
    println!("    etamil --llvm-gaps nUlakam/kAcu.qmz  # what stops --llvm compiling it");
    println!("    etamil --artino --board uno blink.qmz  # a sketch folder for arduino-cli");
}

/// `--check`: report every error the front end can find, and run nothing.
///
/// This exists for the editor. An editor has to be able to tell an author
/// about a mistake while they are still typing, and the only way to do that
/// without a second parser to keep in step is to ask this one — but a mode
/// that also *executes* the file would mean opening a program in a text editor
/// wrote its files and issued its queries. So the pipeline stops after the
/// checker.
///
/// Output is errors only, on stderr, one per line, in the same positioned
/// bilingual form as everywhere else. Nothing is written to stdout, so a
/// caller can treat any output at all as failure.
fn check_only(loaded: Result<Vec<parser::Stmt>, String>) -> ! {
    let ast = match loaded {
        Ok(ast) => ast,
        Err(message) => {
            // Lexical, parse and module-resolution failures arrive already
            // formatted, and may be several joined by newlines.
            eprintln!("✗ {}", message);
            std::process::exit(1);
        }
    };

    match etamil_compiler::check::check(&ast) {
        Ok(()) => std::process::exit(0),
        Err(errors) => {
            for error in &errors {
                eprintln!("✗ {}", error);
            }
            std::process::exit(1);
        }
    }
}

/// Print what the LLVM backend would refuse in this program, then exit.
///
/// `scripts/run_parity.sh` answers the same question by compiling and running
/// every program, which needs LLVM 18 and clang. This needs neither: the list
/// of unbuildable constructs is a pure function of the AST, so the gap can be
/// counted — and ranked, and watched shrinking — on the machine the code is
/// written on rather than only on the machine it is compiled on.
///
/// Exit 0 when nothing is refused, 1 when something is, 2 when the program
/// could not be loaded at all. One construct per line, count first, most
/// frequent first, so a script can sum the column.
fn llvm_gaps(loaded: Result<Vec<parser::Stmt>, String>) -> ! {
    let ast = match loaded {
        Ok(ast) => ast,
        Err(message) => {
            eprintln!("✗ {}", message);
            std::process::exit(2);
        }
    };

    let refused = etamil_compiler::codegen::refusals(&ast);
    if refused.is_empty() {
        std::process::exit(0);
    }

    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for item in &refused {
        *counts.entry(item).or_insert(0) += 1;
    }
    let mut ranked: Vec<(&str, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    for (label, count) in ranked {
        println!("{}\t{}", count, label);
    }
    std::process::exit(1);
}

/// `--artino-gaps`: what `--artino` would refuse, counted like `--llvm-gaps`.
///
/// Only the functions the program reaches are judged, so importing a module
/// with a function no board can run is not a gap unless something calls it.
fn artino_gaps(loaded: Result<Vec<parser::Stmt>, String>) -> ! {
    let ast = match loaded {
        Ok(ast) => ast,
        Err(message) => {
            eprintln!("✗ {}", message);
            std::process::exit(2);
        }
    };
    let refused = etamil_compiler::artino::analyse::refusals(&ast);
    if refused.is_empty() {
        std::process::exit(0);
    }
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for item in refused {
        *counts.entry(item).or_insert(0) += 1;
    }
    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (label, count) in ranked {
        println!("{}\t{}", count, label);
    }
    std::process::exit(1);
}

/// `--artino`: the program as firmware for one board.
///
/// Analysis first, on any build, so a program that cannot become firmware is
/// told why even by an etamil without LLVM. Then, with LLVM: an object for the
/// board, the sketch folder around it, and arduino-cli if it is installed.
fn run_artino(
    ast: &[parser::Stmt],
    source: &Path,
    board_name: &str,
    out: &Path,
    upload: Option<&str>,
) -> ! {
    use etamil_compiler::artino;

    let Some(board) = artino::board(board_name) else {
        let known: Vec<&str> = artino::BOARDS.iter().map(|b| b.name).collect();
        eprintln!(
            "✗ No board called '{}'. artino knows: {}",
            board_name,
            known.join(", ")
        );
        std::process::exit(2);
    };
    if let Some(why) = artino::board_file_mismatch(ast, board.name) {
        eprintln!("✗ {}", why);
        std::process::exit(1);
    }
    // The C++ the program may call: <name>.artino.toml or artino.toml beside it.
    let manifest = match artino::manifest::find(source) {
        None => artino::manifest::Manifest::default(),
        Some(path) => artino::manifest::load(&path, Some(board.name)).unwrap_or_else(|e| {
            eprintln!("✗ {}", e);
            std::process::exit(1);
        }),
    };
    if let Some((number, _)) = manifest.ports.iter().find(|(n, _)| *n < board.serial_ports) {
        eprintln!(
            "✗ artino.toml makes port {} a [[port]], and the {} has its own port {}. Give it a number the board lacks, or boards = [...] leaving this one out.",
            number, board.name, number
        );
        std::process::exit(1);
    }
    let program = match artino::analyse::analyse_with(ast, &manifest.functions) {
        Ok(program) => program,
        Err(mut errors) => {
            errors.sort();
            errors.dedup();
            eprintln!("✗ artino cannot build this program for a board:");
            for error in &errors {
                eprintln!("    - {}", error);
            }
            std::process::exit(1);
        }
    };

    #[cfg(feature = "llvm")]
    {
        let name = artino::sketch::sketch_name(source);
        if let Err(e) = std::fs::create_dir_all(out) {
            eprintln!("✗ Cannot create {}: {}", out.display(), e);
            std::process::exit(1);
        }
        let object = out.join(format!("{}.{}.o", name, board.name));
        let ir = out.join(format!("{}.{}.ll", name, board.name));
        let compiled =
            artino::emit::compile(&program, board, &object, Some(&ir)).unwrap_or_else(|e| {
                eprintln!("✗ {}", e);
                std::process::exit(1);
            });
        // The host is not a board: the object is for the conformance suite to
        // link against its stand-in Arduino API, so there is no sketch.
        if board.fqbn.is_empty() {
            println!("✓ {} for this machine", object.display());
            println!("  IR: {}", ir.display());
            std::process::exit(0);
        }
        let bytes = std::fs::read(&object).unwrap_or_else(|e| {
            eprintln!("✗ Cannot read {}: {}", object.display(), e);
            std::process::exit(1);
        });
        let written = artino::sketch::write(
            out,
            source,
            board,
            program.ports,
            &compiled,
            &manifest,
            &bytes,
        )
        .unwrap_or_else(|e| {
            eprintln!("✗ Cannot write the sketch: {}", e);
            std::process::exit(1);
        });
        let (sketch, libraries) = (written.sketch, written.libraries);
        println!("✓ {} for {} ({})", sketch.display(), board.name, board.fqbn);
        println!("  IR: {}", ir.display());

        let cli = std::env::var("ARDUINO_CLI").unwrap_or_else(|_| "arduino-cli".to_string());
        let mut command = std::process::Command::new(&cli);
        // A known build folder, so the size report can find the linked image.
        let build = sketch.with_file_name("build");
        command
            .arg("compile")
            .arg("--fqbn")
            .arg(board.fqbn)
            .arg("--libraries")
            .arg(&libraries)
            .arg("--build-path")
            .arg(&build)
            .arg(&sketch);
        if let Some(port) = upload {
            command.arg("--upload").arg("--port").arg(port);
        }
        match command.status() {
            Ok(status) if status.success() => {
                // An AVR board's RAM: arduino-cli counts the variables, not the stack.
                if board.triple == "avr" {
                    let elf = build.join(format!("{}.ino.elf", name));
                    match artino::size::report(&cli, &elf, board.ram) {
                        Some((line, true)) => println!("  {}", line),
                        Some((line, false)) => eprintln!("✗ {}", line),
                        None => {}
                    }
                }
                std::process::exit(0)
            }
            Ok(status) => {
                eprintln!("✗ arduino-cli failed ({})", status);
                // The likeliest reason, when the manifest names libraries.
                for library in &manifest.libraries {
                    eprintln!(
                        "  needs the {} library: arduino-cli lib install \"{}\"",
                        library, library
                    );
                }
                std::process::exit(1);
            }
            Err(_) => {
                println!();
                println!("arduino-cli is not on PATH (or set ARDUINO_CLI). To build it elsewhere:");
                println!(
                    "  arduino-cli compile --fqbn {} --libraries {} {}",
                    board.fqbn,
                    libraries.display(),
                    sketch.display()
                );
                std::process::exit(if upload.is_some() { 1 } else { 0 });
            }
        }
    }

    #[cfg(not(feature = "llvm"))]
    {
        let _ = (program, source, out, upload, manifest);
        println!(
            "✓ artino accepts this program for {} ({})",
            board.name, board.fqbn
        );
        eprintln!("✗ Building it needs the LLVM backend, and this etamil was built without it.");
        eprintln!("  Build with `cargo build --release --features llvm` (docs/artino.md).");
        std::process::exit(1);
    }
}

fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let mut use_vm = true; // Default: use VM executor
    let mut use_http_server = false;
    let mut use_async_server = false; // Backend milestone 2: New async server flag
    let mut check_only_mode = false;
    let mut llvm_gaps_mode = false;
    let mut artino_mode = false;
    let mut artino_gaps_mode = false;
    let mut artino_board = "uno".to_string();
    let mut artino_out = "artino-build".to_string();
    let mut artino_upload: Option<String> = None;
    let mut repl_mode = false;
    let mut server_host = "127.0.0.1".to_string();
    let mut server_port = 8080u16;
    let mut filename = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--llvm" => use_vm = false,
            "--vm" => use_vm = true,
            "--check" => check_only_mode = true,
            "--repl" => repl_mode = true,
            "--server" => use_http_server = true,
            "--async" => use_async_server = true, // Backend milestone 2: Async server mode
            "--host" => {
                if i + 1 < args.len() {
                    server_host = args[i + 1].clone();
                    i += 1;
                } else {
                    eprintln!("✗ --host needs a value");
                    std::process::exit(2);
                }
            }
            "--port" => {
                // A port that does not parse used to fall back to 8080
                // silently, so a mistyped flag bound a port the author did not
                // ask for and the program looked like it had started fine.
                if i + 1 < args.len() {
                    match args[i + 1].parse::<u16>() {
                        Ok(port) => server_port = port,
                        Err(_) => {
                            eprintln!(
                                "✗ --port needs a number from 0 to 65535, got '{}'",
                                args[i + 1]
                            );
                            std::process::exit(2);
                        }
                    }
                    i += 1;
                } else {
                    eprintln!("✗ --port needs a value");
                    std::process::exit(2);
                }
            }
            // The GNU --version convention: version, who holds the copyright,
            // the licence, and where the source is. A user who has a binary and
            // no idea where it came from can find all four here, which is what
            // the AGPL's notice requirements are for.
            "--version" | "-V" => {
                println!("etamil {}", env!("CARGO_PKG_VERSION"));
                println!("Copyright (C) 2026 Mohammed Maruff (Esan Maruff)");
                println!(
                    "License AGPL-3.0-or-later: \
                     <https://www.gnu.org/licenses/agpl-3.0.html>"
                );
                println!("This is free software: you are free to change and redistribute it.");
                println!("There is NO WARRANTY, to the extent permitted by law.");
                println!();
                println!("Source: <https://github.com/Maruff/etamil_compiler>");
                return;
            }
            // What the LLVM backend would refuse, without needing LLVM to
            // say so. `run_parity.sh` answers this too, but only on a machine
            // that can build the backend — which is the machine you want the
            // answer before travelling to.
            "--llvm-gaps" => llvm_gaps_mode = true,
            "--artino" => artino_mode = true,
            "--artino-gaps" => artino_gaps_mode = true,
            "--board" | "--out" | "--upload" => {
                let flag = args[i].clone();
                if i + 1 >= args.len() {
                    eprintln!("✗ {} needs a value", flag);
                    std::process::exit(2);
                }
                let value = args[i + 1].clone();
                match flag.as_str() {
                    "--board" => artino_board = value,
                    "--out" => artino_out = value,
                    _ => artino_upload = Some(value),
                }
                i += 1;
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            arg if !arg.starts_with('-') => filename = Some(arg.to_string()),
            // Unknown flags used to be ignored, which meant `etamil --version`
            // fell through to reading a program from stdin and appeared to hang.
            unknown => {
                eprintln!("✗ Unknown option: {}", unknown);
                eprintln!("   Run `etamil --help` to see the available options.");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    // Before anything asks for a file: in the shell, the typing is the program.
    if repl_mode {
        etamil_compiler::repl::run();
    }

    // 1-3. Load, lex, parse, and resolve any இறக்கு imports.
    let loaded = match &filename {
        Some(fname) => module::load_file(Path::new(fname)),
        None => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .expect("Unable to read from stdin");
            module::load_source(&buffer, Path::new("."))
        }
    };

    // --check answers and exits before anything is printed to stdout, so a
    // caller can treat any output at all as a failure.
    if check_only_mode {
        check_only(loaded);
    }

    if llvm_gaps_mode {
        llvm_gaps(loaded);
    }

    if artino_gaps_mode {
        artino_gaps(loaded);
    }

    let ast = match loaded {
        Ok(ast) => ast,
        Err(message) => {
            eprintln!("✗ {}", message);
            std::process::exit(1);
        }
    };
    println!("✓ Parsing complete ({} statements)\n", ast.len());

    // 4. Hold the program to the types it declared. Every error is reported,
    // not just the first: a wrong declaration is usually one of several, and
    // stopping at the first would make fixing them a sequence of recompiles.
    if let Err(errors) = etamil_compiler::check::check(&ast) {
        for error in &errors {
            eprintln!("✗ {}", error);
        }
        std::process::exit(1);
    }

    if artino_mode {
        let source = filename
            .clone()
            .unwrap_or_else(|| "program.qmz".to_string());
        run_artino(
            &ast,
            Path::new(&source),
            &artino_board,
            Path::new(&artino_out),
            artino_upload.as_deref(),
        );
    }

    // Backend milestone 2: Check if async server mode is enabled
    if use_async_server {
        // This used to print a warning saying the async runtime was not wired
        // up and that the flag fell back to the synchronous server. It was
        // left behind when ROADMAP item 4 landed: run_async_server below
        // builds a real tokio runtime and starts the real async server, so the
        // warning told users a working flag was broken.
        println!("=== eTamil HTTP Server (--async) ===");
        println!("🚀 Starting server on {}:{}\n", server_host, server_port);

        if let Err(e) = run_async_server(&server_host, server_port, ast) {
            eprintln!("❌ Async server error: {}", e);
            std::process::exit(1);
        }
    } else if use_http_server {
        // === Backend milestone 1: SYNCHRONOUS HTTP SERVER MODE (MVP) ===
        println!("=== eTamil HTTP Server (Minimum Viable Backend) ===\n");

        let mut server = HttpServer::new(&server_host, server_port);
        register_routes(
            &mut server,
            ast,
            |server, method, path, program| server.register_route(method, path, program),
            |server, seconds, program| server.register_schedule(seconds, program),
        );

        // Also register health check endpoint
        server.register_route(
            "GET",
            "/health",
            vec![parser::Stmt::Print(parser::Expr::Number(
                rust_decimal::Decimal::from(200),
            ))],
        );

        // Start the server
        if let Err(e) = server.start() {
            eprintln!("❌ Server error: {}", e);
            std::process::exit(1);
        }
    } else if use_vm {
        // === VM EXECUTION PATH ===
        println!("=== eTamil VM Executor ===\n");

        // Compile AST to bytecode
        let bytecode = vm::bytecode::compiler::BytecodeCompiler::compile_statements(ast);
        println!("✓ Bytecode generated ({} instructions)", bytecode.len());
        println!("=== Execution Output ===\n");

        // Execute bytecode
        let mut vm = vm::VM::new();
        match vm.execute(bytecode) {
            Ok(_) => println!("\n✓ Execution completed successfully"),
            Err(e) => {
                eprintln!("✗ Runtime error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // === LLVM COMPILATION PATH (Legacy) ===
        #[cfg(feature = "llvm")]
        {
            println!("=== LLVM Code Generation ===");

            let mut compiler = codegen::Compiler::new();
            compiler.compile(ast);

            // The LLVM backend covers less of the language than the VM. If it
            // met anything it cannot build, refuse rather than emit IR that
            // silently computes something else.
            if !compiler.unsupported().is_empty() {
                eprintln!("✗ The LLVM backend cannot compile this program.");
                eprintln!("  Unsupported here, though the VM handles them:");
                let mut seen: Vec<&String> = compiler.unsupported().iter().collect();
                seen.sort();
                seen.dedup();
                for item in seen {
                    eprintln!("    - {}", item);
                }
                eprintln!();
                eprintln!("  Run it on the VM instead:  etamil --vm <file>");
                std::process::exit(1);
            }

            println!("\nGenerated LLVM IR:");
            compiler.dump_module();

            match compiler.emit_ir("output.ll") {
                Ok(_) => println!("\n✓ Successfully saved LLVM IR to output.ll"),
                Err(e) => eprintln!("✗ Error writing IR: {}", e),
            }
        }

        #[cfg(not(feature = "llvm"))]
        {
            eprintln!("❌ Error: LLVM backend is not available on this platform.");
            eprintln!("   Platform: {}", std::env::consts::OS);
            eprintln!("   Reason: LLVM feature not enabled during build");
            eprintln!();
            eprintln!("Please use one of the following modes instead:");
            eprintln!("  --vm              VM bytecode executor (default, recommended)");
            eprintln!("  --server          HTTP sync server");
            eprintln!("  --async           HTTP async server (production)");
            eprintln!();
            eprintln!("Examples:");
            eprintln!("  etamil --vm myprogram.qmz");
            eprintln!("  etamil --async --port 8080 api.qmz");
            std::process::exit(1);
        }
    }
}

/// Start the concurrent server: `--async`.
///
/// The runtime is built here rather than around `main`, because only this path
/// needs one. `main` being `#[tokio::main]` made every blocking database driver
/// panic with "Cannot start a runtime from within a runtime" — and blocking
/// drivers are the reason the VM is synchronous in the first place. See
/// docs/ARCHITECTURE.md.
fn run_async_server(
    host: &str,
    port: u16,
    ast: Vec<parser::Stmt>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut server = AsyncHttpServer::new(host, port);
    register_routes(
        &mut server,
        ast,
        |server, method, path, program| server.register_route(method, path, program),
        |server, seconds, program| server.register_schedule(seconds, program),
    );

    // Handlers run on the blocking pool, so the worker threads here only ever
    // accept connections and move bytes.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(server.start())
}

/// Split a program into `வழி` routes and the prelude they share, and register
/// each one.
///
/// The remainder of the file — imports, functions, setup — is compiled into
/// every handler, so a route can call what the file defines. Both servers
/// register the same way, which is why this is written once and handed the
/// registration function.
fn register_routes<S>(
    server: &mut S,
    ast: Vec<parser::Stmt>,
    mut register: impl FnMut(&mut S, &str, &str, Vec<parser::Stmt>),
    register_schedule: impl Fn(&mut S, u64, Vec<parser::Stmt>),
) {
    // Routes and timed jobs are both lifted out; what is left is the prelude
    // they share.
    let (lifted, prelude): (Vec<parser::Stmt>, Vec<parser::Stmt>) =
        ast.into_iter().partition(|s| {
            matches!(
                s,
                parser::Stmt::DefineRoute { .. } | parser::Stmt::Schedule { .. }
            )
        });
    let (routes, schedules): (Vec<parser::Stmt>, Vec<parser::Stmt>) = lifted
        .into_iter()
        .partition(|s| matches!(s, parser::Stmt::DefineRoute { .. }));

    if routes.is_empty() {
        // No வழி statements: the whole program answers every request, which is
        // how server programs behaved before routing existed.
        println!("ℹ️  No வழி routes found; serving the whole program on /");
        for method in ["GET", "POST", "PUT", "DELETE"] {
            register(server, method, "/", prelude.clone());
        }
        return;
    }

    for schedule in schedules {
        if let parser::Stmt::Schedule { seconds, body } = schedule {
            let seconds = match seconds {
                parser::Expr::Number(n) => {
                    rust_decimal::prelude::ToPrimitive::to_u64(&n).unwrap_or(0)
                }
                other => {
                    eprintln!(
                        "✗ இடைவெளி needs a literal number of seconds, got {:?}",
                        other
                    );
                    std::process::exit(1);
                }
            };
            let mut program = prelude.clone();
            program.extend(body);
            register_schedule(server, seconds, program);
        }
    }

    for route in routes {
        if let parser::Stmt::DefineRoute {
            method,
            path,
            handler,
        } = route
        {
            let path = match path {
                parser::Expr::String(literal) => literal,
                other => {
                    eprintln!("✗ வழி needs a literal path, got {:?}", other);
                    std::process::exit(1);
                }
            };
            let mut program = prelude.clone();
            program.extend(handler);
            register(server, &method, &path, program);
        }
    }
}
