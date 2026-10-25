# Changelog

Releases of the eTamil compiler, standard library and editor support. The
extension's version tracks the language's, so both are 1.1.0 here.

Every release is tagged `vN.N.N`, which is what builds the platform packages.
GitHub's generated notes list the commits; this file says what they add up to.

---

## Unreleased

### Added

- **A package for Android.** `etamil-android-arm64.tar.gz` is built with the Android
  NDK against Android's own libc (API 24, Android 7.0 and later), so a program on a
  phone reaches websites and APIs by name. The Linux arm64 package runs on Android
  too, but its static musl libc looks for `/etc/resolv.conf`, which Android does not
  have, so `வலை_பெறு` could not resolve a host there. A new workflow runs the same
  build for x86_64 in Android 9 and 14 emulators, including a check that reaches a
  website by name; `packaging/smoke-test.sh` runs that check when
  `ETAMIL_SMOKE_NET=1`.

- **A function of your program, called by a SQLite query.** `தளம்_செயல்_பதிவு`
  (`qaLam_ceyal_paqivu`, `_registerFunction`) registers a செயல் on the open database
  under a name, and a query then calls it once per row: `SELECT moqqam_vari(qokY, 18)
  FROM paRRuccIttu`. It answers a சரி, or a தவறு saying why not. The function may only
  compute: one that could read a file, use the network, another database or the
  environment is refused when it is registered, by a check on the compiled code, and
  each call runs in a fresh VM under a step limit. A registration lasts only as long
  as the connection is lent to the program, so a pooled connection never carries one
  to the next request. A number result goes back as exact text by default; SQLite
  orders text after every number, so pass `"numeric"` as a fourth argument to compare
  or sort on it. SQLite only. See `examples/db_samples/qaLam_ceyal_paqivu.qmz`.

- **Libraries for building a project office.** `nUlakam/qittam` gains the rules
  a PMO application runs on: task status and roll-up, the critical path on a
  working calendar and rescheduling a project under way on its actuals, RAG
  health with its reasons, change control as a state machine whose approved
  items become tasks, scope and the features that change it, the portfolio by
  hours and by client, lifecycle readiness, the weekly status report, and the
  context a document template is filled from. `nUlakam/oruwkiNYppu` is the
  Azure DevOps integration — state mapping by category, field ownership, echo
  detection, an outbox with backoff, fingerprints, validated JSON Patch, WIQL,
  service hooks and the REST client — and `nUlakam/qayArippu` holds product
  versions and customer commitments. `examples/aluvalakam/` runs one project
  through all three in 223 lines.

- **The project office as a running service.** `examples/aluvalakam/` adds
  a SQLite-backed HTTP service on the same libraries — schedule, health,
  progress, change requests, the portfolio and an Azure DevOps service hook,
  with the outbox drained on a timer — whose handlers are plain functions,
  tested under `--vm` against a real database. It signs users in with a
  bcrypt-checked password and a signed token, admits each route to the roles
  that may use it, sets scope before baseline, renders the status report and
  charter, and keeps the customer-commitment dashboard.
- **`இப்போதைய_நொடி()`, whole seconds since 1970.** The only clock finer than
  a day counted from the program's start, so anything stored and read back
  after a restart — an outbox row's next attempt — meant nothing. Also
  `ippOqYya_noti` and `_nowSeconds`. 97 builtins now.

### Fixed

- **A baselined task can be finished.** `qittam/paNi.qmz`'s list of fields a
  locked task still accepts had the actual start but not the actual finish,
  so a task could be started on a baselined project and never recorded done.

- **Base64 wrapped with Windows line endings decodes.** `"\r\n"` is one
  letter, so the decoder's line-break skip matched neither `"\r"` nor `"\n"`
  and refused MIME- or PEM-wrapped text written on Windows.

- **The extension offers ARM Linux the arm64 package.** Since 1.2.0 the
  release has published `etamil-linux-arm64.tar.gz`, but the install command
  knew one Linux package, so a Raspberry Pi was told to download the x64
  archive, which cannot run there. It now picks by architecture, as it already
  did on macOS. `scripts/package_extension.py --from-release` also builds a
  `linux-arm64` VSIX that carries the arm64 compiler.

---

## 1.4.2 — 2026-09-28

Security updates to two dependencies. The language, and what every program
means, are unchanged.

### Security

- **rustls 0.23.43 → 0.23.45**, for
  [RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285):
  TLS 1.3 handshake messages were accepted across encryption-level
  boundaries (medium, 5.3). rustls carries outbound HTTP, as `வலை_பெறு` and
  the rest, and the optional `rustls` feature. Both the compiler and the
  Android app update.
- **rust_decimal 1.42.1 → 1.43.0**, which removes rkyv, and
  [RUSTSEC-2026-0235](https://rustsec.org/advisories/RUSTSEC-2026-0235) with
  it, from both lockfiles. rkyv was never compiled into eTamil. It was listed
  only because rust_decimal's `std` feature named it, and Cargo locks a crate
  behind a weak dependency feature even when nothing turns that feature on.

`cargo audit` now reports no vulnerabilities.

---

## 1.4.1 — 2026-09-28

A fix to 1.4.0: reading JSON rounded a number with more than about 17
significant digits. Nothing else changes.

### Fixed

- **`ஜேசான்_படி` reads a number exactly.** 1.4.0 said a number keeps its
  decimal text, but it went through f64 on the way in:
  `12345678901234567.891` was read as `12345678901234568`, and
  `1.0000000000000000001` as `1`. `serde_json` is now built with
  `arbitrary_precision`, so the number is read from the source's own text.
  An exponent, as in `1e3` or `-2.5E-2`, is still read. Writing JSON was
  already exact.

---

## 1.4.0 — 2026-09-27

Two libraries written in eTamil, and the builtins they needed. nuNNaRivu finds
the part of the documentation that answers a question, and can ask a model to
phrase the answer. kOppumuRY lets a program find its files. Writing nuNNaRivu
found seven gaps in the language, and they are fixed. One of them can refuse a
program that ran under 1.3.0: two imported modules may no longer define the
same name.

### nuNNaRivu: retrieval and language models, in eTamil

`nUlakam/nuNNaRivu/` answers "which part of the documentation answers this
question?", and can then ask a model to phrase the answer. It has thirteen
modules, written in eTamil over the host's arithmetic, strings and HTTP.

- **Two kinds of search, because each fails where the other succeeds.**
  `coRqEtal.qmz` scores by shared words (BM25). `oRRumY.qmz` finds by meaning,
  over vectors from `utpoqippu.qmz`. `iNYppu.qmz` merges the two by rank,
  because their scores are not on the same scale.
- **Asking a model is optional.** Retrieval alone takes milliseconds, and
  generation is never required. `urYyAkkam.qmz` uses a model on the machine
  itself by default. Another provider needs a key from whoever runs the
  program. There is no default key, and a provider left without one fails
  before the request, saying so.
- **`aLavItu.qmz` measures whether retrieval works**, so that tuning it is an
  experiment rather than an opinion.

Writing it found seven gaps in the language, and they are fixed:

- **A record can be used as a map.** `புலம்_உள்ளதா` and `புலம்_அல்லது` are
  one hash lookup. `poruL.qmz` had walked every field name, so 800 lookups over
  800 keys took 61.6 s; they take 1.89 s now.
- **JSON is a builtin.** `ஜேசான்_படி` and `ஜேசான்_ஆக்கு` replace a parser
  written in eTamil: a 624 KB file that did not finish in ten minutes takes 1.31
  s. A number keeps its exact decimal text rather than passing through f64.
- **`x = x & e` appends in place** on the VM, as `x = இணை(x, v)` already did,
  so building a string in a loop no longer copies everything written so far.
- **`வர்க்கமூலம்`, `இயற்கை_மடக்கை`, `இயற்கை_அடுக்கு`, `அடுக்கேற்று` and
  `பத்தின்_மடக்கை` are builtins**, computed on the decimal rather than on f64.
  √25 is exactly 5 and log10(1000) exactly 3, where the old hand-written series
  left digits in the 28th place.
- **`வரிசையாக்கு` and `புலத்தால்_வரிசையாக்கு` sort**, replacing the insertion
  sort each library wrote for itself.
- **Two imported modules may no longer define the same name.** Imports are
  flattened, so one quietly replaced the other. That made a program fail three
  calls later with an error that named neither file. A program can still
  define its own version of a library function; only a collision between
  imports is refused. **This can refuse a program that ran under 1.3.0.**
- **A reserved word used as a name says so**, when a name was clearly wanted.

That brings the language to 93 builtins, with 794 functions in `nUlakam`. The
eTamil versions the builtins replace are deleted, since a library function
would shadow a builtin of the same name. `jEcAZ.qmz` goes from 340 lines to
67. `docs/language-gaps-from-nuNNaRivu.md` records the whole list, the
benchmarks, and what is still open: namespaced imports and private helpers.

### The file system, from eTamil

`nUlakam/kOppumuRY/kOppumuRY.qmz` (கோப்புமுறை) lets a program find its files. It can
join and split paths, and read a name's extension. It can list a directory,
walk everything under one, or collect the files with one extension:
`நீட்சியால்_கோப்புகள்("nUlakam", "qmz")`. It can also turn a modification time
into a date.

Only three builtins come from the host, and the library is written in eTamil
around them:
- `கோப்பகம்_படி` reads a directory;
- `கோப்பு_விவரம்` says whether a path is a directory, its size and when it
  changed;
- `கோப்பு_உள்ளதா` says whether a path exists.

Their English names are `_readDir`, `_fileInfo` and `_fileExists`. That brings
the language to 96 builtins, and `nUlakam` to 806 functions.

A path is written with `/` on every platform, since Windows accepts it in every
call. `பாதை_இயல்பாக்கு` turns a backslash into `/`. `கோப்பகம்_படி` answers
with bare names in sorted order, so two runs list a directory the same way, and
the library's listings join each name onto its directory.

In the browser build, a directory is inferred from the in-memory file set. No
modification time was ever recorded there, so `மாற்றம்` is `இன்மை` rather
than a date in 1970.

---

## 1.3.0 — 2026-09-27

eTamil on Arduino boards. `etamil --artino` compiles a program to firmware for an
Uno, Nano, Mega, Pico or Pico 2, and the VM gets a board of its own: pins and
serial ports on a Raspberry Pi, and a simulated board anywhere. For the VM and
the desktop backend, every program means what it meant under 1.2.0.

### artino: a program as firmware

`etamil --artino --board uno prog.qmz` lowers the program through LLVM to an
object for the board and packages it as a precompiled Arduino library, with a
sketch around it. arduino-cli then builds it, or uploads it with `--upload COM5`.
Top-level statements run once at power-on, `இடைவெளி N { }` runs every N seconds,
and `சுழற்சி()` runs every time round the loop.

A board number is the value × 1000 in 64 bits: three decimals, as
`வட்டமிடு` would give them. Wherever that differs from the VM — a
rounding, an overflow, a division by zero — the board says so once, over
serial, and carries on. It never gives a different answer silently.

Text and letters are counted as the VM counts them. So are arrays, results and
`?`, `வடிவம்` records and serial ports. `nUlakam/atippatY/col.qmz` and `aNi.qmz` compile
for a board as written. `docs/artino.md` has what compiles, what is refused and
why, and what each board takes.

An Uno has 32 KB of flash and 2 KB of RAM. After every Uno, Nano or Mega build,
`etamil` shows how much RAM the variables and the deepest stack can take,
because arduino-cli counts only the first.

### C++ libraries, through artino.toml

A manifest beside the program names the C++ it may call: libraries, headers,
objects, and an eTamil name for each call. Numbers, booleans and text cross the
boundary. A `[[port]]` makes a Stream, such as SoftwareSerial, one of the
program's serial ports. The examples drive a servo, a ring of NeoPixels and a
DS18B20 thermometer.

### The hardware library

`nUlakam/vaZporuL/` holds the hardware API — pins, analog readings, time,
serial ports, tone and the watchdog — and a file of pin names for each board:
`yUnO`, `nAnO`, `mekA`, `pIkO` and `rAspY`. Importing a board's file names its
pins, such as `விளக்கு_முனை` or `ஒப்புமை_0`, so moving a program to another board
is changing one line. artino refuses a board file that does not match
`--board`.

On the VM the same functions work too. `ETAMIL_BOARD=sim` gives a simulated
board on any machine, where a test sets the pins, feeds the serial ports and
moves the clock. A Raspberry Pi's pins work through the Linux GPIO device, and
serial ports by device path on Linux and macOS. Neither has run on real
hardware yet. Windows has the simulated board only.

### Editor support

VS Code gains *Build for a board* and *Build and upload to a board*. They need
an `etamil` built with `--features llvm`, and the compiler the extension
carries has none. Completions and highlighting know the 20 new builtins and the
board files: 82 builtins and 722 library functions.

### For contributors

- **artino's conformance suite runs in CI.** `scripts/artino_conformance.sh`
  compiles every `tests/artino` program for the runner and compares it with
  the VM. It runs again as `host-small`, which is how an Uno is compiled.
  Every artino program is also compiled for Uno, Mega, Pico and Pico 2. That
  is the only place the AVR and ARM code generation is exercised.
- **Hardware can be tested off the board.** A `.world` file beside a test
  scripts time, pins, analog readings and the lines arriving on each serial
  port, including nodes that answer a program's polls.
- **The LLVM build takes `libc` on Unix**, for termios and the GPIO ioctls.

---

## 1.2.0 — 2026-09-27

A package for the Raspberry Pi, one file extension instead of two, and a
standard library that says in English what every function is for. The language
is unchanged: every program means under 1.2.0 exactly what it meant under 1.1.0.

### Raspberry Pi, and any 64-bit ARM Linux

`etamil-linux-arm64.tar.gz` joins the release packages. It is for a Raspberry
Pi 4 or 5 running 64-bit Raspberry Pi OS, and any other aarch64 Linux. Like the
x64 package it is one static musl binary with the PostgreSQL and MySQL drivers
included, so it does not care which Debian release the Pi is on. It is built and
smoke-tested on an ARM runner rather than under emulation, so the binary that
passed is the binary that ships.

`uname -m` says which to download: `aarch64` means this one. A 32-bit Raspberry
Pi OS reports `armv7l`, and there is no package for it.

Not yet proven: a Raspberry Pi 5 kernel uses 16K memory pages where the CI runner
uses 4K, so a passing smoke test does not show the binary runs there. The
linker's default alignment should cover it; a report from a Pi 5 settles it.

### `.qmz` is the one extension

`.etamil` was discarded. The VS Code extension already registered only `.qmz`.
Now the tree-sitter grammar, the Rouge lexer, the example and parity runners, and
the documentation say the same. The compiler never looked at the extension and
still does not.

### Every standard-library function says what it is for

153 functions in `nUlakam/` had no English documentation — 100 had no comment at
all, 53 only Tamil. They are written now, in the library's own style: what the
function does, then why it is that way.

The documentation index was reading the wrong line for another 271. Where a
comment opened with a bare signature, the index took the signature — which a
caller can already see — and the sentence explaining the function never left
the file. It skips a signature-only line now. VS Code hovers show the
explanation, and searching for what a function *does* finds it: seven functions
retrieval could not find at all now name the concept a reader searches for
("trial balance", "GST", "parse JSON").

### For contributors

- **The website is checked against the compiler.** CI checks out the site and
  compares its published keyword counts, and the version in `_config.yml`,
  with the lexer and `Cargo.toml`. At 1.1.0 the site went on saying 1.0.0. A
  release now fails CI until the site says the same version.
- **The wasm boundary is checked.** `scripts/check_wasm_boundary.py` compares
  what `lib.rs` and `wasm.rs` claim reaches the browser build with the
  attributes on the declarations.
- **Text checks run once**, in their own conventions job, instead of twice in
  the compiler matrix.
- **One implementation of the script-mark rule.** The rule saying which ASCII
  is eTamil script and which is English is `script_spans()` in the compiler,
  used by both editors, instead of two separate copies.
- **Editor support is generated from the committed tree**, so an untracked file
  in a working directory can no longer leak into shipped artifacts.
- **The dev profile keeps line tables, not full debug info.** The largest debug
  file falls to 55 MB, a fresh `target/` with every profile and feature is 2.6
  GB, and a Windows link failure (`LNK1318`, a full disk) goes with it.
- The installed-binary tests retry an exec that races another test thread on
  Linux (`ETXTBSY`), rather than failing at random.

## 1.1.0 — 2026-09-23

Four additions to the language, each on both backends — the VM, and LLVM
through the runtime — and each an opt-in that leaves every existing program
meaning what it meant.

### `நிலை` — bindings that do not change

`நிலை எல்லை = 250000;` is bound once. Assigning it again, or changing an element
or a field of it, is refused before the program runs; a parameter can be fixed
the same way. Values are copied rather than shared, so fixing the name fixes the
whole value, as Rust's `let` does. `நிலை` remains an ordinary name everywhere
it does not begin a binding — two examples use it as a variable.

### Functions are values

A `செயல்` can be held in a variable, passed, kept in an array or a record, and
returned; a builtin can too. `செயல்(x) { … }` writes one where a value goes, and
inside a function it carries copies of the locals it reads. Calls through a
value, `f(1)(2)` and `விதிகள்[0](x)`, parse and run. A parameter can be declared
`செயல்` and is held to it. The REPL keeps a function value working from one
line to the next.

### `வடிவம்` — record shapes

`வடிவம் கடன் { எண் அசல், எண் வீதம் }` declares which fields a record has and what
each holds. A shaped literal must give every field and no other; a mistyped
field on a declared or `நிலை` name is refused before the program runs, with the
fields the shape does have; a value only known at runtime is checked when the
record is made or a field set. `..பழையது` fills in the rest from another record
of the shape, and `கடன்(பதிவு)` makes a plain record into one as a result.

### Methods

A `செயல்` written inside a `வடிவம்` is attached to it. With `இது` first it is
called on a record and cannot change it — Rust's `&self`; without, it is called
on the shape. No inheritance.

### Map, filter and fold

`nUlakam/atippatY/aNi.qmz` gains `ஒவ்வொன்றுக்கும்`, `வடிகட்டு` and `மடி`, each taking the
rule it applies as a `செயல்`, with a test suite in `aNi_cOqaZY.qmz`.

### Also

- A program saved with CRLF line endings runs. Every line of one used to be an
  "unrecognized input" error; `\r\n` is now read as `\n`, inside strings too,
  so a file means the same thing however an editor saved it.
- The Windows build artifacts of the tree-sitter grammar are no longer tracked.
- The checker now also looks inside conditions, loop collections and returned
  values, where it used to skip calls. Nothing in `examples/` or `nUlakam/`
  was newly refused.
- A compiled program's top-level loop variables and query results are globals,
  as they are on the VM, so a `செயல்` can read them.
- `eTamil_Code/test/counts.test.js` matched count words as substrings, so
  "thirty-two" also claimed "thirty"; it now matches whole words.

---

## 1.0.0 — 2026-09-13

Sixty commits since 0.4.0. The headline is not a feature: it is that the
standard library is now large enough to do a real job, and that several things
the repository used to claim about itself are now checked rather than asserted.

### The standard library nearly tripled

**254 functions in 41 files became 691 in 82**, across seven new directories.
All of it is written in eTamil and tested in eTamil — 881 assertions in the new
suites alone, run by `scripts/run_examples.sh` like any other program.

| directory | what it is |
|---|---|
| `celavu/` | cost accounting — marginal and absorption costing, overhead apportionment including the reciprocal case, process costing with equivalent units and normal/abnormal loss, activity-based costing |
| `qittam/` | project costing — earned value, the critical path, contract pricing types with the point of total assumption, the four dependency types with lag |
| `nErativari/` | direct tax — the five heads, Chapter VI-A, the slab → rebate → surcharge → marginal relief → cess ladder, TDS, advance tax with 234A/B/C, deferred tax under Ind AS 12 |
| `varuvAy/` | revenue — the Ind AS 115 five-step model, contract assets and liabilities, modifications, principal versus agent, onerous contracts under Ind AS 37 |
| `nAtkAtti/` | the calendar — date arithmetic, working days, and the validation the host's own date handling does not do |
| `vaLam/` | resources, timesheets, and posting actual cost to the ledger |
| `itar/` | risk — expected monetary value, and three-point estimation with the variance that a single-point estimate cannot carry |

`examples/qittam/` threads thirteen of these modules across five directories
through one worked project.

### The language says which of its ASCII is English

eTamil is written three ways and two of them are the same bytes: `ceyal` is
`செயல்` spelled under the ezuqqu scheme, `sum` is an English word. There is an
optional **eTamil font** in which the ASCII letters carry Tamil glyphs, and
under it an unmarked English name is unreadable — `sum` draws as ஸும்.

Two marks fix that, and they are now rules of the language:

- an identifier containing English ASCII begins with `_`
- a comment containing English ASCII is wrapped in `__ … __`

Applied across the tree: 38 names and 4,704 comments. `scripts/check_script_rules.py`
enforces it and gates CI. See [docs/reference/SCRIPT_RULES.md](docs/reference/SCRIPT_RULES.md).

### Seventeen keywords that had no English spelling

Half the keyword table could be written in English and half could not, and the
half that could was the plumbing — `_get`, `_select`, `_encrypt`. There was no
English eTamil program, not even a hello-world: the first conditional dropped
you back into Tamil. The types, the literals, `_if`, `_else`, `_loop`, `_print`
and `_input` close it.

**No reserved keyword is now without an English spelling.** 118 of 202 keywords
have one; the 84 that do not are, without exception, words the parser accepts as
ordinary names — the accounting vocabulary, which stays Tamil on purpose.

### The LLVM backend stopped trading the language for a register

It computed in `f64`, then in `i64`, and both were the same bargain: the IR held
the value, so the language shrank to what fits in a register. Now every value is
a handle into an arena in `src/runtime.rs` and every operation is a call into
it — the same `rust_decimal` and the same `Value::to_string` the VM uses, so
`0.1 + 0.2` is `0.3` and `1 / 3` keeps all twenty-eight digits on both sides
because it is the same code rather than two that agree.

`தளம்_இணை`, `தளம்_செய்` and `தளம்_வினா` compile, so a compiled program can open
a database and read and write it. **99 of 111 programs would compile**, and
`scripts/llvm_gap_report.py` makes what is refused countable on a machine with
no LLVM at all.

### The VM stopped copying what it was about to throw away

Two measured fixes:

- Every instruction was deep-copied before dispatch, which meant a heap
  allocation for a variable's name on every access. That was most of the ~75 ns
  per instruction the benchmarks recorded.
- `x = இணை(x, v)` copied the whole array per call, so building a ledger of *n*
  rows copied ~n²/2 elements. It appends in place now, which is
  indistinguishable because every other binding already owns its own copy.

Appending 64,000 items now costs what 4,000 did.

### The editor is the whole installation

The VS Code extension carries the compiler, the standard library, the examples
and the eTamil font, and installs the font for you. It renders both marks —
under `etamil.eTamilFont` the eTamil face is painted over the ASCII that is
eTamil script, leaving the ISO font everywhere else. Its keyword, builtin and
stdlib counts are pinned to the generated language data by a test, because that
README was wrong twice.

### Audits that now gate rather than being remembered

`transliterate.py` checked only the keyword table, which is how `viziqam` — a
spelling that means nothing — reached a module name, a table and two columns.
Four scripts gate CI now: the scheme on keywords, the scheme everywhere else,
the two script marks, and duplicate function names across `nUlakam`.

### Breaking changes

- **`உள்ளதா` is now only the array function.** `AvaNam.qmz` defined a second one
  for substrings; two modules sharing a name is a silent redefinition, and the
  document one is `ஆவணத்தில்_உள்ளதா`.
- **`xml_ஆக்கு` and `pdf_ஆக்கு` are `_xml_ஆக்கு` and `_pdf_ஆக்கு`**, along with
  the document format constants and the z-score functions in `itar/`. Rule 1
  applies to the library first.
- **`கணக்கு_ஆக்கு` in `qittam/` is `கட்டுப்பாட்டுக்_கணக்கு_ஆக்கு`**, for the
  same reason `உள்ளதா` moved.
- **Seventeen new reserved words** in the `_` namespace. The type names are the
  ones to watch: `_int`, `_float`, `_string`, `_text`, `_bool`, `_array`,
  `_data`, `_object` and `_date` cannot be variable names, where most languages
  would allow them.

---

## 0.4.0 and earlier

See the [releases](https://github.com/Maruff/eTamil_lang/releases) and the
git history. `eTamil_Code/CHANGELOG.md` carries the extension's own record back
to 0.1.0.
