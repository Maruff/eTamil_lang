# The LLVM backend: what it does, and what it still refuses

Last rewritten when the backend stopped holding values in registers. The
history matters here, because the shape of the thing is a reaction to two
earlier attempts that were wrong in the same way.

## Two versions that traded the language for a register

**It computed in `f64`.** Every number in eTamil is a fixed-point decimal, and
the reason is the language's opening claim: `0.1 + 0.2` is exactly `0.3` and a
ledger balances to the paisa. Compiled through `--llvm` it was not, and nothing
said so — the IR was emitted, it ran, and it answered `0.30000000000000004`.

**Then it computed in `i64`.** Better: exact to 2^63 instead of 2^53, and
`sdiv` under `தரை` was exactly a floor where `fdiv` could be out by a paisa.
That version is what `examples/finance/pYcA_kaNakku.qmz` was written for, and
it worked — 7 of 68 examples matched the VM, none disagreed.

But both were the same bargain in different clothes. The IR held the value
directly, so the language shrank to whatever fits in a register. Numbers fit.
Strings, arrays, records, results and `இன்மை` did not. Neither did a decimal:

```
1 / 3   →   0.3333333333333333333333333333    on the VM
```

There is no register that holds that, so `i64` had to refuse any fractional
literal and any bare division. Which is most of what a tax calculation is.

### The obvious workaround, and why it was not taken

Fixed two-place decimals — every number an `i64` scaled by 100, which is what
the paise convention in `nUlakam/paNam/kAcu.qmz` already does by hand. Measured
against the VM before committing to it:

| | fixed 2-place | VM |
|---|---|---|
| `0.1 + 0.2` | `0.3` | `0.3` ✓ |
| `1 / 4` | `0.25` | `0.25` ✓ |
| `1000 * 18%` | `180` | `180` ✓ |
| `1 / 3` | **`0.33`** | `0.3333333333333333333333333333` ✗ |

Three out of four is not the standard here. The fourth is a silent wrong
answer, so inexact division would have to stay refused — and then the workaround
buys fractional literals and nothing else, while strings, arrays, records and
results stay out of reach because the IR has no heap to allocate them from.

## What it does now: the IR stops holding values

Every eTamil value in the emitted IR is an `i64` **handle** into an arena that
lives in `src/runtime.rs`, and every operation on one is a C-ABI call into that
module. The IR carries the control flow — branches, loops, calls, returns — and
knows nothing about what a value is.

That is the ordinary way to compile a dynamically typed language. It costs a
call per operation and buys three things, none of them a coincidence:

1. **Decimals are exact, because the runtime is the same `rust_decimal` the VM
   uses.** Not an approximation of the type — the type. `1 / 3` prints all
   twenty-eight digits in both backends for the same reason rather than by
   luck.
2. **Formatting cannot drift**, because printing calls `Value::to_string`, the
   function the VM prints through. Trailing zeros are trimmed in one place, not
   two: `1000 * 18%` is `180.00` by scale and prints as `180` on both sides.
3. **Every builtin works at once**, because `etamil_call` dispatches
   through `VM::invoke_builtin` — the interpreter's own table. `நீளம்` is not
   reimplemented, so it cannot disagree.

`index_of` and `invoke_builtin` are public on the VM for exactly this reason:
the two backends reach the same code, rather than two implementations that
agree until someone edits one of them.

### What that removed

`src/codegen_limits.rs` is gone. It existed to refuse, before any IR existed,
the arithmetic the backend would get wrong — fractional literals, bare
division, whole numbers past `i64`. None of those is wrong any more, so a module
whose entire content was that policy had nothing left to say. Its `தரை`/`மேல்`/
`வட்டமிடு` table went with it: all three are ordinary builtins now, along with
the other fifty-six.

`codegen.rs` also lost `ArrayInfo`, the arrays and records maps, the
double-versus-integer duality, the hand-rolled floor division with its sign
correction, the division-by-zero guard blocks, and the whole
print-the-pieces-of-a-concatenation machinery. `&` is a real operation now, so
`அச்சு "விலை: " & 2.05` is one call. The file is shorter than the version it
replaced.

## The cost, stated plainly

**`output.ll` is no longer self-contained.**

```bash
etamil --llvm myprogram.qmz
clang output.ll -o myprogram \
      -L etamil_compiler/target/release -letamil_compiler \
      -Wl,-rpath,etamil_compiler/target/release -lm
```

`Cargo.toml` already built the `cdylib`, so there is no new artefact — only new
exported symbols in it. But a compiled program now ships with
`libetamil_compiler.so` beside it, and `-rpath` is what lets it find the library
again at run time. `scripts/run_parity.sh` does this, and prints clang's own
error if the link fails, because "clang rejected the IR" on its own is not
something anyone can act on.

**Nothing in the arena is ever freed.** A compiled program runs and exits, and
an arena that only grows cannot dangle, cannot double-free, and needs no
reference counting in the emitted IR. A long-running program compiled this way
would grow without bound. That is the honest limit of the approach and the
reason the VM remains the way to run a server.

## What is still refused

Statements, not expressions. Files, HTTP, routes, scheduling, `இறக்கு` at the
point codegen sees one, and the database statements past connecting, executing
and querying. Each is named
individually by `stmt_label` so that `run_parity.sh` can rank them, and each
needs the VM's own machinery rather than a value representation — a route is not
a value problem.

### What is built: `தளம்_இணை`, `தளம்_செய்`, `தளம்_வினா`

A query is a value problem after all. The SQL and the parameter list are
ordinary expressions, the answer is an array of records, and the only thing the
IR has to carry that it did not already is *which connection* — a C string for
a named one, a null pointer for "the only one open", which is the shape
`Option<&str>` takes across the ABI.

`etamil_db_query` in `runtime.rs` asks **the VM's own connection registry**, on
the same `HOST` the builtins dispatch through. Two registries would agree until
somebody edited one of them, and "which connection is the only one open" is
exactly the kind of question the two backends must not answer differently.

That half is tested here rather than argued about: `runtime.rs` opens an
in-memory SQLite database on that `HOST`, runs a query through the C entry
point, and asserts the rows — including a bound parameter and the null-handle
case. It is the reason the value semantics live in `runtime.rs` and not in the
emitted IR.

`தளம்_இணை` borrows through the same pool the VM borrows through, because under
`--server` this statement is reached once per request and opening each time
would cost a connect, a handshake and an authentication round trip per request.
An unnamed connection takes the driver's name as its handle — the default the
bytecode compiler applies too, so both backends file it under the same key and a
later statement naming nothing finds it under either. Pointing one handle at a
*second* database is refused in the runtime's words as in the VM's: the map is
keyed by handle, so a second insert used to overwrite the first silently and
every query after it went somewhere the program did not think it was going.

`தளம்_செய்` throws the row count away, which is the VM's behaviour rather than
an oversight here — `தளம்_செய்` is the statement form, and the count is reached
through `தளம்_செய்_முயற்சி`, which returns a result.

With all three built, a whole database conversation can be driven through the C
entry points on a machine that cannot build the IR, and six tests in
`runtime.rs` do exactly that: connect, create, insert with bound parameters,
read back; an unnamed connection found through a null handle; and a second
connect to the same database being the no-op a per-request server needs.

What remains is `தரவுசேமி_பிரி`. It is the sole blocker for three programs and
the last statement in the cluster.

Refusing remains the discipline. IR that dropped a statement, or evaluated an
expression as a placeholder, would make a compiled program quietly disagree
with the same source run on the VM, and that is the one failure this project
does not accept. `main.rs` refuses to emit when the list is non-empty.

## Measured, on Ubuntu — the last run with LLVM present

Superseded in scope by the burn-down below, which counts 111 programs rather than
these 68. This is still the only measurement that has actually *run* the compiled
code, and so the only one that can report a mismatch.

`scripts/run_parity.sh`, LLVM 18, clang present so every accepted program was
compiled *and run*:

```
51 match, 2 mismatch, 11 refused, 0 compiled-only, 4 skipped
all 68 accounted for
```

The previous design — values in `i64` registers — scored `7 match, 0 mismatch,
57 refused`. So the runtime bought 44 programs, and no module was rejected by
clang's verifier in either run.

What matches now is most of the library: `nUlakam/paNam/kAcu.qmz` and its suite,
accounting, tax, insurance, customs, banking, UPI addresses, Redis, JSON, the
document builder, and the test framework they are all written against. Money to
the paisa, compiled, agreeing with the interpreter line for line.

### The two that disagreed

A refusal is expected. A disagreement is the one failure this project does not
accept, so both are worth stating precisely.

**`examples/basic_samples/example.qmz` — mine, and fixed.** `உள்ளிடு வருவாய்1;`
reads a line on the VM and stores it. This backend printed the variable's
current value as a "prompt" and never read at all, so the compiled program
emitted `nil` and left its input unconsumed. The prompt-printing was invented
here rather than read out of the VM's bytecode, which prints no prompt and
stores only when the operand is a bare name. `codegen.rs` now does exactly
that, and `etamil_prompt` is gone from the runtime because nothing calls it.

Worth keeping in view: `உள்ளிடு "prompt" & பெயர்` reads a line and **throws it
away** on both backends, because the VM stores only for a bare name. That is a
language wart, not a backend one, and it is now faithfully reproduced rather
than papered over on one side.

**`nUlakam/upi/upi_cOqaZY.qmz` — open.** The suite passes 45 of 45 on the VM, so
exactly one assertion or one printed line differs. Reading the code did not find
it: the constructs it uses that the eight *matching* suites do not are a
`திரும்பு` inside a `ஒவ்வொரு`, a record indexed by a string key, and
`அல்லது` chains returned directly — and each of those looks right on inspection.
Inspection is how the `உள்ளிடு` bug got written in the first place, so the next
step is to look rather than reason:

```bash
./scripts/run_parity.sh --diff nUlakam/upi/upi_cOqaZY.qmz
```

That runs one program under both backends and prints where their output parts
company. The summary form says only *that* two backends disagree, which made
every mismatch cost a round trip.

## The burn-down, countable anywhere

`run_parity.sh` is the measurement that settles anything, and it needs LLVM 18
and clang. That made the gap a number you could only learn on the Ubuntu box —
which is the one place you do not need to be told what to build.

The list of unbuildable constructs is a pure function of the AST, so it has
been lifted out of `#[cfg(feature = "llvm")]` and is reachable everywhere:

```bash
etamil --llvm-gaps nUlakam/paNam/kAcu.qmz        # exits 0: nothing refused
etamil --llvm-gaps examples/api/simple_api.qmz
#   4  வழி (a route)
#   1  சேவையகம்_தொடங்கு (start a server)
#   1  சேவையகம்_நிறுத்து (stop a server)

python3 scripts/llvm_gap_report.py         # the whole corpus, ranked
```

Statements only. The backend also refuses a name nothing in its own scope
defines, and operators today's parser does not build, so a clean report is an
upper bound rather than a promise.

### Where it stands

The corpus is 111 programs now rather than the 68 measured below — nUlakam has
roughly doubled since. **99 would compile; 12 are refused.**

| construct | programs | sole blocker |
|---|---|---|
| `தரவுசேமி_பிரி` (disconnect) | 3 | **3** |
| `வழி` (a route) | 3 | **1** |
| `கோப்பு_திற` / `கோப்பு_மூடு` | 6 | 0 |
| `CSV_படி` / `CSV_எழுது` | 4 | 0 |
| `கோப்பு_படி` / `கோப்பு_எழுது` | 3 | 0 |
| `இடைவெளி`, `சேவையகம்_தொடங்கு`, `சேவையகம்_நிறுத்து` | 1 each | 0 |

**Sole blocker is the column that moves the total.** A construct appearing in
seven programs buys nothing if six of them are also waiting on something else;
what ships a program is being the last thing it waits on.

### Three clusters, and then it is zero

Every one of the sixteen is waiting on one of three groups, and no program
straddles two of them:

**Database — 3 programs, 1 statement.** `தரவுசேமி_பிரி`, and it is the sole
blocker for all three. The cluster was 7 programs and 4 statements; connecting,
executing and querying are built, and with them `vari_vikiqam_cOqaZY.qmz` and
`coqqu_cOqaZY.qmz` — the two suites that actually *run* queries rather than
merely containing them. Whatever the next parity run says about the database
path, it will be saying it about code that executed.

**Files and CSV — 6 programs, 6 statements.** `கோப்பு_திற`, `கோப்பு_மூடு`,
`கோப்பு_படி`, `கோப்பு_எழுது`, `CSV_படி`, `CSV_எழுது`. No partial credit here:
every one of the six programs opens and closes, so nothing ships until at least
four of the six exist.

**Server and scheduling — 3 programs, 4 statements.** `வழி`,
`சேவையகம்_தொடங்கு`, `சேவையகம்_நிறுத்து`, `இடைவெளி`. The hardest of the three
and the least valuable: a compiled binary that serves HTTP needs the whole
runtime the VM already is, and the arena in `runtime.rs` never frees — which is
exactly the property a long-running server cannot have. **This cluster is the
one that should stay refused**, and saying so is what turns "16 refused" into
"13 to build and 3 by design".

So the road to zero, in order: `தரவுசேமி_பிரி` (3 programs, and the cluster is
finished), files and CSV (6 programs, 6 statements, no partial credit), and then
a decision about the server group rather than an implementation of it.

## Re-measuring

```bash
cd etamil_compiler && cargo build --release --features llvm && cd ..
./scripts/run_parity.sh
```

The last line must read `all 68 accounted for`. If it does not, the run stopped
early and nothing above it is a measurement.

## Type-checking on a machine that cannot build it

`codegen.rs` cannot be built without LLVM 18, but it can be *type-checked*
anywhere, because `llvm-sys`'s own source sits in the cargo registry cache even
where it cannot be compiled:

```bash
python scripts/check_llvm_backend.py
```

It generates a signature-only crate from llvm-sys's source — the same
declarations, nothing behind them — points cargo at it through a path override,
runs `cargo check --features llvm --all-targets`, and puts the checkout back
afterwards including `Cargo.lock`. `--clean` undoes an interrupted run.

**Check that it is really looking at `codegen.rs` before trusting a clean run.**
Put a deliberately undefined LLVM symbol in the file and confirm the error: a
`cfg`'d out file passes everything, and this whole file is `cfg`'d out.

This catches every type error and every borrow error. It cannot catch invalid
IR, and it cannot catch a wrong answer.

**The runtime, though, is testable here** — and that is the main reason the value
semantics live in `src/runtime.rs` rather than in the emitted IR. The part most
likely to be wrong is now the part that runs under `cargo test` on any machine:
`0.1 + 0.2` is `"0.3"` and `1 / 3` is all twenty-eight digits, asserted rather
than argued.
