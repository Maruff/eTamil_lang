# artino — eTamil on Arduino boards, through the LLVM backend

> **Status: design, not implemented.** Nothing here exists in the compiler yet. This page records
> the decisions taken on 27 Sep 2026 and the plan for building them. When the code and this page
> disagree, the code wins.

artino compiles an eTamil program for an Arduino board through the **same LLVM backend** that
compiles it for a desktop, links it with the board's Arduino core, and uploads it:

```bash
etamil --artino --board uno --upload COM5 kaNu.qmz
```

It is one of two tracks with one aim: **one compiler back end for every place eTamil runs, and no
program whose compiled output disagrees with the VM.**

- **Track A — close the desktop gap.** `--llvm` builds 99 of the 111 programs in the corpus today
  ([llvm-backend-gaps.md](llvm-backend-gaps.md)). The other 12 wait on eleven statements.
- **Track B — artino.** The same backend gains microcontroller targets: AVR (Uno, Nano, Mega) and
  ARM Cortex-M (Pico, Pico 2).

The first real target is the qIkkAppu fire alarm demo: its detector node and panel, today written in
C++, rewritten in eTamil and passing the same acceptance tests.

## Why LLVM, and what that costs

An earlier draft of this page generated Arduino C++ text instead. Going through LLVM was chosen so
that control flow, functions, shapes, results and every future language feature are lowered **once**,
and a fix to the backend reaches the desktop and the boards together.

| | Through LLVM (chosen) | Generating C++ text (earlier draft) |
|---|---|---|
| Code shared with `--llvm` | Parsing, checking, control flow, calls, shapes, gap walk | Parsing and checking only |
| Arduino libraries | Through generated `extern "C"` shims | Called directly |
| When something is wrong | Debug info maps machine code to `.qmz` lines | Readable C++ with `#line` |
| Windows development | LLVM does not build on the Windows machine today; needs WSL2 or a Windows LLVM build (A6) | Works anywhere `arduino-cli` does |
| Effort to first firmware | Higher: a toolchain spike (B0) comes before anything else | Lower |

The last two rows are the price. B0 exists to find out early whether the price is acceptable.

## Two ways the backend holds a value

**Desktop (today): handles.** Every value in the IR is an `i64` handle into an arena in
`src/runtime.rs`, and every operation is a call into the VM's own code (`VM::invoke_builtin`,
`Value::to_string`, `rust_decimal`). That is why desktop parity is high: the two backends run the
same code. It cannot go onto a board: the runtime is the VM, and the arena never frees.

**Microcontroller (new): static values.** With `--artino`, each value has a fixed representation the
backend knows when it compiles:

| eTamil | LLVM |
|---|---|
| number | `i64`, the value × 1000 (see Numbers) |
| boolean | `i1` |
| text | a fixed-capacity byte buffer plus length |
| array | a fixed-length LLVM array |
| `வடிவம்` shape | an LLVM struct; methods take a pointer to it |
| `சரி` / `தவறு` result | a struct: tag, value, error text |

**Where the code lives.** The first draft of this page gave `Compiler` a `Lowering` choice between
the two. Writing B1 showed why that does not fit: every function in `codegen.rs` returns a handle and
calls the runtime to operate on it, so a second mode would have touched almost every line of a
backend that works, for the benefit of a representation it does not use. So the static lowering is
its own module, `src/artino/`:
- `analyse.rs` types the program and names what it refuses. It is pure Rust, so `--artino-gaps` and
  the unit tests run on any machine.
- `emit.rs` lowers the analysed program to LLVM.
- `sketch.rs` writes the Arduino sketch folder and the precompiled library.

What the two share is everything before lowering: the lexer, the parser, `check.rs`, the module
loader, and the statement labels the gap reports use. A construct artino cannot build is refused by
name, never emitted as a placeholder.

This is the "values in registers" design that llvm-backend-gaps.md retired for the desktop. It
failed there because a register cannot hold a 28-digit decimal or a growable string. It works here
because artino's subset is defined so that everything *does* have a fixed size, and its number type
is defined differently, as follows.

## Numbers

**Decision: a signed 64-bit integer holding the value × 1000 — three decimal places.** The range is
±9,223,372,036,854,775.807 (about 9.2 × 10¹⁵). Addition, subtraction and comparison are exact.

This project does not accept a silent wrong answer, and three places cannot hold `1 / 3`. So:

1. **Multiplication and division are exact whenever the true result has at most three decimals**:
   `1000 * 18%` is `180`, `7 / 2` is `3.5`.
2. **Otherwise the result is rounded half away from zero** — the rule `வட்டமிடு` uses on the VM —
   **and the rounding is reported**, once per place in the program, on the serial port:
   `artino: தொடக்கம், வரி 16: 2 / 3 — needed more than three decimals; rounded (the VM keeps 28
   digits)`. Every report is also counted (`artino_report_count`), including repeats that are not
   printed again.
3. **Rounding the author wrote is not reported.** `வட்டமிடு(…, n)`, `தரை(…)` and `மேல்(…)` are
   computed exactly as asked.
4. **Overflow saturates and is reported.** Nothing wraps: `+` and `-` go through the runtime too.
   Only a sum of two literals is computed by the compiler, which is how `-5` (parsed as `0 - 5`)
   costs nothing.
5. **Division by zero gives 0 and is reported.** The VM stops with an error there. A board has
   nowhere to stop to.

A report names the operation by its text, because the AST carries no position for an expression. It
adds the nearest line the AST does know: an assignment's line, or else the function's. So
`அச்சு அ / ஆ;` in a loop reports as `தொடக்கம்: அ / ஆ`, and an assignment reports as
`தொடக்கம், வரி 16: 2 / 3`. Giving every expression a position is a parser change for the language
as a whole, and would make every report exact.

Firmware that counts, compares and scales never triggers a report, and the conformance suite holds
it to the VM's printed output.

## Program layout

| Written in eTamil | Runs | Becomes |
|---|---|---|
| Statements at the top of the file | Once, at power-on | `artino_setup()`, called from `setup()` |
| `இடைவெளி 0.1 { … }` | Every 0.1 s; several blocks at different rates, none blocking another | A `millis()` timer in `artino_loop()` |
| `செயல் சுழற்சி() { … }` (optional) | As often as `loop()` runs | Called from `artino_loop()` |

`இடைவெளி` already means "every N seconds" under `--server` (`Stmt::Schedule`). There is no blocking
wait: a `delay()` stops every other block, which on a fire panel is a button press never seen.

- **`இடைவெளி 0 { … }` runs every time round the loop**, which is how firmware reacts as fast as it can.
- **Each block first runs on the first pass of the loop**, not N seconds after power-on.
- **The period is a number written in the source.** A period computed at run time is refused.

**Where state lives.** As on the VM, a name assigned inside a `செயல்` is that function's own local:
a function reads the program's variables but cannot change them. It returns a value, and the top level
or an `இடைவெளி` block stores it. The blocks are not functions, so they can change the program's
variables, and that is where firmware keeps its state: counters, baselines, alarm flags.
`சுழற்சி()` is a function, so it suits work that keeps no state. For stateful work every loop, use
`இடைவெளி 0`.

```
இறக்கு "nUlakam/vaZporuL/vaZporuL.qmz";

நிலை எண் விளக்கு = 13;
முனை_வெளியீடு(விளக்கு);            // once
தொடர்_தொடங்கு(115200);

இடைவெளி 0.5 {                       // every half second
    முனை_மாற்று(விளக்கு);
    அச்சு "புகை: " & ஒப்புமை_படி(0);
}
```

(Hardware function names are placeholders; see "The hardware module".)

## Toolchain: from `.qmz` to a board

```
.qmz → lexer → parser → check.rs → gap walk → codegen (Static) → LLVM object for the chip
                                                                          │
                          artino-rt (C) + vaZporuL shim + library shims ──┤
                                                                          ▼
      generated sketch folder:  prog.ino  +  libartino_prog (precompiled Arduino library)
                                                                          │
                                                          arduino-cli compile / upload
```

1. **Target machine.** codegen sets the triple and CPU from `--board`, and emits an object file
   directly through LLVM's target API, without `llc`:

   | `--board` | Triple / CPU | Arduino FQBN |
   |---|---|---|
   | `uno`, `nano` | `avr` / `atmega328p` | `arduino:avr:uno`, `arduino:avr:nano` |
   | `mega` | `avr` / `atmega2560` | `arduino:avr:mega` |
   | `pico` | `thumbv6m-none-eabi` / `cortex-m0plus` | `rp2040:rp2040:rpipico` |
   | `pico2` | `thumbv8m.main-none-eabihf` / `cortex-m33` | `rp2040:rp2040:rpipico2` |

2. **Precompiled library.** The object goes into an Arduino library marked `precompiled=true`, under
   the folder named for the chip (`src/atmega328p/`, `src/cortex-m0plus/`, …). That is Arduino's
   own mechanism for shipping compiled code, so `arduino-cli` links it with no custom build rules.
3. **The sketch.** A generated three-line `prog.ino` calls `artino_setup()` and `artino_loop()`.
4. **artino-rt.** A small C runtime compiled by `arduino-cli` beside the sketch:
   - fixed-point multiply and divide with rounding reports;
   - text and array operations with truncation reports;
   - number printing, formatted to the VM's rules (trailing zeros trimmed);
   - the `இடைவெளி` timers.

   **MIT-licensed** (see Licence).
5. **Shims.** Generated `extern "C"` C++ files: one for `vaZporuL` over the Arduino core API, one per
   manifest entry over a C++ library. These are the only C++ artino writes, and they contain no
   program logic.

**The calling-convention boundary is C.** LLVM's AVR backend and avr-gcc agree on the C calling
convention; C++ classes and name mangling never cross it. B0 proves this on hardware before any
backend work is done.

## What compiles, and what is refused

| Compiles in `Static` lowering | Refused, with reason and hint, by `--artino-gaps` |
|---|---|
| numbers, booleans, `இன்மை` | database, HTTP, routes, server statements |
| `மாறி`, `நிலை`, typed declarations | files and CSV |
| `செயல்` with parameters and returns | crypto, JWT, bcrypt, JSON |
| `எனில்` / `இன்றேல்`, `சுற்று`, `ஒவ்வொரு … இல்` | records whose keys are computed (`பொருள்[சாவி]`) |
| `வடிவம்` shapes, methods, constructors | `இணை` on an array with no fixed capacity |
| arrays of fixed length, `அணி_நிரப்பு` | lambdas that capture variables |
| text of fixed capacity, `&` | `உள்ளிடு` — read the serial port instead |
| `சரி` / `தவறு` and `?` | `இறக்கு` of a module using anything in this column |
| `அச்சு` (to serial), `இடைவெளி`, `சுழற்சி` | |

**No allocation after `artino_setup()`.** Every value's size is known when the program is compiled,
so a board cannot run out of memory an hour into a shift.

- **Text** capacity is 48 bytes unless the manifest says otherwise. A `&` that would overflow it
  truncates and reports. Literals live in flash. Tamil stays UTF-8 and prints correctly over serial.
- **Arrays** take their length from their literal, or from a new constructor
  `அணி_நிரப்பு(மதிப்பு, எண்ணிக்கை)`, added to the VM too so one program runs on both. `இணை` works on
  such an array up to its length.

## The hardware module: `nUlakam/vaZporuL/vaZporuL.qmz`

வன்பொருள் ("hardware") has Tamil, romanised and `_english` names, like every builtin. It covers:
- pins: mode, digital read and write, toggle;
- analog read;
- milliseconds since power-on;
- tone;
- serial: begin, write, read a line with timeout, available;
- watchdog: enable, reset.

**Programs call `vaZporuL`'s functions, never the host builtins.** On the VM, each function is a thin
layer over a host builtin, called by its `_english` alias: `_pinMode`, `_pinWrite`, `_pinRead`,
`_analogRead`, `_millis`, `_sleepMs`, `_serialOpen`, `_serialReadLine`, `_serialWrite`,
`_serialClose`, `_boardName`, and the `_sim*` controls. Under artino, the compiler maps the same
functions to the shim. A protocol written once therefore runs on a Raspberry Pi under the VM and on a
board under artino, whatever Tamil names the builtins end up with.

**Each board has a file of its pin names** beside it: `yUnO.qmz`, `nAnO.qmz`, `mekA.qmz`, `pIkO.qmz`
(Pico and Pico 2) and `rAspY.qmz` (Raspberry Pi, under the VM). Each imports `vaZporuL.qmz` and binds
the same names, for example `விளக்கு_முனை`, `ஒப்புமை_0`, `ஒப்புமை_மில்லிவோல்ட்` and `தொடர்1_துறை`. A program imports
its board's file and names pins rather than numbers:

```etamil
இறக்கு "nUlakam/vaZporuL/pIkO.qmz";

முனை_வெளியீடு(விளக்கு_முனை);
இடைவெளி 0.5 { முனை_மாற்று(விளக்கு_முனை); }
```

Each board file binds `நிலை சொல் பலகைக்_கோப்பு` to the boards it describes, and artino refuses a
build whose `--board` is not one of them. `yUnO.qmz` built for a Pico would otherwise drive the Uno's
pin numbers.

Board state lives in the host, not in the module, because an eTamil function cannot change a module's
variables. `ETAMIL_BOARD=sim` gives a **simulated board** on any machine:
- pins are values held by the host;
- analog inputs return what a test sets;
- time moves only when the test says;
- serial ports are named `sim:<name>`.

The VM's board is `src/vm/board.rs` (B5): the simulated board on any machine, a Raspberry Pi's pins
through the Linux GPIO character device (BCM numbers, one line request a pin), and serial ports by
device path on Linux and macOS (termios, raw, a line at a time). Windows has the simulated board only.
Firmware logic can be tested with `etamil --vm` and `nUlakam/cOqaZY/cOqaZY.qmz` before it is uploaded
(`nUlakam/vaZporuL/vaZporuL_cOqaZY.qmz`).

The library's own rules:
- **Pin and time calls return plain values.** A pin that cannot be driven stops the program with the
  host's message.
- **Opening a serial port returns `சரி`/`தவறு`**, because a port can be unplugged at any time.
- **`காத்திரு` (a blocking pause) is for Raspberry Pi programs only**; artino refuses it in favour of
  `இடைவெளி`.
- **A Raspberry Pi has no analog inputs.** `ஒப்புமை_படி` there is refused with a pointer to an
  external ADC.

## Calling C++ libraries

**Decision: a manifest first, syntax later.** A program's C++ is declared in `<name>.artino.toml`
beside it, or else `artino.toml` in its folder (`src/artino/manifest.rs`):

```toml
[[library]]                 # an Arduino library the sketch needs installed
name = "Servo"
boards = ["uno", "nano", "mega"]    # on any entry: only for these boards

[[include]]
header = "Servo.h"

[[object]]                  # a global the calls use
cpp = "Servo kaqavu;"

[[function]]                # an eTamil name for a C++ call
etamil = "கதவு_திருப்பு"
cpp = "kaqavu.write"       # called with the arguments…
args = ["int"]              # int | num | bool | text
returns = "void"            # void | int | num | bool | text; void when left out

[[function]]
etamil = "விளக்கு_நிறம்"
cpp = "valayam.setPixelColor({0}, valayam.Color({1}, {2}, {3}))"   # …or a template
args = ["int", "int", "int", "int"]

[[port]]                    # a Stream as serial port 1–3, for தொடர்_திற and the rest
number = 1
cpp = "bus"                 # an [[object]] with begin(baud)
boards = ["uno", "nano"]
```

Each `[[function]]` becomes an `extern "C"` shim in the sketch's `artino_shims.cpp`, called by
number (`artino_x_0`, …), because an eTamil name is not a C one. Values cross the boundary like this:

| Kind | Into C++ | Back |
|---|---|---|
| `int` | `int32_t`, the number's whole part. A fraction written in the source is a compile error. One computed at run time is reported (`fraction`), and so is a number past int32. | × 1000 |
| `num` | `double`, the number ÷ 1000 | × 1000, rounded half away from zero |
| `bool` | `bool` | `bool` |
| `text` | `const char *` to the program's buffer | `const char *` or `String`, copied into a 49-byte buffer and cut to fit |

A `[[port]]` makes a Stream one of the program's serial ports. The qIkkAppu node uses it for
SoftwareSerial, as port 1 on an Uno or Nano, the number a Pico's Serial1 has, so one program uses
port 1 on all of them. A `[[port]]` may not take a number the board already has.

The manifest's libraries are not installed for you. arduino-cli needs them in its sketchbook, and
when a build fails `etamil` prints the `arduino-cli lib install` line for each. A program that calls
C++ runs only under artino: the VM has no C++ to call. A `வெளி செயல்` declaration syntax may follow
once the manifest has been used on more libraries.

## Names and errors

- **Symbols.** Internal symbols keep their Tamil names in UTF-8, under a prefix with a dot in it:
  `et.f.<name>` for a function, `et.g.<name>` for a variable, `et.every.<n>` for a block. No source
  identifier can contain a dot, so nothing a program names can collide with the runtime or the
  Arduino core, and a map file shows the name the author wrote. (Romanised ASCII names were only
  needed when the output was going to be C++ source.) Only `artino_setup` and `artino_loop` are
  exported.
- **Errors.** Errors in eTamil source are reported by eTamil, before LLVM, in the compiler's usual
  form: Tamil, then English, with line and column. User code is never compiled as C++, so no C++
  error can point into it. The object carries DWARF line information, so a debugger or `addr2line`
  maps a fault address back to a `.qmz` line.

## Licence

The compiler stays AGPL-3.0-or-later. **artino-rt and the generated shims are MIT**, and compiled
programs carry no licence of eTamil's. Firmware built with artino, including closed-source commercial
firmware, is its author's to license — as a program compiled by GCC is.

## Track A: closing the desktop gap

From the burn-down in llvm-backend-gaps.md, in the order that ships programs soonest:

| Step | Builds | Programs gained | Notes |
|---|---|---|---|
| A1 | `தரவுசேமி_பிரி` (disconnect) | 3 | Sole blocker for all three; finishes the database cluster |
| A2 | `கோப்பு_திற` `கோப்பு_மூடு` `கோப்பு_படி` `கோப்பு_எழுது` `CSV_படி` `CSV_எழுது` | 6 | No partial credit: all six programs open and close |
| A3 | The open mismatch in `nUlakam/upi/upi_cOqaZY.qmz` | — | Found with `run_parity.sh --diff`; a disagreement outranks any refusal |
| A4 | **Memory that is given back**: an arena scope per request and per `இடைவெளி` run, freed when the scope ends | — | The prerequisite for A5. Today's arena only grows, which a long-running program cannot survive |
| A5 | `வழி`, `சேவையகம்_தொடங்கு`, `சேவையகம்_நிறுத்து`, `இடைவெளி` | 3 | Handlers run in their A4 scope; the server loop is the VM's, called from compiled code |
| A6 | `--features llvm` on the Windows development machine, or a documented WSL2 set-up | — | Parity then runs locally, not only in CI |

After A5 the corpus is 111 of 111. **New builtins cost nothing here**: the desktop backend dispatches
builtins through the VM's own table, so the serial and GPIO builtins now being added reach `--llvm`
the day they reach the VM.

## Testing

1. **Parity (desktop)**: `scripts/run_parity.sh`, extended to Track A's statements; the last line must
   read that every program is accounted for.
2. **Conformance (boards)**: the `Static` lowering compiled for the host, the same program run under
   `etamil --vm`; printed output must match exactly unless the run reported a rounding or truncation.
3. **Golden IR**: `.qmz` → IR for AVR and Cortex-M, checked against stored files.
4. **Size budgets**: `arduino-cli compile` for `uno`, failing when flash or RAM grows past a limit.
5. **Simulation (optional)**: simavr or the Wokwi CLI in CI.
6. **Hardware**: the qIkkAppu procedure, stages 2 to 7.

## B1 and B2 as built

B1 and B2 are written, and they compile and link; their first run on hardware is still to come. It is
`etamil --artino [--board …] [--out …] [--upload PORT]` and `etamil --artino-gaps`.

27 Sep 2026:
- The compiler was built with `--features llvm` against LLVM 18 on the droplet.
- `examples/artino/minnu.qmz` (blink) and `examples/artino/pukY.qmz` (a smoke sensor with functions,
  booleans and two timers) were each compiled for three boards.
- Each was linked by arduino-cli against the real cores: `arduino:avr` 1.8.8 and `rp2040:rp2040`
  6.1.1.

| Program | Uno | Mega | Pico |
|---|---|---|---|
| `minnu.qmz` | 3,310 B flash / 263 B RAM | 4,048 B / 263 B | 55,248 B / 12,848 B |
| `pukY.qmz` | 4,020 B / 287 B | 4,758 B / 287 B | 55,784 B / 12,952 B |

The blink is smaller than B0's hand-written IR (3,872 B on the Uno), because B1 runs LLVM's
`default<Os>` pipeline before emitting.

| Area | B1 |
|---|---|
| Values | numbers (× 1000, `i64`), booleans (`i1`) |
| Statements | assignment, `நிலை`, `செயல்` with parameters and returns, `எனில்`/`இன்றேல்`, `சுற்று`, `திரும்பு`, `அச்சு`, `இடைவெளி`, `சுழற்சி` |
| Operators | `+` `-` inline; `*` `/` through the runtime (rounded and reported); comparisons; `மற்றும்` `அல்லது` short-circuiting; `இல்லை` |
| Printing | literal text, numbers and booleans joined by `&`, formatted as the VM formats them |
| Hardware | `முனை_வெளியீடு`, `முனை_உள்ளீடு`, `முனை_மேலிழு_உள்ளீடு`, `முனை_எழுது`, `முனை_படி`, `முனை_மாற்று`, `ஒப்புமை_படி`, `மில்லி_நொடி` |
| Checked | a name keeps one type, conditions are true or false, arguments match parameters, board functions get the right kinds |
| Output | an object for the board, its IR (`.ll`) for reading, and the sketch folder; `arduino-cli` runs when it is on `PATH` |

Rules a reader will meet:
- **An undeclared parameter is a number.** Write `ஈர்ம` before a parameter that takes true or false.
- **Falling off the end of a function that returns a number returns 0.** A board has no `இன்மை`.
- **Serial starts at 115200 baud before the program's first statement.** `தொடர்_திற` arrives with
  text in B3.
- **Literal text lives in flash on AVR** (`.progmem.data`), read back by the runtime. On an Uno that
  moved the blink from 263 to 207 bytes of RAM, even though B2 added report sites.

**B2's conformance suite.** `scripts/artino_conformance.sh` runs each program in
`etamil_compiler/tests/artino/` twice:
- on the VM;
- compiled with `--board host` — the same static lowering a board gets, for this machine — and
  linked with `artino_rt.cpp` over a stand-in Arduino API whose serial port is stdout.

The outputs must match line for line. `aRikkY.qmz` instead has an `.expected` file: it is where the
board must disagree with the VM (1 / 3, an overflow, 5 / 0) and say so, once per site.

27 Sep 2026, on the droplet: **5 matched, 0 failed**:

| Test | Covers |
|---|---|
| `eNkaL` | literals, precedence, exact `*` and `/`, percentages, negatives, printing |
| `muditivu` | comparisons, true/false, short-circuit order, if/else |
| `ceyalkaL` | recursion, return types, a function reading but not changing a global |
| `suRRu` | loops, nested loops, `திரும்பு` from inside a loop |
| `aRikkY` | rounding, overflow and division by zero, reported once each |

With B2, the examples still link on every board. On the Uno, the blink is 3,956 B flash / 207 B RAM
and the smoke sensor 5,198 B / 235 B.

## B3.1 as built: arrays and text

B3 is built in three slices, each finished and tested before the next:
- **B3.1:** arrays and text — below.
- **B3.2:** serial ports with results and `?`, for the qIkkAppu bus protocol.
- **B3.3:** shapes, arrays as parameters, and the qIkkAppu node written in eTamil.

| Area | B3.1 |
|---|---|
| Arrays | of numbers or booleans, up to 255 items. The length comes from a literal (`[180, 182]`) or from `அணி_நிரப்பு(value, count)` with the count written in the source |
| Array operations | indexing and assigning an element, both bounds-checked; `ஒவ்வொரு` over a snapshot, as the VM does; `நீளம்`; assigning copies; printed as the VM prints them, `[1, 2, 3]` |
| Text | values and variables of up to 48 bytes of UTF-8; `&` with text, numbers and booleans; `==` and `!=`; `சொல்லாக்கு`; `சொல்` parameters (copied in) and text returned from a `செயல்` |
| Reports | an index outside its array reads 0, or the write is skipped. Text that does not fit is cut at a character boundary, so no letter is split into bytes that are not text. Both are reported once per site |

`அணி_நிரப்பு` is also new in `nUlakam/atippatY/aNi.qmz`, so the same program runs on the VM.

Not in B3.1:
- **`நீளம்` of text.** The VM counts letters as grapheme clusters (`வரி` is 2), which a board cannot do cheaply, so it is refused rather than answered differently.
- **Indexing text, and `ஒவ்வொரு` over text** — B3.2.
- **Arrays of text, arrays as parameters or return values** — B3.3.

**Where buffers live.** Buffers an expression needs are allocated once per function, in its entry
block. So a function that builds many text lines holds 49 bytes of stack for each one for as long as
it runs. Sharing slots between temporaries that are never live at the same time is the next memory
improvement.

27 Sep 2026, on the droplet: conformance **8 matched, 0 failed**. The three new tests are `aNikaL`
(arrays), `urY` (text) and `aLavu` (sizes reached: a cut that stops before a half-written Tamil
letter, and an index outside its array). All four example programs link for the Uno and the Pico:

| Program | Uno flash / RAM | Pico flash / RAM |
|---|---|---|
| `minnu` | 3,948 B / 207 B | 55,960 B / 12,852 B |
| `pukY` | 5,190 B / 235 B | 56,688 B / 12,972 B |
| `aNikaL` | 7,094 B / 288 B | 56,920 B / 12,868 B |
| `urY` | 6,760 B / 441 B | 57,832 B / 13,092 B |

## B3.2 as built: results, serial ports, letters, rounding

| Area | B3.2 |
|---|---|
| Results | `சரி`, `தவறு`, `சரியா`, `தவறா`, `மதிப்பு`, `தவறு_மதிப்பு`, `இயல்பு`, and `?` in a `செயல்` that returns a result. Every result has one layout, `{ ok, payload }` (50 bytes): the value when `சரி`, the error text when `தவறு`. A board's `தவறு` always carries text. Printed as the VM prints them, `சரி(2.5)` / `தவறு(…)` |
| Unwrapping | `மதிப்பு` of a `தவறு` gives the zero value and is reported (the VM stops); `தவறு_மதிப்பு` of a `சரி` gives empty text, likewise |
| Serial ports | `தொடர்_திற`, `தொடர்_வரி_படி`, `தொடர்_எழுது`, `தொடர்_வரி_எழுது`, `தொடர்_மூடு` on ports 0–3: 0 is `Serial`, 1–3 the board's hardware ports where it has them (Mega 1–3, Pico 1–2, Uno none). A port the board lacks, or one that is closed, is a `தவறு` |
| Reading a line | `\r` is dropped; a line longer than 48 bytes is cut and reported. **No whole line yet is `சரி("")`**, on the VM too: `vaZporuL.qmz` turns the host's `சரி(இன்மை)` into empty text, because a board has no `இன்மை` and one answer everywhere is what lets one program run on all of them |
| Letters | `நீளம்` of text, indexing text, and `ஒவ்வொரு` over its letters, counted as the VM counts them for Latin and Tamil: a base character with the vowel signs, virama and combining marks after it, and `\r\n` as one. Any other script is counted one code point each, and reported |
| The string library | With letters, `nUlakam/atippatY/col.qmz` compiles for a board as written: `துண்டு`, `தேடு`, `தொடங்குகிறதா`, `முடிகிறதா`, `ஒழுங்கு` … |
| Rounding the author asked for | `தரை`, `மேல்`, `வட்டமிடு` (up to three places). A division or multiplication inside one goes straight to the rounded answer and is never reported. That is how a checksum like `ம - 97 * தரை(ம / 97)` runs exactly |
| Parsing | `எண்ணாக்கு` gives `சரி(number)` or a `தவறு`. More than three decimals rounds and is reported |
| `பலகை()` | the board's name, known when compiling: `uno`, `pico`, … |
| Parameters | an undeclared parameter now takes the type its callers pass, rather than defaulting to a number, because `col.qmz` declares none. A parameter keeps one type; callers that disagree are an error naming both |

Differences a reader will meet:
- **Error texts are the board's own, and short:** `துறை 3 இல்லை` ("port 3 absent"), `'abc' ஒரு எண் அல்ல`. The VM's messages are in Tamil and English, and most do not fit a board's 48 bytes. So a board's `தவறு` text is the Tamil half, or its own short Tamil sentence.
- **`மேல்(-0.5)` prints `0` on a board and `-0` on the VM,** whose decimal type keeps a negative zero. Only the VM can print `-0`.
- **A line on a port waits at most the `காலம்` given.** With 0 it does not wait at all, which is what firmware wants.
- **SoftwareSerial is not a port here.** The qIkkAppu node's Uno uses it on pins 10 and 11; it is an Arduino library, so it arrives with the `artino.toml` manifest (B4, and it did).

27 Sep 2026, on the droplet: conformance **12 matched, 0 failed**. The new tests:

| Test | Covers |
|---|---|
| `muDivu` | results and `?` |
| `vattam` | exact rounding |
| `ezuqqu` | letters, and `col.qmz` compiled for a board |
| `thodar` | serial port 0 fed from `thodar.input`: a line with `\r\n`, a reply, ports the host lacks, input running out |

Everything links for the three boards:

| Program | Uno flash / RAM | Mega flash / RAM | Pico flash / RAM |
|---|---|---|---|
| `pukY` | 5,190 B / 235 B | 6,708 B / 706 B | 59,028 B / 13,284 B |
| `muDivu` | 11,482 B / 530 B | 12,648 B / 994 B | 61,884 B / 13,444 B |
| `thodar` | 6,900 B / 609 B | 8,260 B / 1,089 B | 60,012 B / 13,952 B |
| `ezuqqu` (with `col.qmz`) | 13,480 B / 513 B | 14,618 B / 993 B | 64,020 B / 14,564 B |

**Removed in B3.3.** On the Mega, every program carried about 470 B of RAM for `Serial1`–`Serial3`,
whether it opened them or not. The sketch now carries `artino_config.h`, naming the ports the program
opens, and the runtime builds in only those: `thodar` on the Mega went from 1,089 B to 769 B.

## B3.3 as built: records, arrays of anything, the loop under test

| Area | B3.3 |
|---|---|
| Records | `வடிவம்` with typed or inferred fields, `பெயர்{…}` literals, `..base` copies, `அ.புலம்` reads and `அ.புலம் = …` writes, methods with `இது`, and associated functions called as `பெயர்.செயல்(…)`. A record is an LLVM struct of its fields in declared order, passed by address and copied like text, so a function given a record changes its own copy, as on the VM. Printed as the VM prints them: `பெயர்{புலம்: …}`, fields in sorted order. A record may hold another |
| Arrays | of numbers, booleans, texts or records; as parameters and as return values. A read outside the array gives an empty element and is reported |
| Ports | `artino_config.h` beside the sketch names the ports the program opens (above) |
| The loop, tested | A `.world` file beside a test runs the loop blocks on the host against a script: time moving on a fixed step, pins and analog readings set at given times, lines arriving on port 0 (`artino/host/host_main.cpp`). Its output is compared with a `.expected` file |
| Refused | a record literal without a `வடிவம்` (`{அ: 1}`): a board needs the fields' types before power-on |

27 Sep 2026, on the droplet: conformance **14 matched, 0 failed**. The new tests:

| Test | Covers |
|---|---|
| `vadivam` | records, methods, `..base`, records in records, arrays of text and records, arrays in and out of functions |
| `suzaRci` | three `இடைவெளி` blocks against `suzaRci.world`: timing, pins, readings, arriving lines |

| Program | Uno flash / RAM | Mega flash / RAM | Pico flash / RAM |
|---|---|---|---|
| `vadivam` | 9,874 B / 1,338 B | 10,208 B / 1,338 B | 59,136 B / 14,004 B |
| `thodar` | 6,900 B / 609 B | 7,708 B / 769 B | 57,848 B / 13,640 B |

**The qIkkAppu node in eTamil.** `qIkkAppu/firmware/kaNu/kaNu.qmz` is `kaNu.ino` rewritten in eTamil:
the point table as an array of records, the checksum as XOR a bit at a time with `தரை`, hex by text
indexing, the NTC by a table rather than `log`. Run as node 3 against `kaNu.world` (a poll, a bad
checksum, a poll for another node, heating, a flame, a reset), it answers every poll as `kaNu.ino`
would, and each reply's checksum checks.

It fits the Pico, 79,788 B / 16,384 B. **It does not fit an Uno or a Nano: 44,950 B of flash (139%) and
2,073 B of RAM (101%) before the stack.** What takes the space:

- about 14 KB of report sites. Each holds the source text of the operation it guards, and Tamil is
  three bytes a letter;
- about 18 KB of code: every number is a checked 64-bit operation, and on an 8-bit AVR each is a call
  with eight bytes an argument;
- 1,385 B of the program's own RAM, including 245 B for five constant texts and 168 B for the NTC
  table, all of which could live in flash;
- a 49-byte line buffer for each of the four ports, opened or not.

B3.4 made it fit; below.

## B3.4 as built: the node on an Uno

Measured on the linked Uno image, the B3.3 node was 44,452 B of flash: 21.5 KB of program code, 10 KB
of runtime, 9 KB of report-site text (not the 14 KB estimated above) and 3 KB of Arduino core. Its
RAM was worse than the 2,073 B arduino-cli printed, because that figure leaves out the stack:
`artino_loop` alone had a 2.4 KB frame. Every text temporary was its own 49 bytes for the whole
function, and each stack access past 63 bytes cost extra instructions to reach.

| Change | What it does |
|---|---|
| Temporaries live per statement | Each statement starts the lifetimes of the temporaries it makes and ends them after it. A `திரும்பு` ends every open one first. LLVM then gives temporaries of different statements one slot. |
| Buffers read in place | A text, array or record parameter the body never writes is read where the caller holds it, not copied in. `ஒவ்வொரு` walks a variable the body never writes where it is. `x = x & …` appends to x. `திரும்பு a & b` builds into the caller's buffer. |
| Letters are letters | Indexing text, and a local that only a `ஒவ்வொரு` over text ever sets, get a 16-byte buffer (`ARTINO_LETTER_BYTES`), not a text's 49. A longer letter is cut and reported. |
| Constants in flash | A `நிலை` text or array of literals is a constant global: in flash on AVR, copied out only where read. An element of a number array is read alone. |
| Sites in flash | A report site is constant: its number, its line, its text. Whether it has been reported is a bit of `artino_said`. On an Uno or Nano the text is left out: a report gives the line and number, and the sketch's `artino_sites.txt` lists every number with its full text. |
| `பலகை()` decided now | `பலகை() == "pico"` is a constant, so the other board's branch is not in the firmware. |
| Ports the board has | `artino_config.h` never builds a port the board lacks. A port chosen at run time counts as any, so this is what spares an Uno the buffers for ports 1–3. |
| Runtime | No 56-byte powers-of-ten table in RAM. The line buffers are sized to the highest port built. |

**Reports on an Uno or Nano** read `artino: வரி 16, #2 rounded`, or `artino: #3 rounded` when the
compiler does not know the line. Only assignments carry a line in the AST today. The words:

| Word | Meaning | What the VM does |
|---|---|---|
| `rounded` | needed more than three decimals | keeps 28 digits |
| `overflow` | too large for a board number; held at the limit | keeps it |
| `divided by 0` | gave 0 | stops with an error |
| `outside` | index outside the array; read as 0 or not written | stops with an error |
| `cut` | text longer than 48 bytes, or a letter longer than 15; cut to fit | keeps it all |
| `unwrap` | `மதிப்பு` of a `தவறு`; gave the zero value | stops with an error |
| `unwrap error` | `தவறு_மதிப்பு` of a `சரி`; gave empty text | stops with an error |
| `script` | letters of this script counted one code point each | follows Unicode's rules |

**`host-small`** is the host compiled as for an Uno: constants read as from flash, reports by line, the
runtime built with short reports. The conformance suite runs every test both ways. A test whose
output has reports needs a `.small.expected` for the second run (`aRikkY`, `aLavu`).

27 Sep 2026: conformance **28 matched, 0 failed**. That is 14 tests on the host, 12 of them again
as `host-small`, and the two with reports against their `.small.expected`. `kaNu.world` matches as
both.

| `kaNu.qmz` | Flash | RAM (static) | Stack, worst case |
|---|---|---|---|
| Uno | 30,074 B (93%) | 1,073 B (52%) | 804 B |
| Nano | 30,074 B (97%) | 1,073 B (52%) | 804 B |
| Mega | 41,310 B (16%) | 1,713 B (20%) | — |
| Pico | 79,804 B (3%) | 15,088 B (5%) | — |

The stack figure comes from the linked Uno image's call graph: each function's pushes and frame,
two bytes a call, along the deepest path from `main`, plus the deepest interrupt handler. That path is
the loop → `சரிபார்ப்பு` → `இடம்_இல்` → a letter → a report being printed. It leaves 171 B of the
Uno's 2 KB. On the Nano, 30,074 B is under its 30,720 B, which is less than the Uno's because of its
larger bootloader.

Every other program shrank too:

| Program | Uno before (B3.2) | Uno now |
|---|---|---|
| `pukY` | 5,190 B / 235 B | 5,202 B / 230 B |
| `thodar` | 6,900 B / 609 B | 6,020 B / 435 B |
| `muDivu` | 11,482 B / 530 B | 10,072 B / 485 B |
| `ezuqqu` | 13,480 B / 513 B | 9,888 B / 375 B |
| `vadivam` (B3.3) | 9,874 B / 1,338 B | 8,550 B / 1,294 B |

**Limits still in place.** A text is 49 bytes everywhere, and every number is 8. The node's 31-second
heat history is 248 B, against 62 B for `kaNu.ino`'s `int`s. Results and nested calls in one statement
are all alive together, and that statement sets the loop's frame (430 B, the reply to a poll).

## B4 as built: C++ libraries through artino.toml

The manifest above, its shims, and `[[port]]`. Three examples, each with its manifest beside it:

| Example | Library | Uno | Mega | Pico |
|---|---|---|---|---|
| `kaqavu` — a servo sweeping | Servo (Pico: its core's own) | 6,774 B / 258 B | 8,832 B / 369 B | 59,780 B / 13,052 B |
| `viLakku` — a NeoPixel ring chase | Adafruit NeoPixel | 7,286 B / 247 B | 9,338 B / 247 B | 59,300 B / 12,904 B |
| `veppam` — a DS18B20 thermometer, asked without waiting | OneWire, DallasTemperature | 8,968 B / 241 B | 9,878 B / 241 B | 59,252 B / 12,972 B |

All three link for the Nano too. None has run on hardware yet. The libraries were Servo 1.3.0,
Adafruit NeoPixel 1.15.5, OneWire 2.3.8 and DallasTemperature 4.0.6.

**The qIkkAppu node's bus is port 1 everywhere now**: Serial1 on a Pico, and SoftwareSerial on pins 10
and 11 of an Uno or Nano (`kaNu.artino.toml`). That frees USB for uploading and for the readings
it prints once a second, as `kaNu.ino` does. SoftwareSerial costs about 1.8 KB of flash, three
pin-change interrupts among it. Two changes paid for that:

- **Multiplying by a whole number written in the source is exact.** It goes to a small runtime
  function that watches only for overflow. When a program multiplies only that way, as the node
  does, the general multiply (1.7 KB on AVR) is not linked.
- **The node compares its address as text** (`"03"` with its own number padded) rather than
  parsing it. That leaves out `எண்ணாக்கு`'s 1.3 KB parser.

A report's line and number now print with a 16-bit printer, which took 70 B off the deepest stack
path.

| `kaNu.qmz` with SoftwareSerial | Flash | RAM (static) | Stack, worst case |
|---|---|---|---|
| Uno | 30,440 B (94%) | 1,255 B (61%) | 662 B |
| Nano | 30,440 B (99%) | 1,255 B (61%) | 662 B |
| Pico | 78,788 B (3%) | 15,088 B (5%) | — |

That leaves 131 B of the Nano's RAM on the deepest path, before SoftwareSerial's receive interrupt.
It fits, with little to spare. The next cut would be the node's own: its 31 eight-byte heat readings
are 248 B.

27 Sep 2026: conformance **28 matched, 0 failed**; `kaNu.world` matches on the host and as
`host-small`.

## B5 as built: the VM's board, the panel, the tools

**The VM's board.** `nUlakam/vaZporuL` works on the VM now, not only under artino. Every function
has a host builtin under Tamil, romanised and `_english` names (`வன்_முனை_வகை` / `vaZ_muZY_vakY` /
`_pinMode`, and so on), and `src/vm/board.rs` is the board:

| Board | Chosen by | Pins | Serial ports | Time |
|---|---|---|---|---|
| sim | `ETAMIL_BOARD=sim`, and always in the browser | what a test sets, `போலி_முனை` | `sim:<name>`, or a number as an Arduino numbers them, fed by `போலி_தொடர்_ஊட்டு` | moves only when told, or by `காத்திரு` |
| pi | a `pinctrl-bcm2711`/`bcm2835`/`bcm2712`/`rp1` GPIO chip | BCM numbers, GPIO v2 ioctls | device paths | the clock |
| host | anything else | none: an error naming the simulated board | device paths, Linux and macOS | the clock |

`vaZporuL_cOqaZY.qmz` passes 15 of 15 on the simulated board. On the droplet a real serial round trip
through a pseudo-terminal works: a poll read with its `\r\n` dropped, and a reply written. A
missing device, or an Arduino port number on a real machine, is a `தவறு`, not a crash. The GPIO path
matches the kernel's structures (a 592-byte line request, ioctl `0xC250B407`) but has not yet run on
a Pi.

**Tone and the watchdog** were in the plan for `vaZporuL` and are there now. `ஒலி_எழுப்பு` and
`ஒலி_நிறுத்து` drive a plain speaker; they are Arduino-only and simulated on the VM. `காவல்_தொடங்கு(ms)`
and `காவல்_புதுப்பி()` use AVR's `wdt` (the longest period not longer than asked, so it never restarts
late) or the RP2040's watchdog; on the VM they do nothing. Tone is built only into programs that make
one. Otherwise the core's Tone, and its timer interrupt, would cost every AVR sketch.

**The qIkkAppu panel in eTamil.** `qIkkAppu/firmware/palakY_mega/palakY_mega.qmz` is
`palakY_mega.ino` rewritten: nodes and points as arrays of records, one `இடைவெளி 0` block for each
step of the C++ loop, and the JSON to the Pi printed piece by piece, so no line is held to 48 bytes.
The bus frames are `firmware/pErunqu.qmz`, now shared with the node.

To test it off the board, the host stand-in has **nodes**. A world file's `node N BODY` puts a node on
serial port N, and it answers every line the program sends with `<NN,BODY*XX`, the checksum worked
out. `quiet N` silences it. `palakY_mega.world` runs Stage 7's tests 1 to 13 in 48 simulated seconds.
Everything the Mega decides behaves as `palakY_mega.ino` does:

- smoke alarm; silence;
- resound from the call point while silenced;
- latching; reset;
- heat; flame;
- the double knock's 10-second countdown and release; reset stopping it;
- a sensor fault and its restore; a node lost and back;
- a disabled point's alarm ignored;
- evacuate, and silencing it.

Every event comes in order, and the heartbeat's state, sounding, silenced and countdown values track
them second by second. `firmware/run_worlds.sh` runs both sketches' worlds, as the host and as
`host-small`.

| | Flash | RAM (static) | Stack, worst case |
|---|---|---|---|
| panel, Mega | 61,900 B (24%) | 2,825 B | 2,113 B, so 3.2 KB to spare |
| node, Uno | 30,594 B (94%) | 1,200 B | 679 B, 169 B to spare |
| node, Nano | 30,594 B (99%) | 1,200 B | 679 B |

The node's own changes that kept it on the Nano:

- a function, not a block, builds its readings line, so the line is on the stack only while it is
  printed;
- the checksum is compared as hex text, which avoids copying the hex table out of flash twice.

Three general ones also went in:

- `x = f(…)` in a function writes into `x` itself;
- `திரும்பு g(…)` writes into the caller's result;
- tone is built only when used.

**Size reports.** After an AVR build, `etamil --artino` reads the linked image back with avr-objdump.
It prints the variables and the deepest stack against the board's RAM, and warns when they can meet
(`src/artino/size.rs`):

```
  RAM: 1200 B of variables + 679 B of stack at most = 1879 of 2048 B (169 B to spare)
```

The stack is the deepest chain of calls from `main`: each function's pushes and frame, and two bytes
a call, plus the deepest interrupt handler. Calls through a pointer are not followed. arduino-cli's
"left for local variables" is the same RAM, without the question of whether the stack fits in it.

**VS Code.** *eTamil: Build for a board (artino)* and *Build and upload to a board* pick a board from a
fixed list and ask for the port, remembering both for the workspace. Neither reads a command from
settings. They need an `etamil` built with LLVM.

**Editor support** is regenerated: 82 builtins and 722 library functions, the board files among them.

Still to do before hardware counts as done: every one of the above on real boards. The Pi's GPIO path,
the node on the Uno and Nano with SoftwareSerial, the panel on the Mega with three nodes, and Stage 7
run by hand.

## Plan

Track B starts with B0 because it decides whether the LLVM route is viable at all. Track A runs
alongside it.

| Step | Builds | Done when | Weeks (est.) |
|---|---|---|---|
| **B0** | Toolchain spike: hand-written IR for AVR and Cortex-M → object → precompiled library → `arduino-cli` link → upload; a C call in each direction | Blink on an Uno and a Pico, from LLVM-emitted code | 1–2 |
| B1 | `Static` lowering: numbers, booleans, `நிலை`, functions, control flow, `இடைவெளி`, `சுழற்சி`, target machines, `--artino-gaps` | Blink and analog read from `.qmz` on Uno and Pico | 4–5 |
| B2 | artino-rt: fixed point with reports, printing, timers; vaZporuL shim | Conformance suite passes for numbers and printing | 2 |
| B3 | Text, fixed arrays, `அணி_நிரப்பு`, shapes, results, serial read line, DWARF lines | **qIkkAppu node in eTamil** passes procedure stages 3–5 | 3–4 |
| B4 | `artino.toml` and shims | Servo, NeoPixel, DS18B20 examples work | 2 |
| B5 | Simulated vaZporuL on the VM, size reports, VS Code build/upload commands, editor grammars, docs | **qIkkAppu Mega panel in eTamil** passes all acceptance tests | 3 |
| A1–A6 | As in Track A | Corpus 111 of 111 under `--llvm`; parity runs on the development machine | 8–11 |

One engineer: about 15–18 weeks for Track B, and Track A's 8–11 weeks alongside or after it. The
C++-text route was estimated at 12–15 weeks for the boards alone; the difference buys a single back
end and Track A.

## Risks

| Risk | Answer |
|---|---|
| LLVM's AVR output does not link or behave with the avr-gcc-built Arduino core | B0, on hardware, before B1. If it fails, the C++-text route is the fallback, and the gap walk, number rules and vaZporuL carry over unchanged |
| The prebuilt LLVM 18 in use lacks the AVR or ARM target | `llc --version` lists registered targets. Otherwise build LLVM with the targets enabled; CI caches it |
| Numbers differ from the VM | One representation, the rules above, conformance test 2 on every change |
| 2 KB of RAM | No allocation after setup, literals in flash, a RAM figure in every build, a warning on recursion |
| Two lowerings drift apart | One `Compiler`, one gap walk; a construct one lowering cannot build is refused there, never approximated |
| Development happens on Windows, where LLVM does not build today | A6, and until then WSL2 or CI |

## Still open

- Final Tamil names for the `vaZporuL` functions, agreed with the VM serial builtins.
- Whether text capacity should also be settable per variable, not only per program.
