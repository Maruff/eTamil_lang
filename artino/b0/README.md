# artino B0 — the toolchain spike

B0 answers one question before any backend work starts (see [docs/artino.md](../../docs/artino.md)):
**does code that LLVM emits for AVR and Cortex-M link against the Arduino cores, which gcc builds,
and run?**

`prog.ll` is hand-written IR shaped like the backend's future output. It:
- exports `artino_setup` and `artino_loop` to C;
- calls a C shim for pins, time and serial;
- runs a `millis()` timer — what `இடைவெளி 0.5 { … }` will become;
- does 64-bit fixed-point arithmetic (value × 1000). The chips have no instructions for that, so it
  lowers to libgcc calls that must link.

| File | Role |
|---|---|
| `prog.ll` | The "compiled eTamil program" |
| `sketch/artino_b0/artino_b0.ino` | Three lines: `setup()` and `loop()` call the program |
| `sketch/artino_b0/artino_shim.cpp` | `extern "C"` boundary to the Arduino core (MIT). Generated from `vaZporuL` in B1 |
| `build.py` | `llc` → object → `llvm-ar` → precompiled Arduino library → `arduino-cli compile` / upload |

## Tools

No admin rights are needed for any of them.

```bash
rustup component add llvm-tools          # llc and llvm-ar, from Rust's LLVM (has AVR and ARM)
# arduino-cli: unzip the release into ~/tools/arduino-cli/
arduino-cli core install arduino:avr
arduino-cli core install rp2040:rp2040   # needs the arduino-pico board manager URL
```

On Windows with the Microsoft Store Python, keep the Arduino data outside `AppData`:
`~/tools/arduino15/arduino-cli.yaml` with `directories.data` pointing there. The Store Python gives
the programs it starts a private copy of `AppData\Local`, so they would not see the installed boards.
`build.py` picks up that config file automatically.

## Run

```bash
python build.py                     # uno, nano, mega, pico
python build.py uno --upload COM5   # build and upload
```

Then open a serial monitor at 115200. The LED toggles every 500 ms, and each toggle prints:

```
tick 1  count 1.5  18% 0.27  /7  0.214
tick 2  count 3  18% 0.54  /7  0.428
tick 3  count 4.5  18% 0.81  /7  0.642
```

The `/7` column truncates. B0 tests that 64-bit division links and runs, not the rounding rules,
which arrive in B2.

## Results

27 Sep 2026, Windows 11:
- LLVM 22.1.6 (Rust 1.97.1's `llc`);
- arduino-cli 1.5.1, `arduino:avr` 1.8.8 (avr-gcc 7.3);
- `rp2040:rp2040` 6.1.1 (arm-none-eabi gcc 16.1).

| Board | Links | Flash / RAM used | Runs on hardware |
|---|---|---|---|
| Uno | yes | 3,872 B (12%) / 244 B (11%) | not yet run |
| Nano | yes | 3,872 B (12%) / 244 B (11%) | not yet run |
| Mega | yes | 4,602 B (1%) / 244 B (2%) | not yet run |
| Pico | yes | 55,224 B (2%) / 12,856 B (4%) | not yet run |

What the link proved:
- **Entry points and calls cross the C boundary both ways.** `setup()`/`loop()` in gcc-built code call
  `artino_setup`/`artino_loop` in LLVM-built code, which calls back into the gcc-built shim.
- **64-bit arithmetic links to each core's own runtime.** On AVR, LLVM's calls to `__muldi3` and
  `__divdi3` resolve to avr-gcc's libgcc. On the Pico, the SDK's linker `--wrap` redirected LLVM's
  `__aeabi_lmul` and `__aeabi_ldivmod` calls to its hardware-divider routines (`divmod_s64s64`).
  Compiled eTamil gets the chip's fast path with nothing done on artino's side.
- **Arduino's precompiled-library mechanism works for both cores.** The library folder is named from
  `build.mcu`: `atmega328p`, `atmega2560`, `cortex-m0plus`.
- The one linker warning on the Pico (`crtn.o: missing .note.GNU-stack`) is from the Pico toolchain's
  own start-up file, not artino's object.

Still to prove: the serial output above on a real Uno and a real Pico.
