# etamil_compiler

The compiler and runtime for **[eTamil](https://etamil.in)** — a programming
language whose vocabulary is Tamil, aimed at Indian accounting, tax and
FinTech.

```bash
cargo install etamil_compiler
```

That installs the `etamil` binary. The standard library travels inside it, so
there is nothing else to place and no environment variable to set:

```etamil
இறக்கு "nUlakam/paNam/paNam.qmz";
அச்சு ரூபாய்(12345678.5);        // ₹1,23,45,678.50 — Indian digit grouping
```

```bash
etamil --vm money.qmz
```

## What the language is

Not English with translated keywords. Finance is part of the vocabulary:
`வரவு` (credit), `பற்று` (debit), `வரி` (tax), `இருப்புநிலை` (balance sheet).
Every keyword has three interchangeable spellings — Tamil script, a romanized
form typable on a plain keyboard, and for some an English alias:

```etamil
எண் வருவாய் = 100000;    // Tamil script
eN varuvAy = 100000;      // romanized (ezuqqu scheme)
```

**Money is exact.** Every number is a fixed-point decimal, not `f64`, because
this is a language for tax and accounting:

```etamil
அச்சு 0.1 + 0.2;      // 0.3      — not 0.30000000000000004
அச்சு 18%;            // 0.18     exactly
```

## What ships in this crate

- The lexer, parser, type checker, bytecode compiler and VM.
- An optional LLVM backend (`--features llvm`, Linux/macOS).
- `nUlakam/` — the standard library, **written in eTamil**, compiled into the
  binary. Strings, maths, arrays, money, JSON, and a double-entry accounting
  framework with GST and the three financial statements.
- Database drivers behind features: `sqlite` (default), `postgres`, `mysql`,
  `mongodb`.
- An HTTP server, a REPL (`--repl`), and `--check`, which parses and type
  checks without running anything.

The library is looked up on disk first — beside the importing file, then along
`ETAMIL_PATH`, then next to the executable, then the platform data directory —
and only then from the copy inside the binary. So a checkout or a distribution
package always overrides the built-in one, and the library can be worked on
without rebuilding the compiler.

## Command line

| Flag | |
|---|---|
| `--vm` | Run on the bytecode VM (default) |
| `--check` | Lex, parse and type check only — never runs the program |
| `--repl` | Interactive session |
| `--server` / `--async` | HTTP server, synchronous or concurrent |
| `--llvm` | LLVM backend (requires `--features llvm`) |

## Documentation

- Manual — <https://etamil.in/manual/> ([தமிழ்](https://etamil.in/ta/manual/))
- Keyword reference — <https://etamil.in/keywords/>
- Source, issues and the full README —
  <https://github.com/Maruff/etamil_compiler>

## Licence

[AGPL-3.0-or-later](LICENSE). eTamil is a backend language: running a modified
compiler or runtime as a network service counts as distribution, so the changes
have to be offered back to the people using that service.
