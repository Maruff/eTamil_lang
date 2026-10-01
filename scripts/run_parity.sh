#!/usr/bin/env bash
#
# Compare the two backends: does --llvm produce the same answers as --vm?
#
# The VM is the reference. It runs the whole language, and every example is
# expected to work under it. The LLVM backend covers less, and refuses what it
# cannot build rather than emitting IR that computes something else — so for
# most programs the honest result today is "refused", not "wrong".
#
# That refusal is the point of this script. Each one names the constructs that
# stopped it, and the summary counts them, so the gap between the backends is
# a list of things to build rather than an impression.
#
#   ./scripts/run_parity.sh
#   ./scripts/run_parity.sh --diff nUlakam/upi/upi_cOqaZY.qmz
#
# The second form runs one program under both backends and shows where their
# output parts company. The summary form reports *that* two backends disagree
# and never about what, which made every MISMATCH cost a round trip.
#
# Needs a binary built with the LLVM feature:
#
#   (cd etamil_compiler && cargo build --release --features llvm)
#
# If it finds one built without, it says so and exits 0 — a machine that
# cannot build LLVM (Windows, today) is not a failing machine.
#
# Where clang is on PATH, an accepted program is taken all the way: IR is
# compiled, the binary is run, and its output is compared against the VM's.
# Without clang the IR is accepted as far as it goes and reported separately,
# because "it compiled" and "it computes the same thing" are different claims.
#
# The emitted IR is not self-contained. Every eTamil value in it is a handle
# into an arena in `crate::runtime`, and every operation on one is a call into
# the cdylib that Cargo already builds beside the binary — which is what makes
# decimals exact and every builtin reachable. So the link needs it,
# and needs an rpath so the built program can find it again when it runs.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${ETAMIL_BIN:-$ROOT/etamil_compiler/target/release/etamil}"

# A relative ETAMIL_BIN cannot survive the `cd` into a temporary directory
# below, and the failure is silent in the worst way: every invocation fails
# with "No such file or directory", which this script reads as the program
# being refused rather than as the harness being broken. CI passes a relative
# path, so its parity job was measuring nothing and exiting 0.
case "$BIN" in
    /*) ;;
    *) BIN="$(cd "$(dirname "$BIN")" 2>/dev/null && pwd)/$(basename "$BIN")" ;;
esac

if [[ ! -x "$BIN" ]]; then
    if [[ -x "$BIN.exe" ]]; then
        BIN="$BIN.exe"
    else
        echo "error: $BIN not found."
        echo "       build it first: (cd etamil_compiler && cargo build --release --features llvm)"
        exit 1
    fi
fi

export ETAMIL_PATH="$ROOT"

# Does this binary have the LLVM backend at all?
probe="$(mktemp -d)"
printf 'a = 1;\n' > "$probe/probe.qmz"
llvm_check="$("$BIN" --llvm "$probe/probe.qmz" 2>&1)"
rm -rf "$probe"

if grep -q "LLVM backend is not available" <<<"$llvm_check"; then
    echo "This binary was built without the LLVM backend, so there is nothing to compare."
    echo "  build it with:  (cd etamil_compiler && cargo build --release --features llvm)"
    echo "  Linux and macOS only — see README."
    exit 0
fi

have_clang=0
command -v clang >/dev/null 2>&1 && have_clang=1

# Cargo puts the cdylib beside the binary, so the binary locates it.
RUNTIME_DIR="$(cd "$(dirname "$BIN")" && pwd)"
if [[ ! -e "$RUNTIME_DIR/libetamil_compiler.so" \
   && ! -e "$RUNTIME_DIR/libetamil_compiler.dylib" ]]; then
    echo "error: no runtime library in $RUNTIME_DIR."
    echo "       The emitted IR calls into it, so there is nothing to link against."
    echo "       build it: (cd etamil_compiler && cargo build --release --features llvm)"
    exit 1
fi

# --- one file, shown rather than counted -----------------------------------
if [[ "${1:-}" == "--diff" ]]; then
    target="${2:-}"
    if [[ -z "$target" ]]; then
        echo "usage: $0 --diff <file.qmz>"
        exit 2
    fi
    [[ -f "$target" ]] || target="$ROOT/$target"
    if [[ ! -f "$target" ]]; then
        echo "error: no such file: ${2}"
        exit 2
    fi
    # Absolute from here on. The LLVM run below does its work in a temporary
    # directory, so a relative path stops resolving the moment it cds there --
    # and the failure was not "file not found" but "The backend refuses this
    # program", with an empty list of reasons, because that branch is reached by
    # any non-zero exit. Diagnosing a missing file as a refused program is the
    # worst answer this script could give: it accuses the backend of the one
    # thing the script exists to detect.
    target="$(cd "$(dirname "$target")" && pwd)/$(basename "$target")"

    vm_out="$(cd "$(dirname "$target")" && echo "0" | "$BIN" --vm "$target" 2>&1)"
    vm_body="$(sed -n '/=== Execution Output ===/,$p' <<<"$vm_out" \
               | sed '1d;/^✓ Execution completed successfully$/d' | sed '/^$/d')"

    work="$(mktemp -d)"
    llvm_out="$(cd "$work" && echo "0" | "$BIN" --llvm "$target" 2>&1)"
    if [[ $? -ne 0 ]]; then
        reasons="$(grep -E '^    - ' <<<"$llvm_out" | sed 's/^    - /  /')"
        if [[ -n "$reasons" ]]; then
            echo "The backend refuses this program, so there is nothing to compare:"
            printf '%s\n' "$reasons"
        else
            # No reasons means this was not a refusal at all. Printing the
            # backend's own words beats inventing a diagnosis for it.
            echo "The backend did not produce IR, and did not say it was refusing:"
            printf '%s\n' "$llvm_out" | tail -20
        fi
        rm -rf "$work"
        exit 0
    fi

    if [[ $have_clang -eq 0 ]]; then
        echo "clang is not on PATH, so the IR cannot be run."
        rm -rf "$work"
        exit 2
    fi

    if ! (cd "$work" && clang output.ll -o prog \
              -L "$RUNTIME_DIR" -letamil_compiler \
              -Wl,-rpath,"$RUNTIME_DIR" -lm 2>&1); then
        echo "clang would not build the emitted IR (above)."
        rm -rf "$work"
        exit 1
    fi

    native_body="$(cd "$work" && echo "0" | ./prog 2>&1 | sed '/^$/d')"
    rm -rf "$work"

    if [[ "$vm_body" == "$native_body" ]]; then
        echo "They agree on $(basename "$target")."
        exit 0
    fi

    echo "They disagree. < is the VM, > is the compiled program:"
    echo
    diff <(printf '%s\n' "$vm_body") <(printf '%s\n' "$native_body") || true
    exit 1
fi

# Examples the VM is expected to refuse: they use route statements it cannot
# execute. Nothing to compare when there is no reference answer.
declare -A SKIP=(
    ["examples/api/simple_api.qmz"]=1
    ["examples/api/vari_cEvY.qmz"]=1
    ["examples/katY/katY_cEvY.qmz"]=1
)

# Examples needing an external server, and the variable that opts them in —
# the same list run_examples.sh keeps, for the same reason.
declare -A NEEDS_SERVER=(
    ["examples/db_samples/mYcIkul_qaLam.qmz"]="ETAMIL_TEST_MYSQL"
    ["examples/crypto_samples/vaZcAvi_muqqirY.qmz"]="ETAMIL_TEST_PKCS11"
    ["examples/api/ceyqi_varicY.qmz"]="ETAMIL_TEST_AMQP"
)

# Every program below gets this on stdin, and gets it identically on both
# backends. Two separate reasons, and both of them have bitten:
#
# A program reading input must be fed the same thing by the VM and by the
# compiled binary, or it "disagrees" over what it was handed rather than over
# what it computes.
#
# And a program left attached to this loop's stdin would read whatever the loop
# is reading. run_examples.sh has piped `echo 0` since it was written; this did
# not, and `உள்ளிடு` in the ninth example swallowed the remaining fifty-nine
# filenames off the pipe. The loop hit EOF and printed a tidy summary of nine
# files as though that were the whole corpus.
feed() { echo "0"; }

# The whole list up front, so the loop is not reading from a pipe a child could
# drain. Belt as well as braces: the `feed` above is the other half, and it is
# there for the parity reason rather than this one.
mapfile -t FILES < <(find "$ROOT/examples" "$ROOT/nUlakam" \
                         -type f -name '*.qmz' | sort)

if [[ ${#FILES[@]} -eq 0 ]]; then
    echo "error: no examples found under $ROOT/examples or $ROOT/nUlakam."
    exit 1
fi

matched=0; mismatched=0; refused=0; compiled=0; skipped=0
declare -a MISMATCHES=()
unsupported_log="$(mktemp)"
nearest_log="$(mktemp)"

for file in "${FILES[@]}"; do
    rel="${file#"$ROOT"/}"
    rel="${rel//\\//}"

    if [[ -n "${SKIP[$rel]:-}" ]]; then
        echo "  skipped          $rel (the VM cannot run it either)"
        ((skipped++)); continue
    fi

    # Said out loud rather than counted as a silent skip, because a skip
    # nobody mentions reads as coverage that is not there.
    guard="${NEEDS_SERVER[$rel]:-}"
    if [[ -n "$guard" && -z "${!guard:-}" ]]; then
        echo "  skipped          $rel (set $guard=1 to run it)"
        ((skipped++)); continue
    fi

    # The reference answer. An example the VM cannot run is not a parity
    # question, so it is skipped rather than counted against the backend.
    vm_out="$(cd "$(dirname "$file")" && feed | "$BIN" --vm "$file" 2>&1)"
    if [[ $? -ne 0 ]]; then
        ((skipped++)); continue
    fi

    # --llvm writes output.ll into the working directory, so it runs in a
    # temporary one while the VM runs beside the example. That differs for a
    # program reading a data file by relative path — but every such example is
    # refused by the backend today, so the difference is unreachable. If one
    # ever compiles, it will show up here as a mismatch to look at rather than
    # as a wrong answer nobody sees.
    work="$(mktemp -d)"
    llvm_out="$(cd "$work" && feed | "$BIN" --llvm "$file" 2>&1)"
    llvm_status=$?

    if [[ $llvm_status -ne 0 ]]; then
        # Refused, and it says what stopped it. Those lines are the roadmap.
        # They arrive already de-duplicated per program, so counting them counts
        # distinct reasons — which is what makes the "closest" ranking below
        # mean anything.
        reasons="$(grep -E '^    - ' <<<"$llvm_out" | sed 's/^    - //')"
        if [[ -n "$reasons" ]]; then
            printf '%s\n' "$reasons" >> "$unsupported_log"
            printf '%s\t%s\t%s\n' \
                "$(grep -c . <<<"$reasons")" "$rel" "$(paste -sd '~' <<<"$reasons")" \
                >> "$nearest_log"
        fi
        echo "  refused          $rel"
        ((refused++))
        rm -rf "$work"
        continue
    fi

    if [[ $have_clang -eq 0 ]]; then
        echo "  compiled         $rel  (no clang: IR not run)"
        ((compiled++))
        rm -rf "$work"
        continue
    fi

    link_out="$(cd "$work" && clang output.ll -o prog \
                    -L "$RUNTIME_DIR" -letamil_compiler \
                    -Wl,-rpath,"$RUNTIME_DIR" -lm 2>&1)"
    if [[ $? -ne 0 ]]; then
        echo "  IR REJECTED      $rel  (clang would not build the emitted IR)"
        # The first lines of it, because "clang rejected the IR" on its own is
        # not something anyone can act on.
        sed -n '1,3p' <<<"$link_out" | sed 's/^/                   /'
        MISMATCHES+=("$rel — clang rejected the IR")
        ((mismatched++))
        rm -rf "$work"
        continue
    fi

    native_out="$(cd "$work" && feed | ./prog 2>&1)"

    # The VM prints a banner around a program's output; compare only what the
    # program itself wrote.
    vm_body="$(sed -n '/=== Execution Output ===/,$p' <<<"$vm_out" \
               | sed '1d;/^✓ Execution completed successfully$/d' | sed '/^$/d')"
    native_body="$(sed '/^$/d' <<<"$native_out")"

    if [[ "$vm_body" == "$native_body" ]]; then
        echo "  match            $rel"
        ((matched++))
    else
        echo "  MISMATCH         $rel"
        # Both outputs are in hand right here and were being thrown away, so
        # every mismatch cost a second run with --diff on a machine that has
        # LLVM. Print it now: this is the line somebody actually reads.
        #
        # Capped, because a backend that goes wrong early differs on every line
        # after it and the first difference is the one that matters.
        echo "                   < is the VM, > is the compiled program:"
        diff <(printf "%s\n" "$vm_body") <(printf "%s\n" "$native_body") \
            | head -n 60 | sed 's/^/                   /' || true
        echo "                   the compiled program ended with:"
        printf '%s\n' "$native_body" | tail -n 2 | sed 's/^/                   /'
        MISMATCHES+=("$rel")
        ((mismatched++))
    fi
    rm -rf "$work"
done

echo
echo "-------------------------------------------"
echo "  $matched match, $mismatched mismatch, $refused refused, $compiled compiled-only, $skipped skipped"

# The five counts must account for every file, or the summary is describing a
# run that stopped early — which is exactly how nine files once passed for
# sixty-eight. A summary that cannot be trusted to be complete is worse than no
# summary, because it gets quoted.
accounted=$(( matched + mismatched + refused + compiled + skipped ))
if [[ $accounted -ne ${#FILES[@]} ]]; then
    echo
    echo "  BROKEN HARNESS: $accounted of ${#FILES[@]} files accounted for."
    echo "  The run stopped early, so nothing above is a measurement."
    exit 1
fi
echo "  all ${#FILES[@]} accounted for"

if [[ -s "$unsupported_log" ]]; then
    echo
    echo "  What the LLVM backend still cannot build, most frequent first:"
    sort "$unsupported_log" | uniq -c | sort -rn | head -20 | sed 's/^/    /'
    echo
    echo "  Counts of reasons, not of programs. A program with eight reasons"
    echo "  needs all eight, so that list says what is common rather than what"
    echo "  is on the critical path. This one says what is nearly there:"
    echo
    echo "  Closest to compiling — fewest distinct reasons first:"
    sort -n "$nearest_log" | head -8 | while IFS=$'\t' read -r count rel reasons; do
        printf '    %s (%s)\n' "$rel" "$count"
        tr '~' '\n' <<<"$reasons" | sed 's/^/        /'
    done
fi
rm -f "$unsupported_log" "$nearest_log"

if [[ $mismatched -gt 0 ]]; then
    echo
    echo "  The two backends disagree — a refusal is expected, a wrong answer is not:"
    printf '    - %s\n' "${MISMATCHES[@]}"
    exit 1
fi

echo
echo "  No disagreement: where the LLVM backend accepted a program, it agreed with the VM."
