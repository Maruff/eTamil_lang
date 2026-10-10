# eTamil Programming Language

**A programming language whose vocabulary is Tamil, aimed at Indian FinTech.**

[![CI](https://github.com/Maruff/eTamil_lang/actions/workflows/ci.yml/badge.svg)](https://github.com/Maruff/eTamil_lang/actions/workflows/ci.yml)

---

## What is eTamil?

eTamil lets you write programs in Tamil. It is not an English language with translated keywords: finance is built into the vocabulary, so `வரவு` (credit), `பற்று` (debit), `வரி` (tax) and `இருப்புநிலை` (balance sheet) are part of the language itself.

Every keyword accepts three spellings that mean exactly the same thing:

```etamil
எண் வருவாய் = 100000;     // Tamil script
eN varuvAy = 100000;       // romanized (ezuqqu scheme)
```

That is the core idea: Tamil semantics you can type on a plain keyboard.

The syntax — `{ }` for blocks, `;` to end a statement, and much of the rest — was evaluated against C, C++ and Rust before it was finalised, and Rust shaped the semantics too: a failure is a value, not an exception. A few keywords were adopted from [Ezhil](https://en.wikipedia.org/wiki/Ezhil_(programming_language)), the earlier Tamil programming language.

📖 **[Read the user manual at etamil.in/manual](https://etamil.in/manual/)** —
installation through to a database-backed HTTP service, in
[English](https://etamil.in/manual/) and
[தமிழ்](https://etamil.in/ta/manual/).

### Real example

```etamil
// Income tax calculator
எண் வருவாய்;
அச்சு "Enter income: ";
உள்ளிடு வருவாய்;
வரி = 20%;

(வருவாய் > 800000) எனில் {
    அச்சு "High Tax Bracket";
    அச்சு (வருவாய் - 800000) * வரி;
}
இன்றேல் {
    அச்சு "Low Tax Bracket (No Tax)";
}
```

```bash
echo "950000" | etamil --vm examples/basic_samples/example.qmz
```

---

## Project status

eTamil runs backend programs today: functions, collections, error handling, modules, a SQLite database layer, a concurrent HTTP server with routing, and an accounting framework written in the language itself. This table is the honest state of the code.

| Area | Status | Notes |
|---|---|---|
| Lexer (Tamil / romanized / English keywords) | ✅ Working | 203 tokens across 545 spellings; errors carry line and column |
| Variables, arithmetic, percentages, strings | ✅ Working | |
| Comparisons, `எனில்` / `இன்றேல்`, `சுற்று` loops | ✅ Working | |
| Logical `மற்றும்` / `அல்லது` / `இல்லை` | ✅ Working | both sides always evaluated — no short-circuiting |
| File I/O and CSV row counting | ✅ Working | in the VM (`--vm`) |
| VM bytecode executor | ✅ Working | |
| Functions (`செயல்` / `திரும்பு`) | ✅ Working | parameters, returns, local scope, recursion |
| Arrays (`[…]`) and records (`{…}`) | ✅ Working | indexing, field access, assignment |
| Fixed bindings (`நிலை`) | ✅ Working | `நிலை எல்லை = 250000;` is bound once: assigning it again, or changing an element or a field of it, is refused before the program runs. Opt-in — a plain assignment is as mutable as ever — and `நிலை` is still an ordinary name wherever it does not begin a binding |
| Functions as values | ✅ Working | `ச = இரட்டி;`, a `செயல்` passed to another, kept in an array or a record, returned, or written where a value goes: `செயல்(x) { திரும்பு x * 2; }`. One written inside a function carries copies of the locals it uses, as a Rust `move` closure does. A builtin is a value too |
| Record shapes (`வடிவம்`) | ✅ Working | `வடிவம் கடன் { எண் அசல், எண் வீதம் }` names a kind of record. `கடன்{…}` must give every field and no other, each holding what it was declared to; a mistyped field is refused before the program runs; a value that only exists at runtime is checked when the record is made. `கடன்(பதிவு)` makes a database row into one, as a result |
| Methods | ✅ Working | a `செயல்` written inside a `வடிவம்`. With `இது` first it is called on a record, `க.தவணை()`, and cannot change it — Rust's `&self`; a change is a new record, `கடன்{அசல்: 1, ..இது}`. Without it, it is called on the shape, `கடன்.புதிது(…)` |
| Iteration (`ஒவ்வொரு … இல்`) | ✅ Working | arrays, records, strings |
| Results (`சரி` / `தவறு` / `?`) | ✅ Working | Rust semantics; failure is a value, not an exception |
| Modules (`இறக்கு`) | ✅ Working | resolves beside the file, then `ETAMIL_PATH` |
| Decimal arithmetic | ✅ Working | fixed point, not `f64` |
| Standard library (`nUlakam/`) | ✅ Working | strings, math, arrays, money, JSON, encoding, and the document renderer — **written in eTamil** |
| Search and replace (`மாற்று`, `பிரி`, `ஒன்றிணை`) | ✅ Working | host builtins. A separator only matches on a letter boundary, so `பிரி("கா", "ா")` does not cut a letter in half. These were `nUlakam` functions that re-read the string letter by letter — 14 seconds over 8 KB — which put a document-sized string out of reach |
| Whole-file write (`கோப்பு_சேமி`) | ✅ Working | exactly the bytes of the string, no trailing newline, answering with the byte count as a result. `கோப்பு_எழுது` still appends a line, which is what a CSV row wants |
| ODF and OOXML packages (`பொதி_படி`, `பொதி_மாற்று`) | ✅ Working | `.odt`, `.ods`, `.docx` and `.xlsx` are zip archives of XML. A template is copied entry by entry with the text entry swapped, so pictures arrive byte-for-byte and the ODF `mimetype` rule — first, and stored uncompressed — is kept. Replacing an entry that is not there is refused rather than written |
| Running another program (`கட்டளை_ஓட்டு`) | ✅ Working | for the PDF, which comes out of LibreOffice. Deny by default: nothing runs unless `ETAMIL_EXEC_ALLOW` names it, arguments are a list and never reach a shell, and a program that will not finish is killed |
| Sending a file (`பதில்_கோப்பு`) | ✅ Working | a response body that is not text — a PDF, an .odt, a picture. The language names the file and the server reads it, because a body built as a `சரம்` loses every byte that is not valid UTF-8. Content-Length counts what is actually sent; a missing file is a result, not an empty 200 |
| File uploads (`request_files`, `பதிவேற்றம்_சேமி`) | ✅ Working | `multipart/form-data`, parsed over bytes. The request body is no longer decoded to text before parsing, which used to replace every byte of an upload that was not valid UTF-8. Text fields arrive in `request_fields`; files stay as bytes and the handler saves the one it wants, so nothing is spooled to a temporary file for someone to clean up |
| Single sign-on (`சீட்டு_தலைப்பு`, `சீட்டு_பொதுச்_சரிபார்`) | ✅ Working | RS256 against a public key from an identity provider's JWKS, for Entra ID and the like. Fetching the JWKS, choosing the key and caching it are ordinary work for `வலை_பெறு` and `nUlakam/vativam/jEcAZ.qmz`; only reading a token's header and checking a signature live in the host. The issuer and the audience are required arguments, not options — a token a provider really signed, for a different application, is a real token and is still refused |
| Depreciation, payroll and tax rates (`nUlakam/kaNakkiyal/`) | ✅ Working | straight-line and written-down depreciation with schedules that close exactly, block-of-assets, payroll with the ceiling-versus-eligibility-limit distinction, and one marginal slab engine for income tax and professional tax. Rates live in an effective-dated table — every lookup takes the date it is asked about, and **no rate is seeded** |
| Insurance and customs (`nUlakam/kAppItu/`, `nUlakam/cuwkam/`) | ✅ Working | premium, claim settlement and the **average clause** — under-insurance scales back even a partial loss, and the order of a settlement is not interchangeable. Customs centres on the **duty cascade**: the surcharge is on the duty and IGST is on the duty-inclusive value, and getting that wrong understates the tax. E-way bill validity rounds up, because rounding down expires a bill while the lorry is moving |
| Accounting framework | ✅ Working | double entry, GST, three statements — **written in eTamil** |
| SQLite (`தளம்_இணை` etc.) | ✅ Working | parameterised queries only; rows return as an array of records |
| A function of your program, called by a SQLite query (`தளம்_செயல்_பதிவு`) | ✅ Working | once per row, in a fresh VM under a step limit; a function that could reach outside the database is refused when registered. Exact text by default, so compare with the `"numeric"` mode (see `examples/db_samples/qaLam_ceyal_paqivu.qmz`). SQLite only; PostgreSQL and MySQL say so |
| A database write that can fail (`தளம்_செய்_முயற்சி`, `தளம்_வினா_முயற்சி`) | ✅ Working | the statements insist; these attempt and answer a `முடிவு`. A duplicate key was an unrecoverable error — under the server it took the handler with it and became a 500, when a 409 was the answer. And the driver's row count was thrown away, so an UPDATE matching nothing looked exactly like one that matched a row |
| Connection reuse | ✅ Working | `தளம்_இணை` borrows from a process-wide idle cache instead of reconnecting per request; leases are exclusive, so transactions stay isolated. `ETAMIL_DB_IDLE` caps it. An in-memory database is **never** pooled: SQLite gives every connection to `:memory:` its own database, so caching one by that name handed unrelated code the same database |
| PostgreSQL | ✅ Working | `--features postgres`; money as native `NUMERIC`, so a text column stays text — unlike SQLite, where decimals are stored as text |
| MySQL / MariaDB | ✅ Live verified | `--features mysql`; the live sample passes with `ETAMIL_TEST_MYSQL=1 ./scripts/run_examples.sh`; setup details are in `TESTING.md` |
| HTTP server (`--server`) | ✅ Working | worker pool; `வழி` routes with `:id` path parameters, query params, headers and request bodies; `பதில்` responses |
| LLVM backend (`--llvm`) | 🟡 Expressions complete; I/O statements refused | Linux/macOS, `--features llvm`. Every value in the emitted IR is a handle into an arena in `src/runtime.rs`, and every operation on one is a call into the cdylib Cargo already builds — so decimals are **exact** (`1 / 3` prints all twenty-eight digits, as on the VM), formatting cannot drift from the VM's, and all fifty-nine builtins are reachable at once. Strings, arrays, records, results, booleans and `இன்மை` all have a representation, and so do function values, shaped records and methods: each compiled function has an entry the runtime can call through a value, and a shape's checks are `vm::shape`'s, shared with the VM. What is still refused is *statements*: files, databases, HTTP, routes. The IR is therefore not self-contained — it links `-letamil_compiler`. `docs/llvm-backend-gaps.md` explains the design and `scripts/run_parity.sh` measures it against the VM |
| Response headers | ✅ Working | `பதில் 200, உடல், {"Content-Type": "text/html"}` — an ordinary record; defaults to JSON when omitted |
| JSON (`nUlakam/vativam/jEcAZ.qmz`) | ✅ Working | `ஜேசான்_ஆக்கு` / `ஜேசான்_படி` — **written in eTamil**; `\uXXXX` escapes are not decoded |
| Scheduled blocks (`இடைவெளி`) | ✅ Working | `இடைவெளி 3600 { … }` under either server; the number is the gap *between* runs, so a slow job runs late rather than twice at once |
| Bytes | ✅ Working | `பைட்டுகள்` / `பைட்டுச்_சரம்` — a byte array is an ordinary array of numbers, not a new value type |
| base64 and hex (`nUlakam/vativam/kuRiyAkkam.qmz`) | ✅ Working | `அறுபத்துநான்கு_ஆக்கு` `அறுபத்துநான்கு_படி` `பதினாறு_ஆக்கு` `பதினாறு_படி` — **written in eTamil** |
| Signing with a key only one side holds (ECDSA P-256) | ✅ Working | `வளைவு_சாவிகள்` `வளைவு_கையொப்பம்` `வளைவு_சரிபார்` `வளைவு_பொதுச்சாவி`. HMAC proves a message came from someone holding the same secret you do, so either side could have written it; this is signed with a private key and checked with a public one. SHA-256 digest, ASN.1 DER signature, keys as hex — the shapes Hyperledger Fabric MSP and X.509 expect. A signature that does not verify answers false; a key that is not a key is a தவறு |
| Signing (HMAC-SHA256) | ✅ Working | `கையொப்பம்` / `கையொப்பம்_சரியா` — verify a signed webhook; the comparison is constant-time |
| Mutual TLS (client certificates) | ✅ Working | `ETAMIL_TLS_CERT`, `ETAMIL_TLS_KEY`, `ETAMIL_TLS_CA`. Ordinary HTTPS proves the server is who it claims to be; a bank wants the other direction too, and will not discuss an account with a caller it cannot identify. PKCS#8, SEC1 or PKCS#1 keys. The CA and the identity are independent — trust a private root without presenting anything, or both. Half an identity is refused rather than sent anonymously |
| UPI addresses, links and states (`nUlakam/upi/`) | ✅ Working | VPA checking, the `upi://` pay link, and amounts in the only form UPI takes. The payment state machine holds one rule above all: **pending is not failure** — only a settled success authorises shipping, and a late callback cannot rewrite a settled payment. Moving money still needs a PSP and NPCI certification, which no library provides |
| Hyperledger Fabric (`nUlakam/cawkili/fabric.qmz`) | ✅ Working | through a REST gateway, not gRPC. Query and submit, with the read-write conflict told apart from a chaincode refusal and retried; the refusal is not. Identity signs with `வளைவு_கையொப்பம்` — P-256 is what Fabric's default MSP uses |
| Outbound HTTP | ✅ Working | `--features http-client` (on by default); `வலை_பெறு` `வலை_பதி` `வலை_அனுப்பு`. A non-2xx is a result, not a failure |
| Authentication | ✅ Working | bcrypt and JWT in the host; `கடவுச்சொல்_மறை` `கடவுச்சொல்_சரியா` `சீட்டு_ஆக்கு` `சீட்டு_சரிபார்`. Set `ETAMIL_JWT_SECRET` |
| String escapes | ✅ Working | `\n` `\t` `\r` `\"` `\\`; an unknown escape keeps both characters |
| JSON responses (`ஜேசான்_உரை`) | ✅ Working | `ஜேசான்_உரை உடல், 201` — puts the JSON content type on the response for you, and defaults to 200 when the status is omitted. The body must already be text: encoding a record here would need a second JSON encoder beside `nUlakam`'s `ஜேசான்_ஆக்கு`, and two encoders are two answers to one question |
| Redis | ✅ Working | `ரெடிஸ்_இணை` `ரெடிஸ்_கட்டளை` `ரெடிஸ்_பிரி`, with RESP implemented here rather than taken from a crate. One generic command, because that is the shape of Redis — every command works. Arguments are length-prefixed, so a value holding CRLF cannot become a second command; a missing key is nil and not `""`. Not pooled: Redis keeps per-connection state |
| MongoDB | ✅ Working | `--features mongodb`. A document is a `பொருள்` and a collection of them an array of records, so the value model was already document-shaped. **Money is stored as `Decimal128`, never a double** — a balance written as a double is not reliably the balance that comes back. `மொங்கோ_கட்டளை` is `runCommand`, so anything the server takes works. About seventy crates, fewer than `mysql` already costs, and none of it in the default build |
| Async HTTP server (`--async`) | ✅ Working | tokio accept loop, handlers on the blocking pool; the VM stays synchronous |
| Parse error positions | ✅ Working | every error carries a line and column, bilingually |
| Type checking | ✅ Working | a declared type is enforced, with a position; deliberately narrow — no rule the rest of the language does not follow |
| Tests in eTamil (`nUlakam/cOqaZY/cOqaZY.qmz`) | ✅ Working | assertions, a summary, and a non-zero exit when anything fails, so a suite gates CI. `kaNakkiyal/vari_cOqaZY.qmz` is fifteen of them about GST arithmetic. `வெளியேறு(நிலை)` is what ends the process with a status |
| Interactive shell (`--repl`) | ✅ Working | variables persist between lines, a செயல் can be typed across several, `இறக்கு` works, and a bare expression is answered rather than refused — `0.1 + 0.2` prints `0.3`. `:vars` shows what the session holds |
| VS Code extension | ✅ Working | [`Maruff/eTamil_vsCode`](https://github.com/Maruff/eTamil_vsCode) — highlighting for all 203 keywords in every spelling, completions for 98 builtins and 940 `nUlakam` functions, and errors from `--check` as you type. Carries the compiler, the standard library, the examples and the eTamil font, so installing it is the whole installation. Grammar and completion data are **generated** from `lexer.rs`; CI fails if they drift |

Anything marked "not implemented" **fails with an explicit message** rather than quietly doing nothing. That is deliberate: silent no-ops in a tax calculator are worse than errors.

### One thing to know before you write much

**A name is stored exactly as you wrote it**, including when the word you chose is also a keyword: `வங்கி = 5` creates a variable called `வங்கி`, and `{வரி: 100}` produces the field `வரி`. Names used to be filed under their English token name — `Bank`, `Tax` — which anglicised a Tamil author's chosen words and put English field names into Tamil output.

The consequence, which is a real change in meaning: `{வரி: 1}` and `{vari: 1}` are now **different** fields, and `வருவாய்` and `varuvAy` are different variables. Pick one spelling per program. A field name is data — what you typed — not a language construct.

Type keywords and SQL clause keywords remain **hard reserved** and cannot be names at all: `எண்`, `சொல்`, `அணி`, `வரிசை`, `விதி`, `இடம்`, `உள்`, `வெளி`, `குழு`, `சேர்`. Financial keywords are *not* reserved — `தொகை` is a perfectly good name for an amount. [KEYWORDS.md](docs/reference/KEYWORDS.md) lists every one.

### Errors say where

```
✗ வரி 3, நெடுவரிசை 1: ';' எதிர்பார்க்கப்பட்டது, 'அச்சு' கிடைத்தது
  (line 3, column 1: expected ';', found 'அச்சு')
```

Columns count written letters, not bytes, so the position is the one you would point at on the screen.

### Money is exact

Every number in eTamil is a fixed-point decimal, so the arithmetic a tax program actually performs comes out right:

```etamil
அச்சு 0.1 + 0.2;      // 0.3        — not 0.30000000000000004
அச்சு 99.99 * 3;      // 299.97     — not 299.96999999999997
அச்சு 18%;            // 0.18       exactly
```

Equality is exact too. Division keeps full precision rather than rounding at each step, because Indian tax computation rounds once at the end — round explicitly when you need to.

---

## Installation

### Prebuilt packages

| Platform | Download |
|---|---|
| **Windows** x64 | [etamil-windows-x64.zip](https://github.com/Maruff/eTamil_lang/releases/latest/download/etamil-windows-x64.zip) |
| **Linux** x64 | [etamil-linux-x64.tar.gz](https://github.com/Maruff/eTamil_lang/releases/latest/download/etamil-linux-x64.tar.gz) |
| **Linux** arm64 (Raspberry Pi 4/5) | [etamil-linux-arm64.tar.gz](https://github.com/Maruff/eTamil_lang/releases/latest/download/etamil-linux-arm64.tar.gz) |
| **Android** arm64 (in [qos](https://qos.ae) or Termux) | [etamil-android-arm64.tar.gz](https://github.com/Maruff/eTamil_lang/releases/latest/download/etamil-android-arm64.tar.gz) |
| **macOS** Apple Silicon | [etamil-macos-arm64.tar.gz](https://github.com/Maruff/eTamil_lang/releases/latest/download/etamil-macos-arm64.tar.gz) |
| **macOS** Intel | [etamil-macos-x64.tar.gz](https://github.com/Maruff/eTamil_lang/releases/latest/download/etamil-macos-x64.tar.gz) |

Every link points at the latest release, so it stays correct across versions.
Each archive is published with a `.sha256` beside it — see
[all releases](https://github.com/Maruff/eTamil_lang/releases/latest).

The archive holds the compiler, `nUlakam/` (the eTamil standard library) and the
examples. The install script copies them into place, puts `etamil` on your `PATH`
and sets `ETAMIL_PATH` so `இறக்கு "nUlakam/paNam/paNam.qmz"` resolves from any
directory. No installer needs administrator rights.

**Windows (PowerShell)**
```powershell
Expand-Archive etamil-windows-x64.zip -DestinationPath .
.\etamil-windows-x64\install.ps1
```

**Linux** — use `arm64` for a Raspberry Pi 4 or 5 running 64-bit Raspberry Pi
OS (or any other aarch64 machine), `x64` otherwise. `uname -m` tells you which:
`aarch64` or `x86_64`. A 32-bit Raspberry Pi OS reports `armv7l`, and there is
no package for it.

```bash
tar -xzf etamil-linux-x64.tar.gz
./etamil-linux-x64/install.sh
```

**macOS** — use `arm64` for Apple Silicon (M1 and later), `x64` for Intel.
`uname -m` tells you which: `arm64` or `x86_64`.

```bash
tar -xzf etamil-macos-arm64.tar.gz
./etamil-macos-arm64/install.sh
```

macOS quarantines anything downloaded from a browser, and these binaries are
not notarized, so Gatekeeper will refuse the first run with "cannot be opened
because the developer cannot be verified". Clear the quarantine flag once:

```bash
xattr -dr com.apple.quarantine ~/.local/lib/etamil
```

Open a new terminal afterwards — a shell that is already running does not see a
`PATH` change — then `etamil --version`.

There is nothing else to install. The Windows binary links the C runtime
statically, so it does not need the Visual C++ Redistributable; the Linux binaries
are built against musl, so each is one static ELF that does not depend on the build
machine's glibc. The packaged builds include the PostgreSQL and MySQL drivers,
which a downloaded binary cannot have added to it afterwards; the LLVM backend
is not included, because it needs LLVM installed on the machine that runs it.

Uninstalling is deleting a directory: `%LOCALAPPDATA%\Programs\eTamil` on
Windows, `~/.local/{bin/etamil,lib/etamil}` on Linux and macOS.

To build the packages yourself, see [`packaging/`](packaging/).

### Build from source (all platforms)

Needed for the optional database drivers, the LLVM backend, or work on the
compiler itself.

Requires **Rust 1.88+** (edition 2024) and a C toolchain, since the bundled
SQLite and the crypto crates compile C:

- **Windows** — Visual Studio Build Tools with the "Desktop development with
  C++" workload. The MSVC linker is not optional: without it even
  `cargo check` fails, because proc-macro crates link as DLLs.
- **Linux / macOS** — a working `cc` (build-essential, or the Xcode command
  line tools).

```bash
git clone https://github.com/Maruff/eTamil_lang.git
cd etamil_compiler/etamil_compiler
cargo build --release
```

The binary is `target/release/etamil` (`etamil.exe` on Windows). Put it on your `PATH`:

**Linux / macOS**
```bash
sudo cp target/release/etamil /usr/local/bin/etamil
```

**Windows (PowerShell)**
```powershell
Copy-Item "target\release\etamil.exe" "$env:USERPROFILE\bin\etamil.exe"
```

Verify:

```bash
etamil --version
```

### Optional: PostgreSQL and MySQL

SQLite is built in. The others are behind features, so a default build does
not carry their dependencies:

```bash
cargo build --release --features postgres,mysql
```

```etamil
தளம்_இணை போச்குரசீகுல், "postgres://user:pass@localhost:5432/kaNakku";
தளம்_வினா "SELECT peyar, qokY FROM pativukaL WHERE vakY = $1", ["வரவு"], வரவுகள்;
```

PostgreSQL placeholders are `$1, $2, …`; SQLite and MySQL use `?`. Both of the
server backends keep money in the database's own exact decimal type, so a text
column stays text on the way back — where the SQLite backend, which stores
decimals as text, hands back a number. PostgreSQL also folds unquoted
identifiers to lower case: write `"qokY"` if you want that column name back as
you spelled it.

```etamil
தளம்_இணை மைசீகுல், "mysql://root@localhost:3306/kaNakku";
தளம்_வினா "SELECT peyar, qokY FROM pativukaL WHERE vakY = ?", ["வரவு"], வரவுகள்;
```

`examples/db_samples/mYcIkul_qaLam.qmz` checks the things worth checking on a
real server — exact `DECIMAL` sums, an integer key bound from a number, dates
as ISO text, `NULL` as `இன்மை`, and an injection payload staying inert. It
needs a database, so the example runner skips it unless you opt in:

```bash
ETAMIL_TEST_MYSQL=1 ./scripts/run_examples.sh
```

### Optional: LLVM backend

Only needed for `--llvm`, and only available on Linux/macOS:

```bash
cargo build --release --features llvm
./scripts/run_parity.sh            # does it still agree with the VM?
```

`run_parity.sh` runs every example under both backends. A refusal is expected
and only counted; a program the backend accepts and then answers differently
from the VM is a failure, because that is the case nobody would notice from the
outside.

To compile one program by hand, note that the emitted IR is **not**
self-contained — it calls into the runtime that gives it exact decimals and the
builtins, which is the same cdylib Cargo just built:

```bash
etamil --llvm myprogram.qmz
clang output.ll -o myprogram \
      -L etamil_compiler/target/release -letamil_compiler \
      -Wl,-rpath,etamil_compiler/target/release -lm
```

The `-rpath` is so the finished program can still find the library when it runs.
A compiled program therefore ships with `libetamil_compiler.so` beside it.

On a machine without LLVM the backend cannot be built, but it can still be
type-checked — which is what makes it editable there at all:

```bash
python scripts/check_llvm_backend.py
```

---

## Quick start

```bash
echo 'அச்சு "வணக்கம் உலகம்!";' > hello.qmz
etamil --vm hello.qmz
```

On Windows, write the file as UTF-8:

```powershell
'அச்சு "வணக்கம் உலகம்!";' | Out-File hello.qmz -Encoding UTF8
etamil --vm hello.qmz
```

---

## Language reference

### Variables and types

```etamil
எண் age = 25;          // number
எண் price = 99.99;     // fixed-point decimal; no separate int/float yet
எண் rate = 15%;        // percentage literal -> exactly 0.15
சொல் name = "Ravi";    // string
```

A declared type is **enforced**, and a later assignment is held to it too:

```
✗ வரி 2, நெடுவரிசை 6: 'கொடியா' ஈர்ம (Irma, a boolean) என அறிவிக்கப்பட்டது,
  ஆனால் ஒரு அணி (an array) வழங்கப்பட்டது
  (line 2, column 6: 'கொடியா' is declared a boolean, but was given an array)
```

The checker is deliberately narrow: it holds you to what you declared and
states no rule the rest of the language does not follow. A number satisfies
`சொல்`, because every value renders as text and `உள்ளிடு` hands back text that
is routinely compared with numbers. A call, an index and a field access make no
claim, because functions have no declared signatures yet — silence there is the
absence of a claim, not approval.

### Input and output

```etamil
எண் வருவாய்;
அச்சு "Enter income: ";
உள்ளிடு வருவாய்;
அச்சு "Income: " & வருவாய்;   // & concatenates
```

Input always arrives as text and is converted when compared or used in arithmetic.

### Conditionals and loops

```etamil
(வருவாய் > 800000) எனில் {
    அச்சு "High";
}
இன்றேல் {
    அச்சு "Low";
}

எண் i = 0;
(i < 3) சுற்று {
    அச்சு i;
    i = i + 1;
}
```

### Bindings that do not change

```etamil
நிலை எண் அடிப்படை_விலக்கு = 300000;
நிலை அடுக்குகள் = [0, 5%, 20%];

அடிப்படை_விலக்கு = 250000;   // refused: cannot assign twice
அடுக்குகள்[1] = 10%;          // refused: no part of it changes either
```

`நிலை` is opt-in and follows Rust's `let`: bound once, for the rest of its
scope, with no part of the value changed. Values are copied rather than shared,
so `நகல் = அடுக்குகள்;` is an ordinary variable and changing it cannot reach the
original. A parameter can be fixed too — `செயல் வரி(நிலை எண் தொகை)`.

### Functions as values

```etamil
செயல் வடிகட்டு(அணி பட்டியல், செயல் ஏற்பது) அணி {
    விடை = [];
    ஒவ்வொரு உருப்படி இல் பட்டியல் {
        (ஏற்பது(உருப்படி)) எனில் { விடை = இணை(விடை, உருப்படி); }
    }
    திரும்பு விடை;
}

பெரியவை = வடிகட்டு([1200, 450, 3100], செயல்(எ) { திரும்பு எ >= 1000; });

செயல் வரி_விதி(வீதம்) செயல் {
    திரும்பு செயல்(தொகை) { திரும்பு தொகை * வீதம்; };   // carries வீதம்
}
ஜிஎஸ்டி = வரி_விதி(18%);
அச்சு ஜிஎஸ்டி(5000);                                   // 900
```

A named `செயல்` is a value by its name, and so is a builtin: `ஒவ்வொன்றுக்கும்(பட்டியல், நீளம்)`.
One written as a value carries copies of the enclosing function's locals it
reads, taken when it is made; at the top level it reads globals when it runs,
as a named one always has.

### Record shapes and methods

```etamil
வடிவம் கடன் {
    சொல் பெயர்,
    எண் அசல்,
    எண் ஆண்டு_வீதம்,

    செயல் புதிது(பெயர், அசல், ஆண்டு_வீதம்) கடன் {
        திரும்பு கடன்{பெயர்: பெயர், அசல்: அசல், ஆண்டு_வீதம்: ஆண்டு_வீதம்};
    }
    செயல் ஆண்டு_வட்டி(இது) எண் {
        திரும்பு இது.அசல் * இது.ஆண்டு_வீதம்;
    }
    செயல் முன்செலுத்து(இது, எண் தொகை) கடன் {
        திரும்பு கடன்{அசல்: இது.அசல் - தொகை, ..இது};
    }
}

நிலை க = கடன்.புதிது("ராஜா", 500000, 9%);
அச்சு க.ஆண்டு_வட்டி();                      // 45000
அச்சு க.முன்செலுத்து(100000).ஆண்டு_வட்டி();  // 36000
```

A shaped record is checked against its `வடிவம்`: `கடன்{…}` must give every
field and no other, each holding what it was declared to, and `க.அசால்` is
refused before the program runs with the fields `கடன்` does have. A value the
checker cannot see — a number typed in, a field set by a computed key — is
checked when it arrives. `கடன்(பதிவு)` makes a plain record, such as a
database row, into a `கடன்`, and answers a `தவறு` when it does not fit.

A method's `இது` is the record it was called on, and it cannot be changed, as
Rust's `&self` cannot: a method that means to change a record returns a new one,
`..இது` filling in the fields it does not write. A method without `இது` is
called on the shape, and one with it can be too, the record first:
`கடன்.ஆண்டு_வட்டி(க)`. There is no inheritance.

### Operators

| Kind | Operators |
|---|---|
| Arithmetic | `+` `-` `*` `/` (and unary `-`) |
| Comparison | `==` `!=` `<` `<=` `>` `>=` |
| Logical | `மற்றும்` / `maRRum` / `_and`, `அல்லது` / `allaqu` / `_or`, `இல்லை` / `illY` / `_not` |
| String | `&` |

Precedence, loosest first: `or` → `and` → `not` → comparison → `+ -` → `* /`.

```etamil
(வருவாய் > 800000 மற்றும் வயது < 60) எனில் {
    அச்சு "Taxable";
}
```

### File I/O

```etamil
கோப்பு_திற "output.txt", "write";     // opening for write truncates
கோப்பு_எழுது "output.txt", "வணக்கம்";  // subsequent writes append
கோப்பு_மூடு "output.txt";

கோப்பு_படி "output.txt", data;        // read whole file into a variable
அச்சு data;
```

CSV row counting (excludes the header):

```etamil
தரவுரை_படி "students.csv", total;
அச்சு total;
```

---

## Tamil letter mapping (ezuqqu scheme)

12 vowels + 18 consonants + ஃ + 5 borrowed letters.

| Tamil | eTamil | Transliteration | ISO 15919 |
|-------|--------|-----------------|-----------|
| அ | a | a | a |
| ஆ | A | aa | ā |
| இ | i | i | i |
| ஈ | I | ii | ī |
| உ | u | u | u |
| ஊ | U | uu | ū |
| எ | e | e | e |
| ஏ | E | ee | ē |
| ஐ | Y | ai | ai |
| ஒ | o | o | o |
| ஓ | O | oo | ō |
| ஔ | V | au | au |
| க | k | k | k |
| ங | w | ng | ṅ |
| ச | c | ch | c |
| ஞ | W | nj | ñ |
| ட | t | t | ṭ |
| ண | N | nn | ṇ |
| த | q | th | t |
| ந | n | n | n |
| ப | p | p | p |
| ம | m | m | m |
| ய | y | y | y |
| ர | r | r | r |
| ல | l | l | l |
| வ | v | v | v |
| ழ | z | zh | ḻ |
| ள | L | ll | ḷ |
| ற | R | rr | ṟ |
| ன | Z | n | ṉ |
| ஃ | h | h | ḵ |
| ஹ | H | h | h |
| ஜ | j | j | j |
| ஷ | S | sh | ṣ |
| ஸ | s | s | s |
| க்ஷ | x | ksh | kṣ |

### The n-family letters

Tamil has three distinct nasals that English collapses into one `n`. eTamil keeps them apart:

| Tamil | eTamil | ISO 15919 | Example |
|---|---|---|---|
| ண | `N` | ṇ | `எண்` → `eN` |
| ந | `n` | n | `நிதி` → `niqi` |
| ன | `Z` | ṉ | `பயன்` → `payaZ` |

`Z` was free — ழ is lowercase `z` — so it takes ன, leaving `N` unambiguously ண and `n` unambiguously ந. Words containing more than one show the distinction clearly: `நாணயம்` → `nANayam` (ந-ண), `பின்னம்` → `piZZam` (two ன), `வருமானம்` → `varumAZam`.

This replaces an earlier scheme in which ந and ன both used `n` and a few keywords spelled ந as `N`. **Romanized source written before this change needs updating** — Tamil-script source is unaffected.

See the [Tamil Letter Equivalents Guide](docs/reference/COMPILER_TAMIL_LETTER_EQUIVALENTS.md) for the full derivation.

---

## Commands

```
etamil [FLAGS] [OPTIONS] <FILE>
```

| Flag | Effect |
|---|---|
| `--vm` | Run on the bytecode VM (default) |
| `--check` | Lex, parse and type check only, then stop — reports every error and **never runs the program** |
| `--server` | Start the synchronous HTTP server |
| `--async` | Concurrent server: async accept, blocking handlers, Ctrl-C to stop |
| `--llvm` | LLVM backend (requires `--features llvm`; Linux/macOS) |
| `--port <PORT>` | Server port (default 8080) |
| `--host <HOST>` | Server host (default 127.0.0.1) |

---

## Examples

All examples live in [`examples/`](examples/):

| Path | What it shows |
|---|---|
| `basic_samples/example.qmz` | Income tax calculator |
| `io_samples/simple_fileio.qmz` | File read/write |
| `io_samples/fileio_example.qmz` | Longer file handling |
| `db_samples/*.qmz` | SQLite — parameterised queries, rows returned as records |
| `finance/*.qmz` | The accounting framework: GST, payroll, a full cycle, receivables ageing |
| `katY/` | An eCommerce backend — catalogue, per-line GST, atomic orders, a signed payment webhook, and the same orders posted to a double-entry ledger |
| `backend/*.qmz`, `api/*.qmz` | HTTP handlers — run these with `--server`, not `--vm` |

```bash
etamil --vm examples/basic_samples/example.qmz
etamil --server --port 8080 examples/backend/hello_server.qmz
```

A route reads its request through plain variables, so the same handler is
readable in either spelling:

```etamil
வழி பெறு, "/kaNakku/:id" {
    பதில் 200, "id=" & param_id & " vakY=" & query_params["vakY"];
}

வழி பதி, "/pativu" {
    பதில் 201, "got body: " & request_body;
}

வழி பெறு, "/aRikkY.csv" {
    பதில் 200, வரிசைகள், {"Content-Type": "text/csv"};
}
```

A handler reads its request through `request_method` · `request_path` ·
`request_body` · `query_params` · `headers` · `path_params`, and each path
parameter also arrives as `param_<name>`. Response headers are an ordinary
record; without one the server answers `application/json`.

---

## Testing

```bash
cd etamil_compiler
cargo test          # 288 language + 113 unit + 10 host-capture + 8 --check
```

`tests/language_tests.rs` covers the front end end-to-end — operators, control flow, file I/O, the standard library and the accounting framework — by asserting on **program results**, not exit codes. Every bug those cover exited 0 while producing the wrong answer. Unit tests for the HTTP, request and file modules live beside their source.

Every example also runs with its expected outcome checked, including the ones that are *supposed* to fail:

```bash
./scripts/run_examples.sh
python3 scripts/transliterate.py --check   # romanization audit — keywords
python3 scripts/check_names.py --check     # romanization audit — everything else
```

CI runs the build and the full test suite on Linux and Windows for every push and pull request.

---

## Repository layout

```
etamil_compiler/
├── etamil_compiler/        # Rust crate — the host
│   ├── src/
│   │   ├── lexer.rs        # bilingual tokenizer (logos)
│   │   ├── parser.rs       # recursive-descent parser -> AST
│   │   ├── module.rs       # இறக்கு resolution
│   │   ├── vm/             # bytecode compiler + stack VM
│   │   ├── db/             # Database trait + SQLite driver
│   │   ├── http/           # HTTP server, auth, cache, logging
│   │   ├── fileio/         # file and CSV handling
│   │   └── codegen.rs      # LLVM IR backend (optional feature)
│   └── tests/              # end-to-end language tests
├── nUlakam/                # standard library — written in eTamil
│   ├── col.qmz  kaNiqam.qmz  aNi.qmz  paNam.qmz
│   └── kaNakkiyal/         # accounting framework — written in eTamil
├── examples/               # sample eTamil programs
├── scripts/                # keyword generation, romanization audit, runner
└── docs/                   # guides, reference, roadmap
```

## Libraries, written in eTamil

The standard library and the accounting framework are eTamil source, not Rust. The host provides only what a language cannot express — arithmetic on decimals, text measurement, file and socket access — and everything above that is readable and editable by the people who use it.

```etamil
இறக்கு "nUlakam/paNam/paNam.qmz";
அச்சு ரூபாய்(12345678.5);            // ₹1,23,45,678.50 — Indian grouping
```

```etamil
இறக்கு "nUlakam/kaNakkiyal/pErEtu.qmz";

த = பரிவர்த்தனை_ஆக்கு("JV1", "2026-04-01", "மூலதனம்", [
    பற்று_வரிசை("1000", 500000),
    வரவு_வரிசை("3000", 500000)
]);
பேரேடு = மதிப்பு(பதிவிடு(பேரேடு, த));   // refuses anything unbalanced
```

See [`nUlakam/README.md`](nUlakam/README.md) for the full list, and `examples/finance/kaNakkiyal.qmz` for a complete accounting cycle with GST and the three statements.

---

## Contributing

The most useful contributions right now are on the [roadmap](docs/ROADMAP.md): keeping the source text of tokens (which also gives parse errors their positions), the async server, type checking, and the remaining database drivers.

Please add a test to `etamil_compiler/tests/language_tests.rs` for any language behaviour you change, and make sure `cargo test` passes on both Linux and Windows.

---

## Documentation

| Where | What |
|---|---|
| [etamil.in/manual](https://etamil.in/manual/) | The user manual — the complete guide, [also in Tamil](https://etamil.in/ta/manual/) |
| [etamil.in/keywords](https://etamil.in/keywords/) | Every keyword in all three spellings |
| [etamil.in/finance](https://etamil.in/finance/) | The accounting and GST framework |
| [etamil.in/status](https://etamil.in/status/) | What works, what is partial, what is planned |
| [`docs/`](docs/) | Architecture, roadmap and reference material in this repository |

---

## License and credit

Copyright (C) 2026 Mohammed Maruff (Esan Maruff). Licensed under the
[GNU Affero General Public License v3.0 or later](LICENSE) (AGPL-3.0-or-later).
[`NOTICE`](NOTICE) says what that asks of you; [`AUTHORS`](AUTHORS) says who
wrote it.

The AGPL matters here because eTamil is a backend language: running a modified
compiler or runtime as a network service counts as distribution, so the changes
have to be offered back to the people using that service.

Two things the licence does *not* cover. It grants no rights in the name
**eTamil** or the domain etamil.in — fork the code freely, but a fork under the
same name misleads people about whose work they are reading. And it is not an
invitation to drop the attribution: keeping the copyright notices intact is a
condition of the licence (section 5(b)), not a courtesy.

If you use eTamil in research, please cite it — [`CITATION.cff`](CITATION.cff)
carries the metadata and GitHub turns it into a **Cite this repository** button.

---

**Version**: 1.5.0 · **Author**: Esan Maruff
