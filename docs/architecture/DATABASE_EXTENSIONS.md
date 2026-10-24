# Database extensions: PL/eTamil and a SQLite extension (design note)

**Status: a proposal, for a decision. No code.** It covers two rows of the ecosystem
plan, 1.19 *PL/eTamil (database procedures)* and 1.20 *SQLite extension (embedded
database support)*. Both are marked "not started" because each can mean more than one
thing, and the choice changes the work by an order of magnitude. This note says what
each could be, what the repository already gives, and what to decide.

## What exists today

Checked in the source, not assumed:

- **eTamil already talks to databases.** `src/db/` holds a `Database` trait and three
  drivers: SQLite (`rusqlite`, bundled, on by default), PostgreSQL (`postgres`, behind
  `--features postgres`) and MySQL (`--features mysql`), each verified against a live
  server (`docs/ROADMAP.md`). So "embedded database support" is **not missing**; what is
  missing is the other direction, below.
- **Decimals cross as exact values.** The SQLite adapter binds a decimal as text and
  reads text that parses as a decimal back as a number (`src/db/sqlite.rs`), because
  SQLite has no exact decimal type; the PostgreSQL adapter uses `NUMERIC`.
- **The compiler is a library as well as a binary.** `Cargo.toml` builds `etamil_compiler`
  as both `rlib` and `cdylib`, and `VM::new()` plus `vm::execute(bytecode)` run a program.
- **It already has a side-effect-free core.** `src/lib.rs` compiles out `module`, `db`,
  `http`, `redis` and the server for `wasm` targets, and keeps `lexer`, `parser`, `check`,
  `stdlib`, `vm`, `runtime` and `crypt`. That subset can compute (the finance library,
  decimals, strings, collections) but cannot import, open a file, touch a socket or reach a
  database. This matters for the security question below.

## 1.20: what "SQLite extension" could mean

| | What it is | New artifact? | Effort |
|---|---|---|---|
| **A. Functions inside the compiler's own connections** | When an eTamil program opens SQLite, register eTamil functions as SQL functions (`rusqlite`'s `create_scalar_function`), so `SELECT gst_total(amount, 18) FROM invoices` works inside `DBQuery`. | No | Small |
| **B. A loadable extension** | A `.dll`/`.so`/`.dylib` any SQLite client can `.load`, exposing eTamil functions (GST, TDS, rounding, Tamil number formatting) to the `sqlite3` shell, Python, DB Browser and so on. | Yes, a separate crate per platform | Medium |
| **C. Nothing; mark it done** | The row's goal, SQLite from eTamil, already exists. | No | None |

**Recommendation: A, then B only if there is demand.** A gives eTamil programs the
financial functions in SQL, which is the useful part, with no new package. B is a real
product (and a support burden) for people who are not writing eTamil at all.

Things B would have to settle, so they are worth knowing now:

- A loadable extension must not link its own SQLite, but the compiler's `rusqlite` is built
  with `bundled`. It would be a **separate crate** using `rusqlite`'s loadable-extension
  support, not a feature of the compiler.
- Values: SQLite passes numbers as `INTEGER`/`REAL`/`TEXT`. To keep money exact, functions
  must take and return **text** decimals, as the adapter already does.
- Each call needs a VM. A fresh `VM::new()` per call is simple and slow; a cached VM per
  connection is faster and must not leak state between rows. Functions should be declared
  *deterministic* only if the library function is, or SQLite will cache wrongly.
- A panic must never cross the C boundary into SQLite; every entry point needs a guard.

## 1.19: what "PL/eTamil" could mean

A PostgreSQL *procedural language*, so a procedure can be written in eTamil:

```sql
CREATE FUNCTION gst_total(amount numeric, rate numeric) RETURNS numeric
LANGUAGE pletamil AS $$ திரும்பு amount * (1 + rate / 100); $$;
```

| | What it is | Effort |
|---|---|---|
| **A. A native extension** | A PostgreSQL extension (a shared library) holding the eTamil VM and a language handler, built with `pgrx` (the standard Rust framework). Arguments and results map `numeric` to eTamil decimals; SPI gives procedures a way to run queries. | Large |
| **B. Compile to PL/pgSQL** | Translate eTamil into PL/pgSQL text at `CREATE` time. No native code to install. | Medium, but only a subset can be translated, and errors point at generated text |
| **C. An external service** | Procedures that call a separate `etamil` process over HTTP. | Small, but it is not a procedural language: no transactions, no row context |

**Recommendation: A, in two steps, and not before the decisions below.** Step one is a
**trusted** language built on the side-effect-free core above: pure computation over
arguments, with no files, sockets, imports or `DBQuery`. That subset already exists as the
wasm build, so the compute path needs no new sandbox. Step two adds an **untrusted** variant
(superuser-only, like `plpython3u`) with query access through SPI.

What A has to settle:

- **Security.** Postgres distinguishes trusted languages (any user may create functions) from
  untrusted ones. eTamil has file, network and database statements, so the full language is
  untrusted. Only the wasm-gated subset can credibly be trusted, and that claim needs a real
  review, not just the compile-time gate.
- **Types.** `numeric` to a decimal is natural, but the VM's decimal range and rounding must
  match `numeric`'s, and `NULL` has to survive both ways. Composite and array types are a
  later step.
- **Errors.** eTamil's diagnostics are bilingual; a Postgres error has one message. Decide
  which language leads, and map a runtime error to a `SQLSTATE`.
- **Transactions.** Functions run inside the caller's transaction; `PROCEDURE` with `COMMIT`
  is a separate, harder feature. Start with functions.
- **Packaging.** A `.so` per PostgreSQL major version (say 14 to 17) and per platform;
  Windows is hard with `pgrx`. Realistically Linux first, as `.deb`/`.rpm` beside the
  existing packages.
- **Licence.** The compiler is AGPL-3.0-or-later. Loading it into a PostgreSQL server
  process makes that server's extension AGPL. That is allowed, but it is a decision about
  who may ship it, and worth making on purpose.

## Decisions needed

1. **1.20:** A (functions inside the compiler's SQLite), B (a loadable extension too), or C
   (close the row)?
2. **1.19:** is a native extension (A) the intent, or would the compile-to-PL/pgSQL route (B)
   be preferred to avoid installing native code?
3. **1.19:** is a *trusted* subset enough for a first release, with the untrusted language
   later?
4. **1.19:** the oldest PostgreSQL major version to support, and Linux only to begin with?
5. **1.19:** is AGPL for the PostgreSQL extension acceptable?

## What is not known

- Whether `pgrx` builds cleanly against this crate's dependencies (`edition = "2024"`, Rust
  1.88, and a large dependency tree with `tokio` outside the wasm subset). Nothing was
  tried; a one-day spike would answer it before any commitment.
- How much of the standard library works in the side-effect-free core. It is the same code
  the browser editor runs, but its coverage of `nUlakam/` has not been measured for this
  purpose; the browser editor cannot import the library at all today.
- Whether `rusqlite` 0.32's loadable-extension support fits option 1.20 B as described. That
  comes from general knowledge of the crate, not from anything tried here, and the extension
  API has changed between releases.
- Real demand for either. Both rows came from a generated plan, not from a user asking.
