# eTamil performance claims

Measured 2026-10-03. What the benchmark supports, what it does not, and where
each claim stops being true.

Reproduce with `python scripts/bench/compare/run.py --repeats 5 --target-ms 400`.
See [`scripts/bench/compare/README.md`](../scripts/bench/compare/README.md) for
how to get each runtime.

## The headline

**On exact money arithmetic, eTamil finishes a 100,000-operation program faster
than Python, PHP, Java or C# doing the same work.**

| Doing exact money arithmetic | Total time |
|---|---|
| **eTamil — `Decimal`** | **236 ms** |
| Python — `Decimal` | 378 ms |
| C# — `decimal` | 391 ms |
| PHP — `bcmath` | 447 ms |
| Java — `BigDecimal` | 489 ms |

A measured end-to-end result on an identical workload, not a micro-benchmark
excerpt. All five print `249997500`.

It holds because of two things working together. eTamil starts in 126 ms, where
the JVM needs 354 ms and .NET 383 ms before a line of user code runs. And its
decimal arithmetic is quick enough — 452 ns an iteration — that the startup
advantage is not eaten before the program ends.

This is the claim to lead with: a sceptical reader can reproduce it in a minute,
and it describes what people actually run — a script, a report, an invoice run,
not an hour-long loop.

## The claims that hold

The last column is the thing a critic will reach for, stated before they can.

| Claim | The number | Where it stops being true |
|---|---|---|
| Faster than Python's exact decimal, at any workload size | 452 ns vs 689 ns per operation; startup 126 ms vs 125 ms | Nowhere. Ahead on arithmetic, level on startup |
| Faster than PHP's exact decimal, at any workload size | 452 ns vs 845 ns; startup 126 ms vs 162 ms | Nowhere. Ahead on both terms |
| Starts faster than Java, Node and .NET | 126 ms vs 354, 361, 383 ms | Never; it is a fixed cost they pay every run |
| Beats Java and C# on exact money end-to-end | 236 ms vs 489 and 391 ms at 100,000 operations | ≈470,000 operations for C#, ≈680,000 for Java — estimated from the measured anchors |
| Exactness costs nothing extra | Every value is already a decimal | Not a speed claim — see the correctness tax |

### Two framings worth using

**"No correctness tax."** In Python, choosing to be right about money costs
3.2×. In PHP it costs 22.9×. In C# 8.9×. eTamil has no such cliff, because it
never offered the fast-and-wrong option.

**"Fast where programs are short."** Most money code is a script, a batch, a
request handler — thousands of operations, not millions. In that range eTamil
wins outright against every mainstream language's exact-decimal path, and the
reason is startup, which no amount of JIT warmup recovers.

## Per-iteration cost, exact arithmetic

Measured as `T(2N) − T(N)`, which cancels process creation, runtime startup and
printing. Each language calibrated to about 400 ms of its own compute.

| Implementation | ns / iteration | Observed range |
|---|---|---|
| C++ — int64 paisa † | 0.3 | 0.2 – 0.5 |
| C — int64 paisa † | 0.4 | 0.3 – 0.4 |
| Java — long paisa | 1.3 | 0.4 – 15.1 |
| Rust — `rust_decimal` | 7.8 | 7.1 – 8.3 |
| JavaScript — BigInt | 8.5 | 7.0 – 12.0 |
| Java — `BigDecimal` | 19.3 | 14.4 – 32.9 |
| C# — `decimal` | 32.9 | 22.2 – 45.3 |
| PHP — int paisa | 58.6 | 40.9 – 96.4 |
| Python — int paisa | 291.2 | 235.0 – 422.4 |
| **eTamil — `Decimal` (VM)** | **452.3** | 430.3 – 606.7 |
| Python — `Decimal` | 688.7 | 455.9 – 790.8 |
| PHP — `bcmath` | 845.0 | 726.1 – 1178.6 |

† *Loop reduced.* At `/O2` the optimiser recognised the triangular sum and
replaced the loop with a closed form, so these two measure how fast a compiler
avoids the work. Treat them as a floor, not a time.

The wide whisker on Java `long` is C2 warming up, not noise in the harness.

## The correctness tax

Exact arithmetic ÷ binary float, per language. 1.0 means no penalty.

| Language | Fast and wrong | Exact | Tax |
|---|---|---|---|
| PHP | `float` 36.9 ns | `bcmath` 845.0 ns | 22.9× |
| C# | `double` 3.7 ns | `decimal` 32.9 ns | 8.9× |
| JavaScript | `number` 1.1 ns | `BigInt` 8.5 ns | 7.7× |
| Rust | `f64` 1.3 ns | `rust_decimal` 7.8 ns | 6.0× |
| Java | `double` 5.7 ns | `BigDecimal` 19.3 ns | 3.4× |
| Python | `float` 213.6 ns | `Decimal` 688.7 ns | 3.2× |
| **eTamil** | *no float mode* | `Decimal` 452.3 ns | **1.0×** |

This is the most promotable idea in the benchmark, because it reframes the
comparison. The question is not "is eTamil faster than Java" — sometimes yes,
sometimes no. It is **what does each language charge you for being right about
money**, and there eTamil is alone at zero.

eTamil sits at 1.0 because it has no binary float to switch from: every value is
a decimal. That is the guarantee, and also the limit — there is no fast inexact
path for arithmetic that is not money.

C and C++ are left out: their exact rows were removed by the optimiser, so the
ratio would be fiction.

## Startup

The floor is 94 ms — what Windows charges to create any process at all.

| Runtime | Empty program | Over the native floor |
|---|---|---|
| C++ — MSVC `/O2 /EHsc` | 84 ms | 0 ms |
| C — MSVC `/O2` | 88 ms | 0 ms |
| Rust — opt-level 3 + LTO | 104 ms | 10 ms |
| Python — CPython 3.14 | 125 ms | 31 ms |
| **eTamil — bytecode VM** | **126 ms** | **32 ms** |
| PHP 8.4 CLI | 162 ms | 68 ms |
| Java — OpenJDK 21 | 354 ms | 260 ms |
| JavaScript — Node 24 | 361 ms | 267 ms |
| C# — dotnet 9 | 383 ms | 289 ms |

## What not to claim

A performance claim that gets refuted in public costs more than it ever won.

| Do not say | Because | Say instead |
|---|---|---|
| "Faster than C or C++" | They are 1,000× faster per operation | "C++ has no decimal type at all — you hand-roll scaled integers and own the rounding bugs yourself" |
| "Faster than Java" | `BigDecimal` is 23× faster per operation; eTamil only wins end-to-end below ≈680,000 operations | "Faster than Java for the programs people actually run" — and name the workload |
| "eTamil is fast" | Unqualified, no. 452 ns an operation is interpreter territory | "Fast enough that exactness costs you nothing" |
| Any comparison against `float` or `double` | Different computation; their answer is wrong in the last digits | Compare only against `BigDecimal`, `Decimal`, `decimal`, `bcmath` |
| Numbers from `scripts/bench/README.md` | Those are eTamil **0.2.0**, measured 2026-08-17, and the Python figure came from a pathological Microsoft Store build | Use the 2026-10-03 run |
| "Scales well" | Untested. `இணை` is documented as quadratic, and this run neither confirmed nor cleared it | Nothing, until it is measured at 16k–128k |

### The honest one-line positioning

> eTamil is the only one of these languages where money is exact by default, and
> it is faster end-to-end than every mainstream language's exact-money path at
> the sizes real programs run.

Every clause is measured, and the qualifier at the end is what keeps it true.

## How to close the gap

Rust calling `rust_decimal` does this arithmetic in 7.8 ns; eTamil does it in
452 ns. Both call the same crate, so **every one of those 444 ns is the
interpreter, not the maths.**

### Step 0 — make regression impossible before optimising anything

Wire `scripts/bench/compare/run.py` into CI with a threshold on `ns_per_iter`,
failing on a regression beyond noise. The harness already emits JSON for this.

### Step 1 — profile on Linux before choosing

Everything below is a hypothesis until a profile ranks it.

```bash
cargo install flamegraph
cd /srv/etamil/eTamil/etamil_compiler
cargo flamegraph --release -- --vm ../scripts/bench/compare/tax.qmz
```

### The work, in expected-payoff order

| # | Change | Why this one | Evidence it will pay |
|---|---|---|---|
| 1 | Resolve variables to slot indices at compile time | If scopes are `HashMap<String, Value>`, every read hashes a string | `loop_vars.qmz` isolates exactly this; compare against `loop_only.qmz` |
| 2 | Shrink and un-box `Value` | `Decimal` is 16 bytes; if `Value` is larger, or allocates per operation, that cost is paid on every instruction | `size_of::<Value>()`, and whether the hot path calls `clone()` |
| 3 | Threaded dispatch instead of a `match` in a loop | 75 ns per instruction, flat whether or not it does arithmetic | `loop_only.qmz`: 900,000 instructions in 68 ms |
| 4 | Superinstructions for the commonest opcode pairs | Removes dispatches outright rather than making each cheaper | Add an opcode-pair histogram; fuse the top five |
| 5 | Register VM in place of the stack VM | Removes push/pop traffic that superinstructions only paper over | Largest change; do it last, and only if 1–4 fall short |

A tuned bytecode VM runs 5–15 ns an instruction. eTamil is at 75. Steps 1–4 are
the ordinary route from one to the other, and none of them touch the language.

### Separately: the algorithmic bug that outweighs all of it

`இணை` in `interpreter.rs` does `items.clone()` then `push`, so building a list
of *n* items copies about *n*²/2 elements. For the accounting framework — which
builds ledgers by appending — this dominates every dispatch saving above.

One caveat: the 2026-10-03 run printed 31 / 14 / 26 / 26 ms across
N = 2,000 → 16,000, which is **not** quadratic. Either this was already fixed or
those sizes are below the noise floor. Re-measure at 16,000–128,000 before
spending effort on it.

### The LLVM backend is not yet the answer

It builds and it is parity-clean: on 2026-10-03 it compiled against LLVM 18.1.3
on the droplet and `run_parity.sh` found no disagreement anywhere it accepted a
program. Its remaining refusals are **only I/O** — file open/read/write/close,
CSV read/write, database disconnect — nothing arithmetic, nothing control flow.

**But measured today it is slower than the VM.** Compiled to IR, linked against
`libetamil_compiler.so` and run on the droplet:

| On the droplet, 2 vCPU | ns / iteration | Startup |
|---|---|---|
| eTamil — bytecode VM | 1,225 – 1,305 | 32 ms |
| eTamil — LLVM-compiled | 2,085 – 2,185 | **5 ms** |

Both print `249997500`, so exactness survives. The reason for the loss is
visible in the IR: every operation is a call to `etamil_add`, `etamil_subtract`,
`etamil_multiply` or `etamil_compare`, and every value is an opaque `i64`
handle. The backend removes bytecode dispatch and replaces it with a
cross-library call per operation, which on this workload costs more than the
dispatch it saved.

That is the boxed value [`CONTINUATION.md`](CONTINUATION.md) names as the gap
everything waits on, and this is the measurement that proves it. Until values
are unboxed into native registers so LLVM can inline the arithmetic, the backend
cannot approach the 7.8 ns line — it is paying a function call where Rust pays
an instruction.

The 5 ms startup is real and worth having: a compiled eTamil program starts six
times faster than the VM.

### What to leave alone

Startup. At 126 ms — 32 ms over the cost of launching a do-nothing native binary
— it already beats PHP, Java, Node and .NET and ties CPython. There is nothing
to win and a working advantage to lose.

## Method

Every figure measured on one machine on 2026-10-03: Intel Core Ultra
(Family 6 Model 170), Windows 11, pinned to one core at high priority, minimum
of five runs. The workload is 100,000 slab-tax calculations; all twenty
implementations must print `249997500` before a timing is accepted. The LLVM
rows are from the droplet (2 vCPU, Ubuntu 24.04.5) and compare only against the
VM on that same machine.

Two harness choices matter more than they look:

- **The loop bound comes from outside the program** — `argv`, or stdin for
  eTamil, which has no `argv`. With a literal bound, C and Rust evaluate the
  whole loop at compile time and print a constant.
- **Each language runs at its own N**, calibrated to about 400 ms of its own
  compute, with the result divided by N.

### Limits

- **One machine, one OS.** Windows process creation costs 94 ms; Linux is far
  cheaper, which would compress the startup table and help the runtimes that
  currently look worst.
- **Java is measured cold-ish.** C2 gets the calibration run to warm up, which
  is why `long paisa` shows a 0.4–15.1 ns spread.
- **One workload.** A tight arithmetic loop says nothing about allocation,
  strings or I/O.
- **The array-append curve is unresolved**, as above.
