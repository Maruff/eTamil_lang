# eTamil Documentation

**Resuming work?** Read [CONTINUATION.md](CONTINUATION.md) first — state, what is deliberately unfinished, and the traps that cost time.

Start with the [README](../README.md) for what eTamil is and which parts of it run today, then pick a page below.

What changed in a release, and what it adds up to: [CHANGELOG](../CHANGELOG.md).

## Getting started

| Page | What it covers |
|---|---|
| [Installation](getting-started/INSTALLATION.md) | Prerequisites, building from source, verifying, troubleshooting |
| [Quick Start](getting-started/QUICKSTART.md) | Your first program, through to files and a server |

## Libraries

Both are written in eTamil, not Rust.

| Page | What it covers |
|---|---|
| [Standard library](../nUlakam/README.md) | `nUlakam/` — strings, math, arrays, money with Indian digit grouping |
| Accounting framework | `nUlakam/kaNakkiyal/` — chart of accounts, double-entry ledger, GST, and the three statements. See `examples/finance/kaNakkiyal.qmz` for a full cycle |

## Language reference

| Page | What it covers |
|---|---|
| [Keywords](reference/KEYWORDS.md) | Every keyword in all three spellings, with its token name — generated from the lexer |
| [Commands](reference/COMMANDS.md) | CLI flags, exit codes, environment variables |
| [Tamil Letter Equivalents](reference/COMPILER_TAMIL_LETTER_EQUIVALENTS.md) | How the ezuqqu romanization is derived |
| [Script rules](reference/SCRIPT_RULES.md) | The `_` and `__` marks that tell eTamil ASCII from English ASCII |
| [File I/O](reference/FILE_IO_FEATURES.md) | File and CSV statements |
| [VM Quick Start](reference/QUICK_START_VM.md) | Running programs on the bytecode VM |
| [Quick Reference](reference/QUICK_REFERENCE.md) | Syntax at a glance — partly outdated |
| [eTamil Standard](reference/ETAMIL_STANDARD.md) | The language standard, with PDFs alongside |
| [VS Code Extension](reference/VSCODE_README.md) | Editor support (the extension itself lives in another repo) |

## Architecture

| Page | What it covers |
|---|---|
| [Module Overview](architecture/OVERVIEW.md) | How the crate is organized |
| [VM Implementation](architecture/VM_IMPLEMENTATION_SUMMARY.md) | Bytecode format and interpreter |

The pipeline is `lexer.rs` → `parser.rs` → `vm/bytecode/compiler.rs` → `vm/interpreter.rs`. `codegen.rs` is an optional LLVM backend replacing the last two stages.

## Backend

Both pages carry a status banner; read it first.

| Page | What it covers |
|---|---|
| [HTTP Server](backend/HTTP_SERVER_QUICKREF.md) | Server usage. `வழி` routes and a worker pool now work; `--async` is still an alias for `--server` |
| [Database Commands](backend/DATABASE_COMMANDS_GUIDE.md) | Database syntax. SQLite executes; queries are always parameterised. Other engines report that they are unsupported |

## Planning

| Page | What it covers |
|---|---|
| [Architecture](ARCHITECTURE.md) | Why the project is built this way: the layering rule, every design decision and its reasoning, the known warts, and how to work on it |
| [Roadmap](ROADMAP.md) | What is unfinished, why it matters, what finishing it takes |
| [Performance](PERFORMANCE.md) | Measured against eight other languages: which claims hold, which would be refuted, and the optimisation work in payoff order |
| [Language gaps found building nuNNaRivu](language-gaps-from-nuNNaRivu.md) | What a library of 88 functions hit that the language does not yet do well — measured, ranked by what it costs |
| [artino](artino.md) | Design, not yet built: the LLVM backend for Arduino boards (Uno, Nano, Mega, Pico), and closing the desktop VM–LLVM gap |

## Examples

Sample programs live in [`../examples/`](../examples/). `./scripts/run_examples.sh` runs them all and enforces which are expected to fail — today only the routed API examples, which need `--server` rather than `--vm`.

---

## A note on older documents

Superseded documents — phase completion reports, backend planning drafts, older installation guides, VS Code write-ups — were removed rather than kept in the tree. Several described features as complete that were never wired up, which made them actively misleading. They remain in git history if you need them:

```bash
git log --diff-filter=D --name-only -- 'docs/archive/*'
```

When a page here and the **code** disagree, the code wins — please open an issue or fix the page.
