# eTamil functions in SQLite queries (spike)

**Status: a spike of design A in `docs/architecture/DATABASE_EXTENSIONS.md`** (WBS 1.20): register
eTamil functions on a SQLite connection so a query can call them. It changes nothing in the
compiler: it uses only the compiler's public API. It exists to answer whether the idea works, what
it costs, and what integrating it into the compiler would involve.

```rust
let source = "செயல் gst_total(amount, rate) {\n  திரும்பு amount * (1 + rate / 100);\n}\n";
etamil_sqlite_fn::register(&connection, source, &[SqlFunction::new("gst_total", "gst_total", 2)])?;
```

```sql
SELECT id, gst_total(amount, 18) FROM invoices;     -- 1180, 295.59, 118: exact
```

## How it works

1. The source (only function and shape definitions) is parsed and type-checked.
2. It passes the **same allowlist as PL/eTamil** (`../eTamil_pg/eval`): no files, network, other
   databases or host access, and no builtin that has not been classified as pure. So a SQL function
   cannot do what a database function must not.
3. For each function exposed, a wrapper is compiled once: the definitions, then
   `sql_result = f(sql_arg0, sql_arg1);`.
4. Each call from SQLite builds a fresh VM, binds the arguments as variables, runs the wrapper under a
   step limit, and reads `sql_result`: the pattern the compiler's HTTP workers use for a request, so
   no call sees another's state.

Values cross as the compiler's own SQLite adapter treats them (`etamil_compiler/src/db/sqlite.rs`):
a decimal goes as exact text, and text that parses as a decimal comes back as a number. A test
stores a value with the compiler's adapter and reads it through a registered function.

## What was found

**It works, with the compiler untouched.** 19 tests pass (Windows): per-row execution; exact decimals
(`250.50 * 1.18` is `295.59`, not `295.5899…`); Tamil text and Tamil function names; a different SQL
name from the eTamil one; runtime errors becoming SQL errors; an endless loop stopped at the step
limit; the allowlist refusing a function that reads the environment, and refusing it *before*
anything is registered; atomic registration (a mistake in the second function leaves none registered);
re-registering replacing a function; `NULL` in and out; a `சரி`/`தவறு` result unwrapped or failing;
and agreement with the compiler's adapter.

**Three things a user must know, each pinned by a test:**

1. **SQLite orders TEXT after every number.** An exact (text) result makes
   `WHERE gst_total(amount, 18) > 500` true for **every** row, silently. So there are two modes:
   `ExactText` (the default, no precision lost; compare with `CAST(... AS REAL)`) and `.numeric()`
   (an INTEGER when whole, otherwise a REAL, so comparisons, `ORDER BY` and `SUM` behave).
2. **`SUM` over exact text is a float.** SQLite converts the text to REAL to add it. Aggregating in
   SQL gives float precision, not the paisa; to stay exact, sum in eTamil.
3. **A function name has to be legal on both sides.** `add` and `nothing` are SQL keywords, and
   `ஒட்டு` is an eTamil keyword, so SQLite or the eTamil parser refuses them.

**What it costs per row:** about **5.8 microseconds**. `gst_total` over 20,000 rows took 116 ms in a release
build on the Windows development machine, so a million rows is around six seconds. A fresh VM per
call is not the bottleneck, and there is no case for caching one (which would risk leaking state
between rows).

## What integrating it into the compiler would take

None of this was done; it is what the spike shows is needed.

- **A way to register from a program.** A builtin (say `தளம்_செயல்_பதிவு("gst_total")`) is the least
  invasive, but it is a new builtin: it moves the builtin and keyword counts the site and README
  guard, and it needs the VM to keep the running program's `Bytecode` (today `execute` takes it by
  value and the VM does not keep it).
- **A hook on the `Database` trait,** an optional `register_function` that only the SQLite driver
  implements (PostgreSQL and MySQL would say "not supported").
- **Pooled connections.** The compiler reuses SQLite connections between requests. A function
  registered on one stays registered, so a later request that did not register it could call a stale
  one. The simplest rule is to register after every `தளம்_இணை`, which replaces the old definition;
  the alternative is to clear registrations when a connection is returned to the pool.
- **Move the allowlist into the compiler** (`etamil_compiler::purity`), since this crate and the
  PostgreSQL extension both use it and a crate named `etamil_pg_eval` is the wrong home.
- **The `functions` feature of `rusqlite`,** which the compiler does not enable today.
- **Documentation and a sample** for the rules above.

## Not verified

- Only on Windows so far; the CI workflow runs it on Linux.
- Nothing here has been used from an eTamil program, because that needs the compiler change above.
- A loadable SQLite extension (option B in the design note) is not attempted.

## Running it

```bash
cd eTamil_sqlite
cargo test
cargo test --release -- --ignored --nocapture per_row_cost    # the cost per row
```
