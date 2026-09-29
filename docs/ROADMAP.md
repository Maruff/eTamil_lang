# eTamil Roadmap

What is not built yet, why it matters, and what finishing it involves. Items are ordered by how much they block real use.

## A note on "phases"

Two unrelated numbering schemes existed in this project, both called Phase 1-N. They are now kept apart:

| Term | Meaning | Where it appears |
|---|---|---|
| **Paper Phase 1-5** | The research roadmap: compiler core → domain modules → tooling/REPL → pilot projects → policy engagement | *eTamil: An Indian FinTech DSL* (Maruff & Valli), p.13 |
| **Backend milestone 1-4** | This repository's HTTP work: sync server → async → logging → auth | source comments in `src/http/` |

They mean entirely different things. Backend milestones being "complete" says nothing about the paper's phases.

### Where the paper's five phases stand

| Phase | Status | What exists, and what does not |
|---|---|---|
| **1. Compiler and core language** | 🟢 Substantially complete | Lexer (203 keywords × 3 spellings), parser with positions on every error, bytecode VM, fixed-point decimal throughout, functions, arrays and records, results, modules, a narrow type checker, `நிலை` bindings that do not change, functions as values, and record shapes (`வடிவம்`) with methods — all four on both backends. `nUlakam/atippatY/aNi.qmz` has map, filter and fold (`ஒவ்வொன்றுக்கும்`, `வடிகட்டு`, `மடி`); the loops written before them are left as they are. Outside the core: the LLVM backend still refuses I/O statements (section 7), and connections cannot be named and held concurrently (section 5). Closed since this line was written: `மற்றும்`/`அல்லது` short-circuit; `இல்லை` can be an operand of `&`; `ஜேசான்_உரை` is implemented; the LLVM backend no longer computes in `f64` — it calls a runtime, so decimals are exact and all 97 builtins are reachable; `செயல்` declares parameter and return types and is held to them; `a > b > c` chains, expanding to `மற்றும்` over neighbouring pairs and refusing a middle operand that cannot be read twice; every one of the 203 keywords romanizes on-scheme, reached without a breaking rename by making the scheme's spelling canonical and keeping the old one; and `மறை` is XChaCha20-Poly1305 over an Argon2id key, so tampering is detected rather than decrypted |
| **2. Domain modules — accounting, taxation, banking** | 🟡 Two of three started | **Accounting:** `nUlakam/kaNakkiyal/` — chart of accounts, ledger, the three statements, reporting periods, period close, company and currency, clearing; double entry throughout. **Taxation:** GST on transactions (`vari.qmz`). **Banking: not started** — nothing addresses account numbers, IFSC, UPI, NEFT or settlement. TDS and depreciation schedules are also absent |
| **3. Tooling, a REPL shell, database integration** | 🟢 All three | **Database:** SQLite, PostgreSQL and MySQL, each verified against a live server; one database at a time, and the second is now refused rather than swapped. **Tooling:** VS Code extension with grammar and completions generated from `lexer.rs` and a CI gate against drift, `--check`, prebuilt packages, install scripts. **REPL:** `etamil --repl` — `src/repl.rs`. Variables persist in a VM created once and reused; `செயல்` definitions are kept and recompiled each line, which is why definitions are the only statements replayed — replaying the rest would run a session's side effects once per line. A line ending in an unclosed `{` keeps reading until the braces balance, so a `செயல்` or a `சுற்று` can be typed across several lines. `:vars`, `:help`, `:quit` |
| **4. Pilot projects and open-source release** | 🟡 Released; no pilot deployed | **Released:** AGPL-3.0 on GitHub, prebuilt packages for Windows, Linux and macOS (Intel and Apple Silicon), install scripts needing no administrator rights, a bilingual manual at etamil.in, CI on two operating systems. **Pilots:** the examples carry an eCommerce backend, a ledger, payroll and inventory, but none is deployed against a real product. The nearest thing is the document pipeline — `.odt`, `.ods`, `.docx` and `.xlsx` filled from real project templates and converted to PDF — which was built and verified but is not yet running anywhere |
| **5. Policy engagement — MCA, RBI, GSTN** | ⚪ Not started | Nothing in this repository bears on it. It also depends on Phase 2 being further along than it is: a GST module that handles transactions is not the same as one a regulator would recognise |

Against the paper's scheme the project is **mid-Phase 2, with Phase 3 complete**, and Phase 1 close enough to done that what remains in it are known defects rather than missing pieces. Phase 2 is the one with a piece missing rather than a defect: banking is not started.

### Backend milestones, continued

The `src/http/` comments number four milestones — sync server, async, logging, auth. A fifth cluster landed after them and is not numbered there: ODF and OOXML packages, a document renderer in `nUlakam/vativam/AvaNam.qmz`, running another program for PDF conversion, response bodies that are not text, multipart uploads, and RS256 single sign-on against a provider's published keys. Together those are what let a real document-generating backend be written in eTamil.

---

## 1. ~~Decimal arithmetic for currency~~ — RESOLVED

Every number is now a fixed-point `Decimal`, from the lexer through the AST to `Value::Number`. `0.1 + 0.2` is exactly `0.3`; `99.99 * 3` is exactly `299.97`; `18%` is exactly `0.18`.

Equality is exact. The previous `f64` value type compared numbers equal within `1e-10`, so two amounts a fraction of a paisa apart were indistinguishable — a real hazard in reconciliation.

**Division policy:** division keeps the decimal type's full precision and does *not* round at each step. Indian tax computation rounds once at the end, and rounding intermediates compounds error through a chained calculation. An explicit rounding builtin is still needed — see item 3.

**Still open:** `எண்` is the only numeric type. A separate money type carrying a currency, and integer/decimal distinction, would let the type checker (item 7) reject nonsense like adding rupees to a count.

---

## 2. ~~Keep the source text of tokens~~ — RESOLVED

`tokenize()` returns `Vec<Spanned>` — the token, its line and column, and the text it matched. That one capability fixed both problems it was blocking.

**Names are the author's own.** `வங்கி = 5` creates `வங்கி`, not `Bank`; `{வரி: 100}` produces the field `வரி`. Two positions deliberately keep the canonical English name, because what they name belongs to the host rather than the author: the database type in `தளம்_இணை`, which `db::open` matches on, and the HTTP method in `வழி`, which the router matches on.

**Parse errors carry a position**, and the parser returns `Result<_, ParseError>` rather than panicking:

```
வரி 3, நெடுவரிசை 1: ';' எதிர்பார்க்கப்பட்டது, 'அச்சு' கிடைத்தது
(line 3, column 1: expected ';', found 'அச்சு')
```

Columns count written letters rather than bytes, for the same reason string length does.

**The tradeoff, taken deliberately:** `{வரி: 1}` and `{vari: 1}` are now *different* fields, and `வருவாய்` and `varuvAy` different variables. A name is data — what the author typed — not a language construct. Nothing in `nUlakam/` or `examples/` needed changing, because the workaround at the time was compound names, which were always stored as written; a program mixing the two spellings of one keyword-backed name is the case that breaks.

**Still open:** the declared type on an assignment is now kept rather than discarded, but nothing checks it yet — see item 7.

---

## 3. Database execution

**Done for SQLite.** `src/db/` holds a `Database` trait with no driver dependency, and `src/db/sqlite.rs` implements it with the blocking `rusqlite` driver behind the `sqlite` feature (on by default). The VM keeps a connection per database type.

Queries are **always parameterised** — there is deliberately no way to splice a value into SQL text from eTamil:

```etamil
தளம்_வினா "SELECT peyar, qokY FROM pativukaL WHERE vakY = ?", ["வரவு"], வரவுகள்;
```

Rows return as an array of records, so a result set iterates like any other table. Decimals cross the boundary as text rather than `REAL`, so no precision is lost — using an inexact SQL type would defeat the point of decimal arithmetic.

**PostgreSQL** is implemented behind `--features postgres` and verified against a live server. Binding adapts to the column's type rather than picking one, because PostgreSQL infers each parameter's type from where it appears — a Decimal bound straight through would only satisfy `NUMERIC`, and `WHERE id = $1` against an integer key would fail. Money uses the native `NUMERIC` type, so unlike SQLite a text column stays text on the way back.

**MySQL / MariaDB** is implemented behind `--features mysql` and verified against a live server. Binding is simpler than PostgreSQL's because the server coerces on the way in, so parameters go over as text and stay exact for `DECIMAL`; reading back dispatches on the column type, so a `VARCHAR` of digits comes back a string while a `DECIMAL` comes back a number. `examples/db_samples/mYcIkul_qaLam.qmz` checks that, and `run_examples.sh` skips it unless `ETAMIL_TEST_MYSQL` is set, since it needs a server this repository does not provide.

Transactions work, driven as plain SQL — `தளம்_செய் "BEGIN", []` and its COMMIT — because the VM holds one connection across statements. `examples/katY` depends on that for order placement. There is no language-level transaction *construct*, which is a different thing and still open.

**One database at a time, and it says so.** `தளம்_வினா` names no handle, so with two open there would be no way to say which one a query meant. Two *drivers* at once was already refused. Two databases through the *same* driver was not: connections are keyed by driver, so a second `தளம்_இணை சீகுலைட்` overwrote the first, the count stayed at one, and every query afterwards went to the second database while the program still believed it was talking to the first. That is now refused, naming the database already open:

```
'SQLite' ஏற்கனவே 'one.db' உடன் இணைக்கப்பட்டுள்ளது
  ('SQLite' is already connected to 'one.db'): தளம்_பிரி first
```

Connecting again to the *same* database is not an error — it asks for nothing new. Disconnecting first and then connecting elsewhere works as it always did.

**Still to do:** genuinely concurrent connections, which needs the language to be able to name one — a handle returned by `தளம்_இணை` and taken by `தளம்_வினா`. ~~MongoDB and Redis need a design before an implementation~~ — both are done, and neither went through the `Database` trait, because neither should have. Redis is a command and a reply, so the host offers exactly one command generically and every Redis command works through it. MongoDB is documents, and a document is already a `பொருள்` — the mapping needed no invention, only care about numbers, which are stored as `Decimal128` rather than as doubles.

---

## 4. ~~The async HTTP server~~ — RESOLVED

`--async` runs a tokio accept loop and hands each request to `spawn_blocking`. `--server` keeps its thread pool and is unchanged.

The drafted `async_mod.rs` / `async_handler.rs` were gone by the time this was written, and nothing was salvaged from them: they wanted axum and futures, and neither turned out to be needed. Routing is this crate's own, and tokio was already a dependency, so the whole thing added **no new crates**.

**The VM stays synchronous.** Making it async would mean a yield point at every I/O instruction, and it would rule out the blocking database drivers that are far simpler to bind into a synchronous interpreter. What changed is only how connections are accepted: a connection now costs a task rather than a thread, so a slow client no longer occupies one of `2 × cores` workers for as long as it takes to send its request. Handlers still block, on tokio's blocking pool, so every driver keeps working untouched.

A prerequisite that had not been noticed: `main` was `#[tokio::main]`, which made every blocking driver panic with `Cannot start a runtime from within a runtime`. The runtime is now built by `--async` alone, around the async server.

Route matching and handler execution moved to `handler::dispatch`, shared by both servers, so the two cannot drift apart about what a route means — `path_matches` had already been duplicated in `router.rs` and `mod.rs`.

**Still to do:** the async server does not carry the logging and metrics the sync one does, and there is no connection or request timeout.

---

## 5. ~~Routes and responses as language statements~~ — RESOLVED

`வழி` statements are lifted out of the program at startup and registered with the router; everything else becomes a prelude compiled into every handler, so a route can call the file's imports and functions. `பதில்` records status and body for the server to send. Handlers compile once at registration rather than on every request.

Request data is injected as variables — `request_method`, `request_path`, `query_params`, `headers`, `request_body` — and map indexing makes `query_params["id"]` readable from eTamil.

Path parameters (`/kaNakku/:id`) are matched, and arrive as `path_params` and as `param_<name>`. `பதில்` takes its headers as an ordinary record — `பதில் 200, உடல், {"Content-Type": "text/html"}` — which is what lets a route serve HTML or a CSV export instead of the JSON the server otherwise assumes. Request bodies reach eTamil as `request_body`.

Each request still builds a fresh VM, but a connection is no longer reopened with it: `தளம்_இணை` borrows from a process-wide idle cache and the lease returns on release. Exclusively — two requests sharing one connection would share its transaction state, and a `BEGIN` in one would enclose the other's queries. A connection is rolled back as it goes back, so a handler that opened a transaction and failed cannot hand the next request a connection sitting mid-transaction.

`ஜேசான்_உரை உடல், 201` sends a body with the JSON content type already on it, defaulting to 200 when the status is left off. It asks for text rather than rendering a record, because a Rust-side encoder beside `nUlakam`'s `ஜேசான்_ஆக்கு` would be a second answer to one question — and eTamil's record syntax is not JSON, so emitting it would only look right.

---

## 6. ~~Resolving ந vs ன in the romanization~~ — RESOLVED

The three nasals now have distinct letters: **ண = `N`, ந = `n`, ன = `Z`.**

`Z` was unassigned (ழ is lowercase `z`), so it takes ன and leaves `N` unambiguously ண. Thirty keyword spellings changed, including four that spelled ந as `N` (`nikara`, `nirY`, `kOppu_nirY`, `vAwkunar`), one that used `N` for ங் (`pawku`, formerly `paNgu`), and one that spelled நிலை two different ways (`nilY_ceyqi`, formerly `nilai_ceyqi`).

This is a **breaking change to romanized source**; Tamil-script source is unaffected.

### Done: the romanization is on-scheme

`scripts/transliterate.py` implements the scheme and `--check` audits the lexer
against it. Nineteen keywords were off-scheme — `t`/`q` swapped, `ச` written `s`,
letters the scheme does not assign (`D`, `g`, `zh`), dropped doubled consonants,
and the compound convention before `_`. All nineteen are fixed, and **without
the breaking rename this section used to assume was necessary**.

A keyword may carry several spellings. The audit reads the *second* alternative
of each `#[regex(...)]` as canonical, so the scheme's spelling goes there and the
old one stays after it. Both lex; no program stopped compiling. 524 spellings
across 202 tokens, up from 505, and `--check` is a gating CI step.

`தொகை` was held back for a while on a mistaken reading, and the mistake is worth
recording because it was in an example's own comment. That comment claimed a
keyword used as a field name is stored under its *token* name, so `{நிலுவை: 1}`
would become the field `Outstanding`. It is not: `name_of` returns the source
text, so a field name is stored exactly as written, and `qokY` becoming a keyword
leaves `பதிவு.qokY` reading the same SQL column it always did. Checked against
the four examples that use it, plus the whole suite.

What is true is narrower and already documented above: 94 of the 203 keywords may
be used as names, and the other 109 may not — using one of those as a field name
is a parse error, not a silent rename.

---|---|
| `t`/`q` swapped (ட is `t`, த is `q`) | `soqqu`→`coqqu`, `toqai`→`qokY`, `uqal`→`utal`, `talY`→`qalY` |
| Letters not in the scheme at all | `matippIDu` (`D`), `toguippu` (`g`), `vazhi` (`zh`), `paDil` (`D`) |
| ச written `s` instead of `c` | `soqqu`→`coqqu` |
| Doubled consonants dropped | `iraqi_pulli`→`iRuqi_puLLi` |
| Compound convention | `varumAZ_aRikkY` vs `varumAZa_aRikkY` — the lexer drops the inherent vowel before `_`; decide and document this |

Sweeping these is one more breaking change to romanized source, so it should land in a single commit with lexer tests asserting every letter round-trips. CI runs `--check` today as a non-gating step; make it gating once the sweep is done.

---

## 7. ~~Type checking~~ — RESOLVED

`Stmt::Assign` carries the declared type and a position, and `src/check.rs` holds the program to it — on the declaring statement and on every later assignment to that name. Errors are bilingual and positioned, and all of them are reported rather than only the first, because a wrong declaration is usually one of several.

The checker is deliberately narrow: it enforces what the author declared and states **no rule the rest of the language does not follow.**

- Arithmetic on text is legal, because `உள்ளிடு` yields text and the VM converts it when it is used as a number. Flagging it would break the language's own headline example.
- A number satisfies `சொல்`, since every value renders as text and `&` concatenates whatever it is given.
- `தேதி` is ISO-8601 text, which is the representation the whole language uses.
- A call, an index, a field access and `இன்மை` make no claim at all. Functions have no declared signatures, so guessing would reject working programs. Silence is the absence of a claim, not approval.
- A function parameter and a loop variable drop any outer declaration of the same name, because they are different variables.

**Still open:** function signatures — `செயல்` cannot declare its parameter or return types, so a call is unconstrained. That, and the money type from item 1, are what would let the checker reject adding rupees to a count.

---

## Smaller known issues

- ~~**No short-circuiting.**~~ — RESOLVED. `மற்றும்` and `அல்லது` now stop as soon as the answer is known, so a guard can guard:

  ```etamil
  அ = [];
  (நீளம்(அ) > 0 மற்றும் அ[0] == 1) எனில் { ... }   // no longer an error
  ```

  The answer is still a Boolean and still the same Boolean — only what runs to produce it changed. `எழுத்து` in `col.qmz` and `பகுதியை_எடு` in `AvaNam.qmz` were written to work around this and stay because they read well, not because they are needed.
- ~~**Arrays and records never compared equal.**~~ — RESOLVED. `Value`'s equality had no arm for either, so both fell to its catch-all: `[1, 2] == [1, 2]` was false, and so was `[] == []`. Nothing warned, which is the worst way for an equality to be wrong — a program checking whether a result matched what it expected was told no, and read the no as a difference in the data. An array now compares by position, because an array is ordered; a record compares by field, because a record is not. Found by a BSON round trip that could not be made to pass: the numbers compared equal one at a time and the structures holding them did not.
- **Chained comparisons parse oddly.** `a > b > c` becomes `(a > b) > c`, so `3 > 2 > 1` is `false`.
- **Encryption is XOR, not AES.** `src/fileio/crypto.rs` uses a repeating-key XOR cipher with a default key. It should not be described as encryption in user-facing docs until it uses a real AEAD.
- **`rustfmt` and `clippy` are not clean.** CI runs both with `continue-on-error: true`; remove that once the backlog is cleared.
