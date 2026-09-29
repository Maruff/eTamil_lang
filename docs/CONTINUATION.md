# Continuation notes

Read this first when picking up eTamil work in a fresh session. It records what
was done, what is deliberately unfinished, and the traps that cost time.

Last updated: 2026-09-25.

## State

Work now goes to GitHub through pull requests, in both repositories — five were
opened and merged this way in one session — and the branch is deleted, local and
remote, after each merge. Both checkouts sit on `main` at origin HEAD between
pieces of work.

The website is a separate repository, `Maruff/eTamil`, cloned beside this one as
`../eTamil_site`. Several things now span the two; see the counts note below.

```bash
cd etamil_compiler && cargo test                    # 511 tests
cargo test --features mongodb                       # 521 tests
cd .. && bash scripts/run_examples.sh               # 114 as expected, 1 skipped
```

**The standard library now lives inside the binary.** `build.rs` compiles
`nUlakam/` in and `module::locate` tries it last, after every filesystem
lookup, so a checkout or `ETAMIL_PATH` still overrides it. This is what makes
`cargo install` — and every other package manager, which places an executable
and nothing else — produce a working compiler rather than one whose first
import fails. Adding a file to `nUlakam/` needs no build change; `build.rs`
reruns on any change under it.

## What was built recently, newest first

| Area | Where |
|---|---|
| Line tables rather than full debug info in the dev profile | `etamil_compiler/Cargo.toml` |
| `script_spans`, so the browser and the extension read the script marks from the compiler rather than from two copies of the rule | `src/wasm.rs`, `eTamil_site/ide/src/etamil-font.js` |
| Rule 2's `__` opens a region across a comment block, in the scanner, the grammar and the CodeMirror parser | `eTamil_Code/src/marks.ts`, `scripts/generate_editor_support.py`, `docs/reference/SCRIPT_RULES.md` |
| eTamil/ISO switch in both editors, and the eTamil font in the browser one | `eTamil_Code/src/scriptMode.ts`, `eTamil_site/ide/src/etamil-script-switch.js` |
| Two drift checks, and the text checks run once instead of once per OS | `scripts/check_wasm_boundary.py`, `scripts/check_site_counts.py`, `.github/workflows/ci.yml` |
| Recognition as a language: status, gates, order | `docs/RECOGNITION.md` |
| Standard library compiled into the binary | `etamil_compiler/build.rs`, `src/stdlib.rs`, `tests/installed_binary.rs` |
| Crate publishable to crates.io | `etamil_compiler/Cargo.toml` (`include`, keywords), `etamil_compiler/README.md`, `scripts/vendor_for_publish.py` |
| Citable — `CITATION.cff` | repository root |
| LLVM: whole numbers as `i64`, exact division, globals | `src/codegen.rs`, `src/codegen_limits.rs`, `examples/finance/pYcA_kaNakku.qmz` |
| Money as whole paise | `nUlakam/paNam/kAcu.qmz` |
| LLVM refusal for f64 arithmetic | `etamil_compiler/src/codegen_limits.rs`, `docs/llvm-backend-gaps.md` |
| Database attempt-and-report; `:memory:` never pooled | `interpreter.rs`, `db/pool.rs` |
| Insurance, customs and trade | `nUlakam/kAppItu/`, `nUlakam/cuwkam/` |
| MongoDB (`--features mongodb`) | `src/mongo.rs` |
| Array and record equality (was broken) | `src/vm/value.rs` |
| Redis, RESP implemented here | `src/redis.rs`, `nUlakam/qaLam/retis.qmz` |
| Depreciation, payroll, tax rate tables | `nUlakam/kaNakkiyal/` |
| UPI addresses, links and states | `nUlakam/upi/` |
| Fabric via REST gateway | `nUlakam/cawkili/` |
| Core banking: interest, loans, asset classification | `nUlakam/vawki/` |
| Tests written in eTamil; `வெளியேறு` | `nUlakam/cOqaZY/cOqaZY.qmz` |
| REPL (`--repl`) | `src/repl.rs` |
| mTLS, ECDSA | `src/mtls.rs`, `src/signing.rs` |
| ODF/OOXML documents | `nUlakam/vativam/AvaNam.qmz`, package builtins |

## Next, in the order the user asked for

1. **Run the LLVM work on Ubuntu.** *Partly answered:* CI now has an
   `LLVM backend builds` job that installs LLVM 18, builds `--features llvm`
   and runs `Backend parity (--vm against --llvm)`, and it has been green on
   every run this session. So it is built and the parity harness does execute
   — on a runner. What has still never happened is a developer running it on a
   machine they can poke at, which is what the section below is about.
2. **The boxed value** — the next LLVM gap, and the one everything else waits
   on. `docs/llvm-backend-gaps.md` has the order.
3. **eCommerce libraries** — the one item from the original seven never started.
   Builds on the accounting and GST modules, which exist.
4. **gRPC and protobuf** — only if direct Fabric peer access is wanted. The REST
   gateway already works, so this buys little.

### The LLVM work has not been run

`llvm-sys 180` needs LLVM 18 and the feature is Linux/macOS only, so nothing in
`codegen.rs` has executed on the machine it was written on. It *is* type-checked:

```bash
python scripts/check_llvm_backend.py     # works on Windows; no LLVM needed
```

That builds a signature-only stand-in for `llvm-sys` out of llvm-sys's own
source in the cargo registry cache and runs `cargo check --features llvm`. So
the Rust compiles, and the file is editable here at all — it had broken three
times without anyone noticing before this existed. What it cannot tell you is
whether the IR verifies or whether it answers what the VM answers.

```bash
(cd etamil_compiler && cargo build --release --features llvm)
./scripts/run_parity.sh
```

`run_parity.sh` runs every example under both backends, fails only where they
*disagree* — a refusal is expected and counted — and ranks what stopped each
one. `examples/finance/pYcA_kaNakku.qmz` is the example written to compile on
both, so it is the one to look at first: if it refuses, read why; if it
mismatches, that is the bug.

Expect the possibility of a trivial IR-verifier complaint before judging
anything else. The likely candidates, in order: a block left without a
terminator around `எனில்` with a `திரும்பு` in it, and `printf`/`write`
argument types.

## Two sessions, one checkout

A second session worked in this same working tree for a while — a wasm build for
the browser editor — and landed it in `3db8985`. One commit, `5bb806c`,
accidentally swept its `wasm.rs` in alongside MongoDB work before that.

**Check `git status` before staging, and stage files by name rather than `-A`.**
Better: give a concurrent session its own git worktree. Staging by name is what
kept `0dbffe1` clean while `3db8985` was being written beside it.

### Editor support drifts silently

`eTamil_Code/src/generated/` and `eTamil_Code/syntaxes/` are generated from the
compiler and from nUlakam, and nothing regenerates them for you:

```bash
python scripts/generate_editor_support.py --check    # what CI runs
python scripts/generate_editor_support.py            # fix it
```

**The website's numbers drift the same way, and are gated now too.** etamil.in
states the keyword, builtin and stdlib counts in thirteen places and the version
in `_config.yml`, all typed by hand. They were wrong — 202 tokens across 541
spellings when the lexer had 203 and 545, 691 nUlakam functions when it had 696,
and 1.0.0 after the compiler was tagged v1.1.0:

```bash
python scripts/check_site_counts.py --check --site ../eTamil_site
```

CI checks out `Maruff/eTamil` to run it, which couples the two: **a keyword
added here fails that step until the site is updated too**, and the fix lives in
the other repository. Merge the site's PR first.

**Run the check before pushing, whenever a nUlakam function or a builtin was
added or renamed.** An earlier note here guessed the outstanding drift came from
the wasm session's `interpreter.rs` edits; it did not. It was ten `stdlib`
entries from `nUlakam/paNam/kAcu.qmz`, drifting since `b2de386` — a plain nUlakam
commit, no builtins involved. Adding a function to nUlakam is enough to fail
that gate.

## Traps that cost real time

**Reserved words.** Many of the words a financial library most wants are
keywords and cannot be used as names: `காப்பீடு`, `இழப்பு`, `விலக்கு`, `சரக்கு`,
`கட்டணம்`, `பங்கு`, `தேய்மானம்`, `சொத்து`, `செலவு`, `ஊதியம்`, `நிகர`, `விலை`,
`அசல்`, `தொகை`, `முறை`, `உரை`, `உடல்`, `படி`, `எழுது`, `வரம்பு`, `மதிப்பீடு`,
`தலைப்பு`, `விசை`, `இல்லை`, `பொருள்`, `வரிசை`. Compounds lex as ordinary
identifiers, so `அசல்_தொகை` is fine. **Check before writing, not after:**

```bash
grep -cE '#\[regex\("WORD\|' etamil_compiler/src/lexer.rs
```

**A copied binary is sometimes still busy.** `tests/installed_binary.rs` copies
the compiler into a scratch directory and executes the copy. `fs::copy` closes
what it writes, but `cargo test` runs on several threads and a spawn from any of
them can inherit that descriptor between the open and the close — Linux then
refuses to exec with `ETXTBSY`, `Text file busy`. It is a race, not a state: the
same test passes on a re-run, and four other tests in that file copied and ran
the same binary in the job that failed. Both exec sites retry on it now. **This
cannot be reproduced on Windows**, which has no ETXTBSY, so a green local run
says nothing about it; Linux CI is the only thing that confirms the fix.

**`target/debug` will fill the disk if nothing stops it.** It reached 40.9 GB
against release's 4.1 GB on the same code, because the dev profile ran at
cargo's default `debug = true` and Windows MSVC writes a `.pdb` per binary. The
failure that exposed it was not a warning about space:

```
LINK : fatal error LNK1318: Unexpected PDB error; LIMIT (12)
```

That is the linker unable to write a PDB onto a full disk, and it reads like a
toolchain bug. `[profile.dev] debug = 1` is in `Cargo.toml` now. Note when
measuring: most of that 40.9 GB was accumulation rather than the setting —
release fell to 0.63 GB on a clean rebuild with no configuration change at all.

**A function cannot change a global.** Assigning to a name inside a `செயல்`
makes a local. That is why `cOqaZY.qmz` threads the test run through every
assertion instead of keeping a counter.

**`சோதனை_முடிவு` returns on success and only exits on failure**, so calling it
in a skip path prints a summary and then carries on into the tests it meant to
skip. Branch with `இன்றேல்` instead.

**A SQL NULL arrives as `nil`,** and `nil` equals nothing — test it with
`வகை(x) == "nil"`, not by comparing values. And eTamil has no `nil` to bind, so
a column that should be NULL is *omitted from the INSERT* rather than bound to
`""`.

**`%` is postfix percentage, not infix modulo.** `18%` is `0.18`, and
`சதவீதம்` divides by 100 itself — so it wants `18`, not `18%`. Writing the
natural thing gives an answer a hundred times too small and nothing complains.
Both `vari.qmz` and `vatti.qmz` pin this in their suites.

**`&` and `+` bind equally and associate left.** So
`அச்சு "மொத்தம்: " & அ + ஆ` is `("மொத்தம்: " & அ) + ஆ`, which is a number, and
the label vanishes from the output with nothing complaining. Bracket the sum:
`& (அ + ஆ)`. Cost a wrong line in `pYcA_kaNakku.qmz` before the VM run caught
it.

**Windows shell.** Heredocs mangle backslashes and backticks in shell
double-quotes are command substitution — a ROADMAP line lost all its code spans
that way. Write patches as files with the Write tool and run them with
`PYTHONIOENCODING=utf-8 python file.py`; console output needs that or Tamil
crashes on cp1252.

## The rule the libraries follow

Engine in eTamil, statutory figures in SQLite, supplied and verified by the
user. No rate, threshold or slab is hardcoded anywhere. Tables are
effective-dated so a return re-run next year produces what it produced when it
was filed, and `சரிபார்க்கப்படாதவை`-style functions list rows still marked
`PLACEHOLDER` so a program can refuse to file a figure nobody has vouched for.

`nUlakam/kaNakkiyal/vari_vikiqam.sql` ships the schema and the 36 states with
their GST codes, and **no rates at all**.
