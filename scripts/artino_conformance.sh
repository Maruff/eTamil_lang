#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
#
# artino's conformance suite: does a program compiled for a board print what
# the VM prints?
#
#   scripts/artino_conformance.sh
#   ETAMIL_BIN=etamil_compiler/target/debug/etamil scripts/artino_conformance.sh
#
# Each etamil_compiler/tests/artino/*.qmz is run twice:
#   - on the VM, `etamil --vm`;
#   - compiled with `etamil --artino --board host` — the same static lowering a
#     board gets, targeting this machine — and linked with artino_rt.cpp over a
#     stand-in Arduino API (etamil_compiler/artino/host), whose Serial is stdout.
# The outputs must be identical. A test with a `.expected` file beside it is
# one where the board must disagree with the VM and say so — a rounding, an
# overflow, a division by zero — or one the VM cannot run yet (serial ports),
# and its output is compared with that file. A `.input` file is what arrives
# on the board's serial port 0. A `.world` file runs the loop blocks as well,
# against a script of time, pins and arriving lines (artino/host/host_main.cpp).
#
# Each test then runs again compiled with `--board host-small`, which does what
# only a small board does — constants copied out of flash, reports by line —
# on this machine. A report reads differently there, so a test whose output
# has one runs twice only if it has a `.small.expected` too.
#
# Needs an etamil built with `--features llvm` and a C++ compiler; Linux or
# macOS, like run_parity.sh.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${ETAMIL_BIN:-$ROOT/etamil_compiler/target/release/etamil}"
[[ -x "$BIN" ]] || BIN="$ROOT/etamil_compiler/target/debug/etamil"
CXX="${CXX:-c++}"
RUNTIME="$ROOT/etamil_compiler/artino"
TESTS="$ROOT/etamil_compiler/tests/artino"
export ETAMIL_PATH="$ROOT"

if [[ ! -x "$BIN" ]]; then
    echo "error: no etamil at $BIN — build it with --features llvm first"
    exit 1
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# The runtime and the host stand-in are the same for every test: build them once.
if ! "$CXX" -std=c++17 -O1 -I "$RUNTIME/host" -I "$RUNTIME" -c "$RUNTIME/artino_rt.cpp" -o "$work/artino_rt.o" \
   || ! "$CXX" -std=c++17 -O1 -DARTINO_LINE_REPORTS=1 -I "$RUNTIME/host" -I "$RUNTIME" -c "$RUNTIME/artino_rt.cpp" -o "$work/artino_rt_small.o" \
   || ! "$CXX" -std=c++17 -O1 -I "$RUNTIME/host" -I "$RUNTIME" -c "$RUNTIME/host/host_main.cpp" -o "$work/host_main.o"; then
    echo "error: the runtime does not compile for the host"
    exit 1
fi

matched=0
failed=0
for test in "$TESTS"/*.qmz; do
    name="$(basename "$test" .qmz)"
    out="$work/$name"
    mkdir -p "$out"

    build="$("$BIN" --artino --board host --out "$out" "$test" 2>&1)"
    object="$(ls "$out"/*.host.o 2>/dev/null | head -n 1)"
    if [[ -z "$object" ]]; then
        echo "  FAIL  $name — did not compile:"
        grep -E '✗|    - ' <<<"$build" | sed 's/^/        /'
        failed=$((failed + 1))
        continue
    fi
    if ! "$CXX" "$work/host_main.o" "$work/artino_rt.o" "$object" -o "$out/prog" 2>"$out/link.txt"; then
        echo "  FAIL  $name — did not link:"
        head -n 5 "$out/link.txt" | sed 's/^/        /'
        failed=$((failed + 1))
        continue
    fi
    # What arrives on the board's serial port 0, when the test gives any.
    input="/dev/null"
    [[ -f "$TESTS/$name.input" ]] && input="$TESTS/$name.input"
    # A world file runs the loop too (host_main.cpp); its output needs a .expected.
    if [[ -f "$TESTS/$name.world" ]]; then
        board="$(ARTINO_HOST_WORLD="$TESTS/$name.world" "$out/prog" < "$input")"
    else
        board="$("$out/prog" < "$input")"
    fi

    if [[ -f "$TESTS/$name.expected" ]]; then
        wanted="$(cat "$TESTS/$name.expected")"
        against="$name.expected"
    else
        wanted="$("$BIN" --vm "$test" 2>&1 | sed -n '/=== Execution Output ===/,$p' \
                  | sed '1d;/^✓ Execution completed successfully$/d' | sed '/^$/d')"
        against="the VM"
    fi

    if [[ "$board" == "$wanted" ]]; then
        echo "  ok    $name  (against $against)"
        matched=$((matched + 1))
    else
        echo "  FAIL  $name  (against $against):"
        diff <(printf '%s\n' "$wanted") <(printf '%s\n' "$board") | head -n 30 | sed 's/^/        /'
        failed=$((failed + 1))
    fi

    # The same test built as for a small board. Its reports name a line and a
    # number rather than the operation, so a test with any needs .small.expected.
    if [[ -f "$TESTS/$name.small.expected" ]]; then
        wanted="$(cat "$TESTS/$name.small.expected")"
    elif [[ "$wanted" == *"artino: "* ]]; then
        continue
    fi
    small="$out/small"
    mkdir -p "$small"
    "$BIN" --artino --board host-small --out "$small" "$test" >/dev/null 2>&1
    object="$(ls "$small"/*.host-small.o 2>/dev/null | head -n 1)"
    if [[ -z "$object" ]] || ! "$CXX" "$work/host_main.o" "$work/artino_rt_small.o" "$object" -o "$small/prog" 2>/dev/null; then
        echo "  FAIL  $name as host-small — did not build"
        failed=$((failed + 1))
        continue
    fi
    if [[ -f "$TESTS/$name.world" ]]; then
        board="$(ARTINO_HOST_WORLD="$TESTS/$name.world" "$small/prog" < "$input")"
    else
        board="$("$small/prog" < "$input")"
    fi
    if [[ "$board" == "$wanted" ]]; then
        echo "  ok    $name as host-small"
        matched=$((matched + 1))
    else
        echo "  FAIL  $name as host-small:"
        diff <(printf '%s\n' "$wanted") <(printf '%s\n' "$board") | head -n 30 | sed 's/^/        /'
        failed=$((failed + 1))
    fi
done

echo
echo "artino conformance: $matched matched, $failed failed"
[[ $failed -eq 0 ]]
