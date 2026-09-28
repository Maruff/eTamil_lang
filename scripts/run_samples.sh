#!/usr/bin/env bash
#
# Run every uqavi/ sample and check it still works.
#
# The samples are the library's documentation: one runnable program per
# function, showing what it is for. Documentation that is not run stops being
# true, so these are executed rather than admired — a sample that no longer
# matches its function fails here.
#
# They are separate from run_examples.sh because there are hundreds of them
# and each costs about a second. Folding them in would take that gate from
# three minutes to twenty; CI runs the two as separate jobs, at the same time.
#
#   ./scripts/run_samples.sh                  # all of them
#   ./scripts/run_samples.sh atippatY paNam   # only these folders
#
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${ETAMIL_BIN:-$ROOT/etamil_compiler/target/release/etamil}"
[[ -x "$BIN" ]] || BIN="$BIN.exe"

if [[ ! -x "$BIN" ]]; then
    echo "no compiler at $BIN — run: cargo build --release" >&2
    exit 1
fi

if (($# > 0)); then
    roots=()
    for folder in "$@"; do
        roots+=("$ROOT/nUlakam/$folder/uqavi")
    done
else
    roots=("$ROOT/nUlakam")
fi

# இறக்கு resolves relative to the importing file first; set this so a sample
# runs the same from anywhere.
export ETAMIL_PATH="$ROOT"

# Samples that write files do it here rather than in the repository.
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

pass=0
fail=0
FAILURES=()

while IFS= read -r file; do
    rel="${file#"$ROOT/"}"

    # The hardware samples drive the simulated board, so they need no board
    # and behave the same on every machine.
    board=""
    [[ "$rel" == nUlakam/vaZporuL/uqavi/* ]] && board="sim"

    output="$(cd "$WORK" && ETAMIL_BOARD="$board" "$BIN" run "$file" 2>&1)"
    if grep -q "Execution completed successfully" <<<"$output"; then
        ((pass++))
    else
        echo "  FAILED           $rel"
        # The first ✗ line says what went wrong; the rest is the banner.
        grep -m 1 "✗" <<<"$output" | sed 's/^/                   /'
        FAILURES+=("$rel")
        ((fail++))
    fi
done < <(find "${roots[@]}" -type f -path '*/uqavi/*' -name '*.qmz' 2>/dev/null | sort)

echo
echo "-------------------------------------------"
echo "  $pass samples ran, $fail failed"

if ((fail > 0)); then
    echo
    echo "Samples that did not run:"
    for one in "${FAILURES[@]}"; do
        echo "  $one"
    done
    exit 1
fi

echo "  every sample ran"
