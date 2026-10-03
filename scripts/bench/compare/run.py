#!/usr/bin/env python3
"""Time the same slab-tax loop in every language installed on this machine.

    python scripts/bench/compare/run.py
    python scripts/bench/compare/run.py --repeats 7 --target-ms 600

Four things this does that a `time`-in-a-loop does not, each because the first
version of it produced a wrong number without saying so.

**Per-iteration cost comes from two workload sizes, not one.** Creating a
process on this class of Windows machine costs 110-140 ms before a single
instruction of the program runs — anti-malware scans every image — and it is
noisy enough that "total minus an empty program" produced *negative* compute
times for C and Rust. Timing at `N` and at `2N` and taking the difference
cancels every constant: process creation, runtime startup, parsing, printing.
What is left is the loop.

    ns/iteration = (T(2N) - T(N)) * 1e6 / N

**It sizes the workload per language.** A loop that takes eTamil a second takes
C microseconds. Each implementation is calibrated so its own compute time
reaches `--target-ms`; without that the fast end of the table reads 0 ms, which
is a missing measurement pretending to be a result.

**It checks the answer.** Every implementation must print the same digits as an
exact rational computed here before its timing is reported. A benchmark that
computes the wrong thing quickly is not a fast benchmark.

**It reports whether the loop survived the optimiser.** This computation is a
triangular sum, and at `/O2` a C or Rust compiler is entitled to replace the
whole loop with a closed form. When that happens T(2N) - T(N) collapses towards
zero, and the row is marked rather than quietly reported as a hundred-fold win.

Timings are the *minimum* of the repeats. The minimum is the run least polluted
by other work on the machine; a mean on a desktop measures the desktop.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass, field
from fractions import Fraction
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent.parent
BIN = HERE / "bin"

EXE = ".exe" if sys.platform == "win32" else ""
ETAMIL = REPO / "etamil_compiler" / "target" / "release" / f"etamil{EXE}"
DOTNET_DLL = HERE / "csharp" / "bin" / "Release" / "net9.0" / "bench.dll"

# Below this, one iteration is faster than the multiply-divide-add chain could
# possibly be on current hardware, so the loop was reduced rather than run.
REDUCED_NS = 0.6


def pin_to_one_core() -> str:
    """Pin this process, and so every child, to a single CPU at high priority.

    Not a nicety. This machine is an Intel hybrid part — 14 logical CPUs across
    performance, efficient and low-power-efficient cores — and Windows moves
    short-lived processes between them freely. Unpinned, the same measurement
    varied by up to 1.85x *within* one run and by 3x between runs, because some
    repeats landed on a P-core and some on an E-core. That is not noise you can
    average away; it is two different processors.

    Children inherit the affinity mask and the priority class on Windows, so
    setting it once here covers every program launched below.
    """
    if sys.platform != "win32":
        try:
            import os

            os.sched_setaffinity(0, {0})  # type: ignore[attr-defined]
            return "pinned to cpu0"
        except (AttributeError, OSError):
            return "not pinned (unsupported)"

    try:
        import ctypes
        from ctypes import wintypes

        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)

        # The signatures matter. Without them ctypes assumes a C int return,
        # which truncates the process pseudo-handle on 64-bit and makes the
        # call fail with ERROR_INVALID_HANDLE — which is exactly what happened
        # the first time this ran, reported as "SetProcessAffinityMask failed".
        kernel32.GetCurrentProcess.restype = wintypes.HANDLE
        kernel32.GetCurrentProcess.argtypes = []
        kernel32.SetProcessAffinityMask.restype = wintypes.BOOL
        kernel32.SetProcessAffinityMask.argtypes = [wintypes.HANDLE, ctypes.c_size_t]
        kernel32.SetPriorityClass.restype = wintypes.BOOL
        kernel32.SetPriorityClass.argtypes = [wintypes.HANDLE, wintypes.DWORD]

        handle = kernel32.GetCurrentProcess()
        # CPU 0. On Intel hybrid parts core 0 is a performance core.
        if not kernel32.SetProcessAffinityMask(handle, 1):
            return (
                "not pinned (SetProcessAffinityMask: "
                f"error {ctypes.get_last_error()})"
            )
        HIGH_PRIORITY_CLASS = 0x00000080
        priority = "high priority" if kernel32.SetPriorityClass(
            handle, HIGH_PRIORITY_CLASS
        ) else "normal priority"
        return f"pinned to cpu0, {priority}"
    except Exception as error:  # pragma: no cover - platform dependent
        return f"not pinned ({error})"


@dataclass
class Impl:
    name: str
    group: str  # "exact" or "float"
    runtime: str
    command: object  # (n) -> argv
    empty: object  # () -> argv
    stdin_n: bool = False  # eTamil has no argv; N arrives on stdin
    note: str = ""

    available: bool = True
    empty_ms: float = 0.0
    startup_over_native: float = 0.0
    fixed_total_ms: float = 0.0
    n: int = 0
    ns_per_iter: float = 0.0
    ns_low: float = 0.0
    ns_high: float = 0.0
    reduced: bool = False
    rel_error: float = 0.0
    samples: list = field(default_factory=list)


def expected_paisa(n: int) -> int:
    """The exact answer in paisa.

    0.05 x N(N-1)/2. N(N-1) is always even, so this is always a whole number of
    paisa and never needs more than two decimal places — which is why the
    scaled-integer implementations can agree with the decimal ones exactly.
    """
    return 5 * (n * (n - 1) // 2)


def expected(n: int) -> str:
    """The exact answer as the programs print it.

    Formatted from the integer, never through a float. Doing this with
    `f"{float(value):.2f}"` was wrong above about 10^14 — it rejected Rust's
    correct `155624338473637.55` because the check itself had rounded. The
    benchmark's own oracle was the least precise thing in the run.
    """
    paisa = expected_paisa(n)
    whole, frac = divmod(paisa, 100)
    if frac == 0:
        return str(whole)
    return f"{whole}.{frac:02d}".rstrip("0").rstrip(".")


def float_error(n: int, printed: str) -> float | None:
    """Relative error of a printed float answer against the exact one.

    Binary floats cannot represent 0.05, so at large N these implementations
    are measurably wrong — which is the whole reason eTamil uses decimals. They
    are held to a tolerance rather than to equality, and the error is reported
    rather than hidden.
    """
    try:
        got = Fraction(printed)
    except (ValueError, ZeroDivisionError):
        return None
    exact = Fraction(expected_paisa(n), 100)
    if exact == 0:
        return 0.0
    return abs(float((got - exact) / exact))


def normalise(text: str) -> str:
    """Compare answers, not formatting.

    The programs disagree about trailing zeros and about printing `.00` at all:
    C's `%.2f`, Python's `Decimal` and eTamil's `normalize()` each choose
    differently. None of that is the arithmetic under test.
    """
    if not text.strip():
        return ""
    value = text.strip().splitlines()[-1].strip()
    if "." in value:
        value = value.rstrip("0").rstrip(".")
    return value


def run_once(impl: Impl, n: int) -> tuple[float, str]:
    command = [str(part) for part in impl.command(n)]
    stdin = f"{n}\n" if impl.stdin_n else ""

    start = time.perf_counter()
    result = subprocess.run(
        command,
        input=stdin,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        cwd=HERE,
    )
    elapsed = (time.perf_counter() - start) * 1000.0

    if result.returncode != 0:
        raise RuntimeError(
            f"exited {result.returncode}: "
            f"{(result.stderr or result.stdout).strip()[:300]}"
        )

    # eTamil wraps its output in a banner; take the last line that is not one.
    answer = ""
    for line in reversed([l for l in result.stdout.splitlines() if l.strip()]):
        if not line.startswith(("✓", "✗", "===", "⚠")):
            answer = line
            break
    return elapsed, answer


def best_of(impl: Impl, n: int, repeats: int) -> tuple[float, float, str]:
    """Minimum and median of `repeats` runs, plus what the program printed.

    Both statistics are kept because they disagree, sometimes by 2x. The
    minimum is the run least disturbed by other work; the median says how
    disturbed a typical run was. Reporting only the minimum on a machine this
    noisy would state a precision the measurement does not have.
    """
    run_once(impl, n)  # warm the page cache and any JIT
    times = []
    answer = ""
    for _ in range(repeats):
        elapsed, answer = run_once(impl, n)
        impl.samples.append({"n": n, "ms": round(elapsed, 2)})
        times.append(elapsed)
    return min(times), statistics.median(times), answer


def interleaved(
    impl: Impl, n: int, repeats: int
) -> tuple[float, float, float, str, str]:
    """Time N and 2N alternately and difference each *pair*.

    Two changes from the obvious approach, both because the obvious approach
    produced numbers that contradicted themselves.

    Interleaving, rather than all of N then all of 2N: machine drift over the
    course of a measurement otherwise masquerades as growth. Measured back to
    back this loop looked superlinear in eTamil; interleaved, it was linear all
    along.

    Differencing within each pair, rather than subtracting one aggregate from
    another: `min(high) - min(low)` picks its two ends from different runs, so
    when the noise is comparable to the signal the result can come out *lower*
    than the same figure computed from medians — which is what happened, and is
    impossible for a real quantity. A paired difference cancels whatever the
    machine was doing during that pair.

    Returns (median difference, min difference, max difference, outputs).
    """
    run_once(impl, n)
    run_once(impl, 2 * n)

    diffs = []
    low_out = high_out = ""
    for _ in range(repeats):
        low_ms, low_out = run_once(impl, n)
        impl.samples.append({"n": n, "ms": round(low_ms, 2)})

        high_ms, high_out = run_once(impl, 2 * n)
        impl.samples.append({"n": 2 * n, "ms": round(high_ms, 2)})

        diffs.append(high_ms - low_ms)

    diffs.sort()
    return statistics.median(diffs), diffs[0], diffs[-1], low_out, high_out


def calibrate(impl: Impl, target_ms: float, ceiling: int) -> int:
    """Smallest N whose own compute time reaches `target_ms`.

    Uses the two-point difference to judge compute time, for the same reason
    the final measurement does: on this machine a single timing is mostly
    process creation.
    """
    n = 20_000
    while n < ceiling:
        low, _ = run_once(impl, n)
        high, _ = run_once(impl, 2 * n)
        # One pair is enough to size the workload; precision comes later.
        compute = max(high - low, 0.0)
        if compute >= target_ms:
            return n
        if compute > 3.0:
            projected = int(n * (target_ms / compute))
            n = min(ceiling, max(n * 2, projected))
        else:
            n = min(ceiling, n * 8)
    return ceiling


def find_php() -> str | None:
    """PHP, from PATH or from where winget's zip install puts it.

    The winget package edits PATH but an already-running shell does not see it,
    so looking only at PATH finds nothing until the machine is logged out and
    back in. The Packages directory is checked second for that reason.
    """
    found = shutil.which("php")
    if found:
        return found
    packages = Path(os.environ.get("LOCALAPPDATA", "")) / "Microsoft" / "WinGet" / "Packages"
    if packages.is_dir():
        for candidate in sorted(packages.glob("PHP.PHP*/php.exe"), reverse=True):
            return str(candidate)
    return None


def build_impls() -> list[Impl]:
    python = sys.executable
    node = shutil.which("node")
    dotnet = shutil.which("dotnet")
    java = shutil.which("java")
    php = find_php()

    def b(stem: str) -> Path:
        return BIN / f"{stem}{EXE}"

    impls = [
        Impl(
            "eTamil — Decimal (VM)", "exact", "etamil, bytecode VM",
            lambda n: [ETAMIL, "--vm", "tax.qmz"],
            lambda: [ETAMIL, "--vm", "empty.qmz"],
            stdin_n=True, note="rust_decimal, interpreted",
        ),
        Impl(
            "Rust — rust_decimal", "exact", "rustc, opt-level 3 + LTO",
            lambda n: [b("tax_decimal"), n], lambda: [b("empty"), 1],
            note="the same crate eTamil's VM calls",
        ),
        Impl(
            "C# — decimal", "exact", "dotnet 9, Release",
            lambda n: [dotnet, DOTNET_DLL, "decimal", n],
            lambda: [dotnet, DOTNET_DLL, "empty"],
            note="native 128-bit decimal",
        ),
        Impl(
            "C — int64 paisa", "exact", "MSVC 19.44, /O2",
            lambda n: [b("tax_int_c"), n], lambda: [b("empty_c")],
            note="hand-scaled integers",
        ),
        Impl(
            "C++ — int64 paisa", "exact", "MSVC 19.44, /O2 /EHsc",
            lambda n: [b("tax_int_cpp"), n], lambda: [b("empty_cpp")],
            note="same loop as the C row, compiled as C++",
        ),
        Impl(
            "Python — Decimal", "exact", "CPython 3.14",
            lambda n: [python, "tax_decimal.py", n], lambda: [python, "empty.py"],
            note="stdlib decimal",
        ),
        Impl(
            "Python — int paisa", "exact", "CPython 3.14",
            lambda n: [python, "tax_int.py", n], lambda: [python, "empty.py"],
            note="hand-scaled integers",
        ),
        Impl(
            "Node — BigInt", "exact", "node 24",
            lambda n: [node, "tax_bigint.js", n], lambda: [node, "empty.js"],
            note="hand-scaled BigInt",
        ),
        Impl(
            "PHP — int paisa", "exact", "PHP 8.4 CLI",
            lambda n: [php, "tax_int.php", n], lambda: [php, "empty.php"],
            note="hand-scaled integers, PHP_INT_SIZE 8",
        ),
        Impl(
            "PHP — bcmath", "exact", "PHP 8.4 CLI",
            lambda n: [php, "tax_bcmath.php", n], lambda: [php, "empty.php"],
            note="arbitrary precision over decimal strings",
        ),
        Impl(
            "Java — BigDecimal", "exact", "OpenJDK 21, C2",
            lambda n: [java, "-cp", str(HERE / "java"), "Tax", "bigdecimal", n],
            lambda: [java, "-cp", str(HERE / "java"), "Tax", "empty"],
            note="stdlib arbitrary-precision decimal",
        ),
        Impl(
            "Java — long paisa", "exact", "OpenJDK 21, C2",
            lambda n: [java, "-cp", str(HERE / "java"), "Tax", "long", n],
            lambda: [java, "-cp", str(HERE / "java"), "Tax", "empty"],
            note="hand-scaled integers",
        ),
        Impl(
            "C — double", "float", "MSVC 19.44, /O2",
            lambda n: [b("tax_double_c"), n], lambda: [b("empty_c")],
        ),
        Impl(
            "C++ — double", "float", "MSVC 19.44, /O2 /EHsc",
            lambda n: [b("tax_double_cpp"), n], lambda: [b("empty_cpp")],
        ),
        Impl(
            "Rust — f64", "float", "rustc, opt-level 3 + LTO",
            lambda n: [b("tax_f64"), n], lambda: [b("empty"), 1],
        ),
        Impl(
            "C# — double", "float", "dotnet 9, Release",
            lambda n: [dotnet, DOTNET_DLL, "double", n],
            lambda: [dotnet, DOTNET_DLL, "empty"],
        ),
        Impl(
            "Node — number", "float", "node 24",
            lambda n: [node, "tax_float.js", n], lambda: [node, "empty.js"],
        ),
        Impl(
            "Python — float", "float", "CPython 3.14",
            lambda n: [python, "tax_float.py", n], lambda: [python, "empty.py"],
        ),
        Impl(
            "PHP — float", "float", "PHP 8.4 CLI",
            lambda n: [php, "tax_float.php", n], lambda: [php, "empty.php"],
        ),
        Impl(
            "Java — double", "float", "OpenJDK 21, C2",
            lambda n: [java, "-cp", str(HERE / "java"), "Tax", "double", n],
            lambda: [java, "-cp", str(HERE / "java"), "Tax", "empty"],
        ),
    ]

    for impl in impls:
        missing = None
        if impl.name.startswith("eTamil") and not ETAMIL.exists():
            missing = "cd etamil_compiler && cargo build --release"
        elif impl.name.startswith("Node") and not node:
            missing = "node not on PATH"
        elif impl.name.startswith("C#") and (not dotnet or not DOTNET_DLL.exists()):
            missing = "dotnet build csharp/bench.csproj -c Release"
        elif impl.name.startswith("C —") and not b(
            "tax_int_c" if "int64" in impl.name else "tax_double_c"
        ).exists():
            missing = "C not built — see README"
        elif impl.name.startswith("C++") and not b(
            "tax_int_cpp" if "int64" in impl.name else "tax_double_cpp"
        ).exists():
            missing = "C++ not built — see README"
        elif impl.name.startswith("PHP") and not php:
            missing = "php not on PATH"
        elif impl.name.startswith("Java") and (
            not java or not (HERE / "java" / "Tax.class").exists()
        ):
            missing = "java not on PATH" if not java else "javac java/Tax.java"
        elif impl.name.startswith("Rust") and not b("tax_decimal").exists():
            missing = "cd rust && cargo build --release"
        if missing:
            impl.available, impl.note = False, missing

    return impls


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repeats", type=int, default=5)
    parser.add_argument("--target-ms", type=float, default=400.0)
    parser.add_argument("--fixed-n", type=int, default=100_000)
    parser.add_argument("--ceiling", type=int, default=400_000_000)
    parser.add_argument("--json", type=Path)
    args = parser.parse_args()

    # The report is written in en dashes, × and ⚠︎, and a Windows console is
    # cp1252 by default, which cannot encode any of them. Printing the first
    # per-iteration row then raised UnicodeEncodeError and killed the run after
    # all the measuring was done but before the JSON was written — the whole
    # benchmark lost to the last step. UTF-8 with replacement keeps the numbers
    # even where the console cannot draw the characters.
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")

    pinning = pin_to_one_core()

    impls = build_impls()
    unavailable = [i for i in impls if not i.available]
    impls = [i for i in impls if i.available]

    print("# eTamil cross-language comparison\n")
    print(f"- cpu: {platform.processor() or platform.machine()}")
    print(f"- os: {platform.system()} {platform.release()}")
    print(f"- scheduling: {pinning}")
    print(f"- repeats: {args.repeats}, minimum reported\n")

    # --- The floor -------------------------------------------------------
    # Everything is measured through subprocess creation, so establish what
    # that costs before attributing any of it to a runtime.
    native = next((i for i in impls if i.name == "C — double"), None)
    floor = 0.0
    if native:
        command = [str(part) for part in native.empty()]
        subprocess.run(command, capture_output=True, cwd=HERE)
        times = []
        for _ in range(max(args.repeats, 7)):
            start = time.perf_counter()
            subprocess.run(command, capture_output=True, cwd=HERE)
            times.append((time.perf_counter() - start) * 1000.0)
        floor = min(times)
        print(
            f"Creating a process and running a do-nothing **native** binary "
            f"costs **{floor:.0f} ms** here. That is the floor under every "
            f"number in the next table, and the reason per-iteration cost is "
            f"measured by difference instead.\n"
        )

    # --- Startup ---------------------------------------------------------
    print("## Startup\n")
    print("Time to start the runtime and print one line.\n")
    for impl in impls:
        try:
            command = [str(p) for p in impl.empty()]
            stdin = "1\n" if impl.stdin_n else ""
            subprocess.run(command, input=stdin, capture_output=True, text=True, cwd=HERE)
            times = []
            for _ in range(args.repeats):
                start = time.perf_counter()
                subprocess.run(
                    command, input=stdin, capture_output=True, text=True, cwd=HERE
                )
                times.append((time.perf_counter() - start) * 1000.0)
            impl.empty_ms = min(times)
            impl.startup_over_native = max(0.0, impl.empty_ms - floor)
        except Exception as error:
            impl.available = False
            impl.note = f"failed: {error}"

    print("| Runtime | Empty program | Over the native floor |")
    print("|---|---|---|")
    seen = set()
    for impl in sorted((i for i in impls if i.available), key=lambda i: i.empty_ms):
        if impl.runtime in seen:
            continue
        seen.add(impl.runtime)
        print(
            f"| {impl.runtime} | {impl.empty_ms:.0f} ms "
            f"| +{impl.startup_over_native:.0f} ms |"
        )

    # --- Identical workload ----------------------------------------------
    print(f"\n## Identical workload, N = {args.fixed_n:,}\n")
    want = expected(args.fixed_n)
    for impl in [i for i in impls if i.available]:
        try:
            impl.fixed_total_ms, _median, output = best_of(
                impl, args.fixed_n, args.repeats
            )
            got = normalise(output)
            if impl.group == "exact":
                ok = got == want
            else:
                # `or 1.0` was wrong here: a relative error of exactly 0.0 is
                # falsy, so every float implementation that got the answer
                # perfectly right was recorded as wrong.
                error = float_error(args.fixed_n, got)
                ok = error is not None and error <= 1e-12
            if not ok:
                impl.available = False
                impl.note = f"wrong answer: {got!r} != {want}"
        except Exception as error:
            impl.available = False
            impl.note = f"failed: {error}"

    print(f"All must print `{want}`. Wall clock, including startup.\n")
    print("| Implementation | Arithmetic | Total |")
    print("|---|---|---|")
    for impl in sorted((i for i in impls if i.available), key=lambda i: i.fixed_total_ms):
        print(f"| {impl.name} | {impl.group} | {impl.fixed_total_ms:.0f} ms |")

    # --- Per iteration ---------------------------------------------------
    print(
        f"\n## Per-iteration cost\n\nMeasured as `T(2N) - T(N)`, which cancels "
        f"process creation, runtime startup and printing. Each language "
        f"calibrated to about {args.target_ms:.0f} ms of its own compute.\n"
    )
    for impl in [i for i in impls if i.available]:
        try:
            impl.n = calibrate(impl, args.target_ms, args.ceiling)
            median_diff, min_diff, max_diff, low_out, high_out = interleaved(
                impl, impl.n, args.repeats
            )

            for n, out in ((impl.n, low_out), (2 * impl.n, high_out)):
                got = normalise(out)
                if impl.group == "exact":
                    if got != expected(n):
                        raise RuntimeError(f"wrong answer at N={n}: {got!r}")
                else:
                    # A binary float is expected to drift here; hold it to a
                    # tolerance and keep the error, because at these workload
                    # sizes it is the most interesting number in the row.
                    error = float_error(n, got)
                    if error is None or error > 1e-6:
                        raise RuntimeError(
                            f"answer at N={n} is off by {error}: {got!r}"
                        )
                    impl.rel_error = max(impl.rel_error, error)

            impl.ns_per_iter = max(median_diff, 0.0) * 1e6 / impl.n
            impl.ns_low = max(min_diff, 0.0) * 1e6 / impl.n
            impl.ns_high = max(max_diff, 0.0) * 1e6 / impl.n
            impl.reduced = impl.ns_per_iter < REDUCED_NS
        except Exception as error:
            impl.available = False
            impl.note = f"failed: {error}"

    timed = [i for i in impls if i.available and i.ns_per_iter > 0]
    baseline = min((i.ns_per_iter for i in timed if not i.reduced), default=1.0)

    for group, title in (
        ("exact", "Exact decimal arithmetic — what eTamil guarantees"),
        ("float", "Binary float — fast, and inexact for money"),
    ):
        rows = sorted((i for i in timed if i.group == group), key=lambda i: i.ns_per_iter)
        if not rows:
            continue
        print(f"\n### {title}\n")
        error_column = group == "float"
        header = (
            "| Implementation | Runtime | N | ns / iteration "
            "| observed range | Relative |"
        )
        rule = "|---|---|---|---|---|---|"
        if error_column:
            header += " Answer wrong by |"
            rule += "---|"
        print(header)
        print(rule)
        for impl in rows:
            flag = " ⚠︎ reduced" if impl.reduced else ""
            row = (
                f"| {impl.name} | {impl.runtime} | {impl.n:,} "
                f"| **{impl.ns_per_iter:.1f}**{flag} "
                f"| {impl.ns_low:.1f} – {impl.ns_high:.1f} "
                f"| {impl.ns_per_iter / baseline:.0f}× |"
            )
            if error_column:
                row += (
                    " exact |" if impl.rel_error == 0 else f" {impl.rel_error:.1e} |"
                )
            print(row)

    if any(i.reduced for i in timed):
        print(
            "\n⚠︎ *Loop reduced*: the difference between N and 2N collapsed, so "
            "the optimiser recognised this triangular sum and replaced the loop "
            "with a closed form. Those rows measure how fast a compiler can "
            "avoid the work, not how fast the arithmetic is. Treat them as a "
            "floor.\n"
        )

    # --- Growth of இணை ----------------------------------------------------
    # Kept separate because it is not a per-iteration cost at all. `இணை`
    # clones the whole array before pushing, so building a list of n items
    # copies about n^2/2 elements. For the accounting framework — which builds
    # a ledger by appending one transaction at a time — this dominates
    # everything the loop benchmark measures, and no amount of faster dispatch
    # would fix it.
    append_program = HERE / "append.qmz"
    if ETAMIL.exists() and append_program.exists():
        print("\n## Building a list by appending\n")
        print(
            "Doubling N should double the time. Quadrupling it means the whole "
            "array is being copied on every append.\n"
        )
        appender = Impl(
            "eTamil — append",
            "exact",
            "etamil, bytecode VM",
            lambda n: [ETAMIL, "--vm", "append.qmz"],
            lambda: [ETAMIL, "--vm", "empty.qmz"],
            stdin_n=True,
        )
        etamil_startup = next(
            (i.empty_ms for i in impls if i.name.startswith("eTamil")), 0.0
        )

        print("| N | time | growth |")
        print("|---|---|---|")
        previous = None
        for n in (2_000, 4_000, 8_000, 16_000):
            try:
                best, _median, output = best_of(appender, n, 3)
                if normalise(output) != str(n):
                    print(f"| {n:,} | wrong answer: `{normalise(output)}` | |")
                    continue
                compute = max(best - etamil_startup, 0.0)
                growth = "" if previous is None else f"x{compute / previous:.1f}"
                print(f"| {n:,} | {compute:.0f} ms | {growth} |")
                previous = max(compute, 0.01)
            except Exception as error:
                print(f"| {n:,} | failed: {error} | |")

        print(
            "\nA factor near 2 is linear; near 4 is quadratic. `இணை` in "
            "`interpreter.rs` does `items.clone()` then `push`, so quadratic is "
            "expected until append can mutate in place.\n"
        )

    if unavailable:
        print("\n## Not measured\n")
        for impl in unavailable:
            print(f"- **{impl.name}** — {impl.note}")

    problems = [i for i in impls if not i.available and i.note]
    if problems:
        print("\n## Problems\n")
        for impl in problems:
            print(f"- **{impl.name}** — {impl.note}")

    if args.json:
        args.json.write_text(
            json.dumps(
                {
                    "cpu": platform.processor(),
                    "os": f"{platform.system()} {platform.release()}",
                    "spawn_floor_ms": round(floor, 2),
                    "results": [
                        {
                            "name": i.name,
                            "group": i.group,
                            "runtime": i.runtime,
                            "empty_ms": round(i.empty_ms, 2),
                            "fixed_total_ms": round(i.fixed_total_ms, 2),
                            "n": i.n,
                            "ns_per_iter": round(i.ns_per_iter, 3),
                            "ns_low": round(i.ns_low, 3),
                            "ns_high": round(i.ns_high, 3),
                            "loop_reduced": i.reduced,
                            "note": i.note,
                            "samples": i.samples,
                        }
                        for i in impls + unavailable
                    ],
                },
                indent=2,
            ),
            encoding="utf-8",
        )

    return 0


if __name__ == "__main__":
    sys.exit(main())
