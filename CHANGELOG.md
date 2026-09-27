# Changelog

Releases of the eTamil compiler, standard library and editor support. The
extension's version tracks the language's, so both are 1.1.0 here.

Every release is tagged `vN.N.N`, which is what builds the platform packages.
GitHub's generated notes list the commits; this file says what they add up to.

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

`nUlakam/aNi.qmz` gains `ஒவ்வொன்றுக்கும்`, `வடிகட்டு` and `மடி`, each taking the
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

See the [releases](https://github.com/Maruff/etamil_compiler/releases) and the
git history. `eTamil_Code/CHANGELOG.md` carries the extension's own record back
to 0.1.0.
