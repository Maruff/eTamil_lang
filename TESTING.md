# Testing eTamil on Ubuntu

Every command here is meant to be run from a fresh clone. Where something is
expected to fail, that is stated — a few examples fail deliberately, and a
run that "passes" them would mean a regression.

```bash
git clone https://github.com/Maruff/etamil_compiler.git
cd etamil_compiler
```

---

## 1. Prerequisites

```bash
sudo apt update
sudo apt install -y build-essential pkg-config curl git
```

Rust **1.85 or newer** (the crate uses edition 2024):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc --version          # expect 1.85.0 or newer
```

Optional, only for the `--llvm` backend — note it must be **LLVM 18**, since
`llvm-sys` is pinned to `"180"`:

```bash
sudo apt install -y llvm-18-dev libpolly-18-dev
export LLVM_SYS_180_PREFIX=/usr/lib/llvm-18
```

---

## 2. Build

```bash
cd etamil_compiler
cargo build --release
```

The binary is `target/release/etamil`. Put it on your PATH for the rest of
this document:

```bash
export PATH="$PWD/target/release:$PATH"
export ETAMIL_PATH="$PWD/.."          # so இறக்கு can find nUlakam/
etamil --version
etamil --help
```

**Expected:** a version line and a usage block. If `etamil --version` hangs,
you are on an old build — that bug is fixed.

---

## 3. The test suite

```bash
cd etamil_compiler
cargo test
```

**Expected: 105 passed, 0 failed**, plus the unit tests inside `src/http/`
and `src/fileio/`.

The suite asserts on **program results**, not exit codes. An older suite
checked only that examples exited 0, which is why several wrong-answer bugs
survived in it — do not reintroduce that style.

Useful variations:

```bash
cargo test foreach            # one group
cargo test -- --nocapture     # see program output
cargo test --release          # optimised
```

---

## 4. Run every example

```bash
cd ..            # repository root
./scripts/run_examples.sh
```

**Expected: 20 ran, 3 failed by design.** The script fails the build if any
example other than those three breaks, or if one of those three unexpectedly
succeeds.

The three deliberate failures, all reporting "not implemented in the VM yet":

| Example | Why |
|---|---|
| `examples/api/simple_api.qmz` | `வழி` route statements are not executable |
| `examples/db_samples/multi_db_test.qmz` | no database layer |
| `examples/db_samples/test_db_connectivity.qmz` | no database layer |

That is the intended behaviour: statements the VM cannot execute fail loudly
rather than being silently skipped.

---

## 5. Run examples individually

### Language features

```bash
etamil --vm examples/language/ceyalkaL.qmz
```
Functions, recursion, `சரி`/`தவறு` results and the `?` operator.
**Expect:** `காரணீயம்(6) = 720`, an average of `333.33`, a caught error.

```bash
etamil --vm examples/language/aNikaL_poruLkaL.qmz
```
Arrays (columns), records (rows), iteration, `புலம்_எடு`.
**Expect:** a total outstanding of `59800`.

### Finance

```bash
etamil --vm examples/finance/vaNikavari_pattiyal.qmz
```
A GST invoice with CGST/SGST split and Indian digit grouping.
**Expect:** amounts like `₹1,64,997.00`, tax split evenly across CGST/SGST.

```bash
etamil --vm examples/finance/campaLam.qmz
```
A payroll run over an array of employee records with slab tax and PF.
**Expect:** four employees and a total in `₹` with lakh/crore grouping.

### Basics and I/O

```bash
echo "950000" | etamil --vm examples/basic_samples/example.qmz
```
**Expect:** `High Tax Bracket` then `30000`.

```bash
echo "500000" | etamil --vm examples/basic_samples/example.qmz
```
**Expect:** `Low Tax Bracket (No Tax)`.

```bash
cd /tmp && etamil --vm ~/etamil_compiler/examples/io_samples/simple_fileio.qmz
```
These write files into the **current directory**, so run them somewhere
disposable. **Expect:** `கோப்பிலிருந்து வருவாய்: 125000` and an `output.txt`.

### The standard library

```bash
etamil --vm nUlakam/paNam/paNam.qmz     # loads cleanly, defines functions, prints nothing
```

To exercise it:

```bash
cat > /tmp/money.qmz <<'EOF'
இறக்கு "nUlakam/paNam/paNam.qmz";
அச்சு ரூபாய்(12345678.5);
அச்சு காசு_வடிவம்(0 - 4500.5);
EOF
etamil --vm /tmp/money.qmz
```

**Expect:**
```
₹1,23,45,678.50
-4,500.50
```

Note the Indian grouping — three digits, then pairs — not `12,345,678.50`.

---

## 5b. The accounting framework

```bash
etamil --vm examples/finance/kaNakkiyal.qmz
```

A full cycle: opening capital, a GST sale, a GST purchase, rent, a receipt,
and a deliberately unbalanced entry that must be refused.

**Expect:**

| | |
|---|---|
| Unbalanced entry | `✓ சமநிலையற்ற பரிவர்த்தனை மறுக்கப்பட்டது` |
| Trial balance | `பற்று 7,86,000.00   வரவு 7,86,000.00` and `✓ சமநிலை` |
| Net profit | `₹1,05,000.00` |
| Balance sheet | assets `₹6,91,000.00` and `✓ சொத்து = பொறுப்பு + பங்கு` |

If the trial balance does not agree, the ledger is being written by something
other than `பதிவிடு`, since posting refuses anything unbalanced.

---

## 6. Check decimal correctness

This is the property the language exists for, so it is worth verifying
directly:

```bash
cat > /tmp/exact.qmz <<'EOF'
அச்சு 0.1 + 0.2;
அச்சு 99.99 * 3;
அச்சு 20%;
EOF
etamil --vm /tmp/exact.qmz
```

**Expect exactly:**
```
0.3
299.97
0.2
```

If you see `0.30000000000000004`, the build has regressed to `f64`.

---

## 7. The HTTP server

```bash
etamil --server --port 8080 examples/backend/hello_server.qmz &
sleep 1
curl -i http://localhost:8080/
curl -s http://localhost:8080/health
```

**Expect:** an HTTP response, and a worker-count line at startup.

### Verify it is actually concurrent

The server used to handle one connection at a time. Twenty parallel requests
should all return promptly:

```bash
time (seq 1 20 | xargs -P 20 -I{} curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8080/)
```

**Expect:** twenty `200`s, finishing in well under a second. If the elapsed
time scales linearly with request count, the worker pool has regressed to
serial handling.

Tune the pool:

```bash
ETAMIL_WORKERS=4 etamil --server --port 8080 examples/backend/hello_server.qmz
```

Stop it with `kill %1`.

`--async` currently prints a warning and runs the same synchronous server;
that is expected until roadmap item 4 lands.

---

## 7b. The database layer

SQLite is compiled in by default (`--features sqlite`, on unless you pass
`--no-default-features`). `rusqlite` is bundled, so no system SQLite is
needed — but it does compile C, which is why `build-essential` is required.

```bash
cd /tmp
etamil --vm ~/etamil_compiler/examples/db_samples/kaNakku_qaLam.qmz
```

**Expect:** a ledger created, four rows inserted, credits and debits listed
with `₹` formatting, and a closing balance of `₹2,17,300.00`. It writes
`kaNakku.db` in the current directory, hence running it from `/tmp`.

Check that parameters really are bound rather than interpolated:

```bash
cat > /tmp/inject.qmz <<'EOF'
தளம்_இணை சீகுலைட், ":memory:";
தளம்_செய் "CREATE TABLE t (peyar TEXT)", [];
தளம்_செய் "INSERT INTO t VALUES (?)", ["Ravi'; DROP TABLE t; --"];
தளம்_வினா "SELECT peyar FROM t WHERE peyar = ?", ["Ravi'; DROP TABLE t; --"], r;
அச்சு நீளம்(r);
EOF
etamil --vm /tmp/inject.qmz
```

**Expect `1`.** The table survives and the hostile string is matched as data.
If the table were dropped, the value would have reached SQLite as SQL.

A build without the driver should say so rather than fail obscurely:

```bash
cd etamil_compiler && cargo build --release --no-default-features
./target/release/etamil --vm /tmp/inject.qmz
```

**Expect:** `this build has no SQLite support: rebuild with --features sqlite`.

### 7c. MySQL / MariaDB connectivity

MySQL support is optional and is enabled with the `mysql` Cargo feature. The
same backend works with MariaDB because both servers speak the MySQL wire
protocol. The driver currently uses a plain local TCP connection, so this
check is intended for a local development server.

Install and start MariaDB (use `mysql-server` and `mysql-client` instead on a
MySQL installation):

```bash
sudo apt install -y mariadb-server mariadb-client
sudo systemctl enable --now mariadb
sudo mariadb -e "CREATE DATABASE IF NOT EXISTS kaNakku"
sudo mariadb -e "CREATE USER IF NOT EXISTS 'etamil'@'localhost' IDENTIFIED BY 'etamil-test'; GRANT ALL ON kaNakku.* TO 'etamil'@'localhost'; FLUSH PRIVILEGES"
```

Build the compiler with the driver enabled:

```bash
cd etamil_compiler
cargo build --release --features mysql
```

The checked-in lockfile keeps `subprocess` at 0.2.9, because newer releases use
language syntax unavailable in the documented Rust 1.85 minimum.

The sample at `examples/db_samples/mYcIkul_qaLam.qmz` contains the connection
URL. Before running it, change its `தளம்_இணை` line to match the test account,
for example:

```etamil
தளம்_இணை மைசீகுல், "mysql://etamil:etamil-test@127.0.0.1:3306/kaNakku";
```

Then run the end-to-end connectivity check from the repository root:

```bash
ETAMIL_TEST_MYSQL=1 ./scripts/run_examples.sh
```

Or run only the MySQL sample:

```bash
ETAMIL_PATH="$PWD" etamil_compiler/target/release/etamil --vm examples/db_samples/mYcIkul_qaLam.qmz
```

**Expect:** the ledger rows are inserted and queried, `0.1 + 0.2` is reported
as exactly `0.3`, the integer-key lookup returns one row, `VARCHAR` and
`DECIMAL` are reported as different types, the date is returned as ISO text,
and the injection check leaves the table intact. The script skips this sample
unless `ETAMIL_TEST_MYSQL=1` is set; it must not be counted as a passing test
when no MySQL/MariaDB server is available.

Verified on 2026-08-20 against a local MySQL server using the checked-in sample
with `root` configured without a password. The direct run completed all checks,
including `0.1 + 0.2 = 0.3`, typed result conversion, NULL/date handling, and
the injection check. Prefer the dedicated `etamil` test account above instead
of an empty root password outside a disposable development database.

---

## 8. The LLVM backend

Linux/macOS only. The backend is a deliberately smaller subset than the VM:
unsupported constructs are reported and no IR is emitted for those programs.
The build and a minimal arithmetic smoke test are verified.

```bash
cd etamil_compiler
cargo build --release --features llvm
printf 'எண் x = 2 + 3;\nஅச்சு x;\n' >/tmp/etamil_llvm_smoke.qmz
./target/release/etamil --llvm /tmp/etamil_llvm_smoke.qmz
test -s output.ll
head -n 12 output.ll
```

**Expect:** the command succeeds and writes non-empty LLVM IR to `output.ll`.
The verified LLVM subset also covers numeric functions, imported modules,
numeric array iteration/indexing, and numeric record field access. Heterogeneous
values and other unsupported constructs are correctly refused; run those
programs with `--vm` instead. Without the feature, `--llvm` prints an
explanatory error and exits 1, which is also correct.

---

## 9. Romanization audit

```bash
python3 scripts/transliterate.py --check
python3 scripts/check_names.py --check
```

**Expect: 0 off-scheme from each.** Both gate CI, so a failure is a regression.

The first reads `lexer.rs` and holds every keyword's romanization to the
scheme. The second holds everything that is *not* a keyword — module and file
names, SQL tables and columns, record keys — because nothing did, and
`viziqam` for விகிதம் reached a module name, a table and two columns before a
reader caught it. It works by running each name back through the scheme: a
Latin letter still sitting inside Tamil output is a letter the scheme never
assigned. English names that trip it are listed in `ALLOW` in that file.

Transliterate a word by hand:

```bash
python3 scripts/transliterate.py வணக்கம் நிதி நாணயம்
```

Regenerate the keyword reference after changing the token list:

```bash
python3 scripts/generate_keywords.py
git diff --stat docs/reference/KEYWORDS.md      # expect no diff if unchanged
```

---

## 10. Full check, in one go

```bash
cd etamil_compiler && cargo build --release && cargo test && cd .. \
  && ./scripts/run_examples.sh \
  && python3 scripts/transliterate.py --check \
  && python3 scripts/check_names.py --check ; echo "exit: $?"
```

All of it should exit 0.

## 11. Build and test the Linux package

The downloadable package is what an eTamil user actually installs, so it is worth
testing as one. Build it with musl — the default gnu target links against this
machine's glibc, and the resulting binary will refuse to start on an older
distribution:

```bash
rustup target add x86_64-unknown-linux-musl
sudo apt install musl-tools
TARGET=x86_64-unknown-linux-musl ./packaging/build.sh
```

That writes `dist/etamil-linux-x64.tar.gz` and its `.sha256`.

Test it from a clean extraction, **not** from the repository — the repository has
`nUlakam/` sitting right there, which will hide a packaging mistake:

```bash
cd /tmp && rm -rf pkgtest && mkdir pkgtest && cd pkgtest
tar -xzf ~/…/dist/etamil-linux-x64.tar.gz
cd etamil-linux-x64
./etamil --version
file ./etamil                     # expect: statically linked
ldd  ./etamil                     # expect: not a dynamic executable
printf 'இறக்கு "nUlakam/paNam/paNam.qmz";\nஅச்சு ரூபாய்(12345678.50);\n' > /tmp/t.qmz
./etamil --vm /tmp/t.qmz           # expect ₹1,23,45,678.50
```

Then the installer itself, which needs no root:

```bash
./install.sh
exec "$SHELL" -l                   # pick up PATH and ETAMIL_PATH
cd /tmp && etamil --version
etamil --vm /tmp/t.qmz             # resolves nUlakam via ETAMIL_PATH, not cwd
```

That last line is the one that matters: it proves `இறக்கு` finds the standard
library from an unrelated directory. To undo it all:

```bash
rm -rf ~/.local/bin/etamil ~/.local/lib/etamil
```

Release steps and the reasoning behind the archive names are in
[`packaging/README.md`](packaging/README.md).

---

## Developing on Windows without the MSVC linker

Rust's default Windows toolchain needs `link.exe` from Visual Studio Build
Tools. Without it **nothing compiles** — not even `cargo check`, because
proc-macro crates are built and linked as DLLs during the build.

If installing Build Tools is not an option, the GNU toolchain ships its own
linker and can build the compiler core:

```bash
rustup toolchain install stable-x86_64-pc-windows-gnu
cargo +stable-x86_64-pc-windows-gnu test
```

**What this does and does not cover.** The core — `lexer`, `parser`,
`module`, `vm`, `db` (trait only) — is pure Rust and builds fine. These do
**not**:

| Crate | Blocked because |
|---|---|
| `ring` (via `jsonwebtoken`) | compiles C; needs a MinGW gcc |
| `rusqlite` | compiles bundled SQLite |
| `chrono` | needs `dlltool.exe`, absent from rustup's MinGW |

So `src/http/`, `src/fileio/` and the SQLite driver cannot be tested this
way. Build them on Linux, or install Build Tools.

This is why the date primitives in `vm/interpreter.rs` use a hand-written
civil-calendar conversion rather than `chrono`: reaching for the obvious
crate would have made the language's own date handling untestable on the
machine it was written on.

To test the core against a checkout without touching the real crate, a shim
crate that pulls the modules in by `#[path]` works well:

```toml
# Cargo.toml of a scratch crate named etamil_compiler
[dependencies]
logos = "0.16.1"
rust_decimal = "1.37"
unicode-segmentation = "1.11"

[[test]]
name = "language_tests"
path = "/abs/path/to/etamil_compiler/tests/language_tests.rs"
```

```rust
// src/lib.rs
#[path = "/abs/path/to/etamil_compiler/src/lexer.rs"]  pub mod lexer;
#[path = "/abs/path/to/etamil_compiler/src/parser.rs"] pub mod parser;
#[path = "/abs/path/to/etamil_compiler/src/vm/mod.rs"] pub mod vm;
#[path = "/abs/path/to/etamil_compiler/src/module.rs"] pub mod module;
#[path = "/abs/path/to/etamil_compiler/src/db/mod.rs"] pub mod db;
```

Set `ETAMIL_STDLIB` to the `nUlakam` directory so the standard-library tests
can find it.

---

## Troubleshooting

**`linker 'cc' not found`** — install `build-essential`.

**`edition2024 is required`** — Rust is older than 1.85. Run `rustup update`.

**`error: could not find native static library 'Polly'`** — LLVM 18 dev
packages are missing or `LLVM_SYS_180_PREFIX` is unset. Only affects
`--features llvm`.

**`cannot open module 'nUlakam/...'`** — set `ETAMIL_PATH` to the repository
root, or run from a directory where the relative path resolves. `இறக்கு`
searches beside the importing file, then `ETAMIL_PATH`, then next to the
binary.

**A file example says `cannot read`** — run it from a writable directory; the
I/O examples create their input files in the current directory.

**`bad interpreter: /bin/bash^M`** — the checkout has CRLF line endings.
`.gitattributes` should prevent this; if it happens, run
`git config core.autocrlf false && git rm --cached -r . && git reset --hard`.

---

## What to report back

If something fails, the useful details are:

1. `rustc --version` and `uname -a`
2. the exact command and its full output
3. for a test failure, `cargo test -- --nocapture` output for that test
4. whether `cargo test` alone passes, since that isolates compiler bugs from
   example or environment problems
