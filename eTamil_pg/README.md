# PL/eTamil spike

**Status: a spike, to answer the questions in `docs/architecture/DATABASE_EXTENSIONS.md`.**
It is not a release and not a procedural language yet. What it is: a PostgreSQL extension
with two SQL functions that run eTamil inside a backend.

```sql
CREATE EXTENSION etamil_pg;
SELECT etamil_expr('0.1 + 0.2');                          -- 0.3, exactly
SELECT etamil_eval('அச்சு("வணக்கம்");');                   -- வணக்கம்
SELECT n, etamil_expr(n || ' * 2') FROM generate_series(1, 3) AS n;
SELECT etamil_eval('இறக்கு "nUlakam/paNam/paNam.qmz";');   -- ERROR: a database function may not use this statement (Import)
```

## Layout

| Path | What it is |
|---|---|
| `eval/` | A plain Rust crate: runs an eTamil program that may only compute, and refuses one that reaches outside. **No PostgreSQL, builds and tests anywhere**, including Windows. |
| `src/lib.rs` | The extension, built with `pgrx` 0.19.3: turns arguments and results into SQL values and a failure into a SQL error. Linux only, since `pgrx` does not build on Windows. |
| `.github/workflows/pg-spike.yml` | Builds and tests it inside PostgreSQL 14, 16 and 17. |

## What the spike found

**Checked here (no PostgreSQL needed):**

- **The compiler builds as a library without its default features.** `cargo check --lib
  --no-default-features` succeeds: no SQLite and no HTTP client are needed to run the VM.
  (`tokio` and a few other crates still link, because they are not optional; nothing starts
  a runtime.)
- **The dependency trees resolve together.** `pgrx` 0.19.3 and `etamil_compiler` 1.4.2 resolve into
  one `Cargo.lock` (330 packages) with no conflict. The extension needs Rust 1.96 for `pgrx`;
  the compiler's own floor stays 1.88, since they are separate crates.
- **A program runs with captured output.** The compiler already has an embedding path, used by
  the Android app: `host::begin_capture` and `end_capture`. It is per thread, which suits a
  PostgreSQL backend (one thread per process). The tests check that capture is switched off
  again on every path out, so one call cannot swallow the next one's output.
- **Runaway recursion cannot crash a backend.** The VM counts call depth itself and stops at 256
  with a clean error. A test runs it on a thread with a 2 MiB stack to prove the Rust stack is
  never the limit.
- **An endless loop stops.** A step limit of 1,000,000 instructions (a tenth of the browser's)
  ends it with an error.
- **What a database function may not do is enforced before it runs**, by an allowlist over the
  syntax tree (`eval/src/lib.rs`):
  - Only statements that compute are allowed: assignment, functions and shapes, `திரும்பு`,
    index and field assignment, expressions, `அச்சு`, conditions and loops. Imports, input, files,
    databases, routes and servers are refused, **and so is any statement kind the language
    gains later**, until someone reads it.
  - Builtins: **40 of the 97 on `main` are allowed (120 spellings, Tamil included) and 57 are refused**.
    The A1 banking work adds nine more (XML and e-Sign primitives); they are refused by default until
    someone reads them and adds them to the list.
    The list is in `eval/build.rs`, grouped by why each is left out: files and the web, other
    databases, the host (`_env`, `_exit`, `_run`, `_sleepMs`), hardware, keys and secrets. A
    builtin the compiler gains later is refused until it is added there, and the build fails if
    an allowed alias disappears from the interpreter.
  - A refused builtin is refused **by name as well as by call**, so `f = _env; f("HOME")` and a
    use inside a lambda are caught.
  - Every refusal test asserts the program was refused by the allowlist (stage `pure`) and not
    by failing to parse.

- **A fresh VM per call is cheap.** Each call lexes, parses, checks, compiles and builds a VM from
  scratch. Measured in a release build on the Windows development machine, outside PostgreSQL: **about
  5 microseconds** for a trivial expression, and about **221 microseconds** for a 100-iteration loop
  that calls a function. So there is no case for caching a VM between calls (which would also risk
  leaking state between rows); the cost that matters is the program's own work. Inside PostgreSQL
  and on Linux the numbers will differ.

**Checked in CI** (run 37201767152, commit `b661e18`, 2026-10-04): the extension builds with `cargo pgrx`
against **PostgreSQL 14, 16 and 17**, and all eight `#[pg_test]` tests pass inside a real backend: a program
run, exact decimals, a Tamil text round trip, one call per row of a query, no shared output between calls,
an import refused, `_env` refused, and a syntax error raised as a SQL error. So the compiler, with its
thread-local state and linked `tokio`, loads into a backend and runs. (The first run built the extension
but could not install it into the system PostgreSQL's directories, and the second had three tests whose
expected error messages were prefixes where `pgrx` matches exactly; both were mistakes in the workflow and
the tests, not in the extension.)

**Still not checked:**

- Behaviour under load or over a long session: memory use across many calls, and a backend that runs
  programs for hours. The compiler allocates through Rust's allocator, not `palloc`, so memory a program
  uses is invisible to PostgreSQL's memory accounting and limits.
- PostgreSQL 13, 15 and 18, and anything but Linux x86-64.
- Concurrency: tests run one backend at a time.

## What this is not

- **Not a procedural language.** `CREATE LANGUAGE pletamil` needs a call handler that receives the
  function's arguments, binds them to names in the program, and turns a returned value back into
  a SQL value. This spike only runs a program and returns its printed text.
- **Not trusted.** `etamil_pg.control` says `superuser = true, trusted = false`. The allowlist is
  tested, but it has not had the review a *trusted* language needs: PostgreSQL lets any user
  create a function in one. Things still open are listed below.
- **Not a sandbox.** It limits what a program may *name*, not how much memory it may use, and the
  step limit does not bound one expensive builtin (a large sort, a password hash).

## Open questions this raises

1. **Return values.** A function wants `RETURNS numeric`, not text. The VM prints; it does not
   hand back a value. A call handler needs either a way to read the returned value or the
   convention that a function body ends with `திரும்பு`, wrapped as a eTamil function and called.
2. **A variable named like a builtin.** `Variable(name)` is refused if `name` is a forbidden
   builtin, even when the program defined its own variable of that name. Conservative, and
   it may surprise: `_env = 1;` is refused.
3. **Clock builtins** (`_today`, `_nowSeconds`, `_millis`) are allowed, which makes a function that
   uses them `VOLATILE`. `IMMUTABLE` and `STABLE` need the allowlist split further.
4. **`_hashPassword`** is allowed (it is pure) and expensive; a query that calls it per row is a
   denial of service on itself.
5. **Errors.** The compiler's message is bilingual and one line; PostgreSQL has a message, a
   detail and a hint, and an `SQLSTATE`. All errors are `ERRCODE_RAISE_EXCEPTION` today.
6. **Licence.** Loading this into a PostgreSQL server makes the extension AGPL. Accepted in the
   design note; recorded here because it is a property of the artifact.

## Running it

```bash
# Everything that does not need PostgreSQL (any platform):
cd eval && cargo test

# The extension (Linux; needs PostgreSQL and its headers, and Rust 1.96 or later):
cargo install cargo-pgrx --version =0.19.3 --locked
cargo pgrx init --pg16=/usr/lib/postgresql/16/bin/pg_config
cargo pgrx test pg16
```
