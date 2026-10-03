# Cross-language comparison

The same slab-tax loop in every language installed on the machine, run at a
workload size each one can actually be timed at, and reported as nanoseconds
per iteration.

```bash
pwsh scripts/bench/compare/run.ps1          # Windows
./scripts/bench/compare/run.sh              # Linux / macOS
```

This sits alongside `scripts/bench/`, which measures a fixed 100,000-iteration
workload and is the number quoted in `docs/ARCHITECTURE.md`. The difference is
purpose: that one answers "how long does this program take", this one answers
"how does the VM compare to other runtimes", and answering the second needs two
things the first does not have.

## Why the iteration count comes from outside the program

Every program here reads its iteration count from `argv` — from stdin for
eTamil, which has no `argv`.

Not for convenience. At `-O2`, C and Rust will evaluate a loop whose bounds are
a literal *at compile time* and print a constant, so the measurement becomes
process startup and nothing else. The first version of the C benchmark here ran
in 0.0 ms for exactly that reason. Taking the count from outside makes the loop
unpredictable and the work real.

The same applies to sizing. A workload that takes eTamil a second takes C
microseconds, which is below the resolution of anything measuring whole
processes — the existing `scripts/bench` table records C's arithmetic as
"upper bound; below measurement resolution" for that reason. Each language here
runs at its own `N`, chosen so wall time is at least ~200 ms, and the result is
divided by `N`. That is the only way the fast end of this table is a measurement
rather than a ceiling.

## Why the table is split by arithmetic

**Every value in eTamil is a fixed-point decimal.** That is the language's
central promise — `0.1 + 0.2` is exactly `0.3`, and a ledger balances to the
paisa. Comparing that against `double` and reporting a slowdown would be
comparing different computations, not different runtimes: a `double` cannot
represent `0.05` at all, and its answer to this benchmark is wrong in the last
digits.

So results are grouped by what the arithmetic *guarantees*:

- **Exact** — eTamil (`rust_decimal`), Python `Decimal`, C# `decimal`, Rust
  `rust_decimal`, C `int64` scaled to paisa, JavaScript `BigInt`. All print the
  same digits.
- **Binary float** — Python `float`, JavaScript `number`, C `double`, Rust
  `f64`. Fast, and inexact for money.

The row that matters most is Rust with `rust_decimal`, because that is the
*same crate eTamil's VM calls*. The gap between those two lines is not
arithmetic cost; it is the cost of interpreting bytecode. Nothing else in this
table isolates that.

## Files

| | |
|---|---|
| `tax.qmz` | eTamil — decimal. Reads `N` from stdin |
| `tax_decimal.py` `tax_int.py` `tax_float.py` | Python — `Decimal`, scaled `int`, `float` |
| `tax_bigint.js` `tax_float.js` | Node — `BigInt`, `number` |
| `tax_int.c` `tax_double.c` | C — `int64` scaled, `double` |
| `tax_int.cpp` `tax_double.cpp` | C++ — the same loops at the same `/O2` |
| `tax_int.php` `tax_bcmath.php` `tax_float.php` | PHP — scaled `int`, `bcmath`, `float` |
| `java/Tax.java` | Java — `BigDecimal`, scaled `long`, `double`, by argument |
| `rust/src/bin/*.rs` | Rust — `rust_decimal`, `f64` |
| `csharp/Program.cs` | C# — `decimal` and `double`, selected by argument |
| `empty.*` | Startup only, so it can be subtracted |

C++ is a separate row from C rather than an assumed equivalent. For this loop
the two should compile to the same instructions, but "should" is the kind of
claim a benchmark directory exists to settle, so both are built and both are
timed.

Java and C# put their whole program behind a mode argument instead of a file
each. The empty program has to start the same runtime as the timed ones for the
subtraction to mean anything, and a separate class or assembly would load
something different from what is being measured.

## Getting the runtimes

`node`, `dotnet` and `python` are found on `PATH`. The rest:

| Runtime | Windows |
|---|---|
| C, C++ | `build_c.bat` — needs MSVC Build Tools |
| Rust | `cd rust && cargo build --release` |
| PHP | `winget install PHP.PHP.8.4` — zip-based, no administrator needed |
| Java | a JDK on `PATH`, then `javac java/Tax.java` |

The JDK is the awkward one: every winget JDK package is a machine-scope MSI and
wants elevation. The Temurin `.zip` from <https://adoptium.net> needs none —
unpack it anywhere and put its `bin` on `PATH`. Verify its SHA-256 against the
one the Adoptium API publishes before unpacking it.

PHP's winget package edits `PATH`, but a shell that was already running does
not see the change, so `run.py` also looks in `%LOCALAPPDATA%\Microsoft\WinGet\
Packages` before deciding PHP is missing.

Every program prints `0.05 × N(N−1)/2`. The runner checks that they agree
before it reports a timing, because a benchmark that computes the wrong thing
quickly is not a fast benchmark.
