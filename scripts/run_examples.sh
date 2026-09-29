#!/usr/bin/env bash
#
# Run every example and check each one behaves as expected.
#
# Some examples are expected to FAIL: four use route statements the VM cannot
# execute, and the artino ones are firmware. Failing loudly is the intended
# behaviour. This script fails if
# any other example breaks, or if one of those starts passing without the
# expectation being updated.
#
# Examples needing a database server that this repository does not provide are
# skipped unless their guard variable is set, so a plain run stays hermetic.
#
# The uqavi/ sample programs are not run here. There are hundreds of them and
# each costs about a second, which would take this gate from three minutes to
# twenty. scripts/run_samples.sh runs those, and CI gives it its own job so the
# two run at the same time rather than one after the other.
#
#   ./scripts/run_examples.sh
#   ETAMIL_TEST_MYSQL=1 ./scripts/run_examples.sh

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
        echo "       build it first: (cd etamil_compiler && cargo build --release)"
        exit 1
    fi
fi

# இறக்கு resolves relative to the importing file first, but set this so
# examples work no matter where they are run from.
export ETAMIL_PATH="$ROOT"
# Programs run from a scratch directory, so the office service's test is told
# where its Word templates are, as the service itself would be.
export ALUVALAKAM_TEMPLATES="$ROOT/examples/aluvalakam/vArppukaL"

# Examples that must fail, and the text their error must contain.
declare -A EXPECT_FAIL=(
    ["examples/api/simple_api.qmz"]="not implemented"
    ["examples/api/vari_cEvY.qmz"]="not implemented"
    ["examples/katY/katY_cEvY.qmz"]="not implemented"
    # Routing only; what it routes to is aluvalakam_kYyALi.qmz, tested by
    # aluvalakam_kYyALi_cOqaZY.qmz here under --vm.
    ["examples/aluvalakam/aluvalakam_cEvY.qmz"]="not implemented"
    # Firmware, for artino: the VM runs no இடைவெளி blocks and has no C++ to
    # call. scripts/artino_conformance.sh is where these are tested.
    ["examples/artino/minnu.qmz"]="not implemented"
    ["examples/artino/pukY.qmz"]="not implemented"
    ["examples/artino/kaqavu.qmz"]="unknown function"
    ["examples/artino/viLakku.qmz"]="unknown function"
    ["examples/artino/veppam.qmz"]="unknown function"
)

# Programs that need a board, run on the simulated one: they set pins and feed
# serial ports themselves, so they are hermetic too.
declare -A SIM_BOARD=(
    ["nUlakam/vaZporuL/vaZporuL_cOqaZY.qmz"]=1
    ["examples/artino/minnu.qmz"]=1
    ["examples/artino/pukY.qmz"]=1
)

# Examples needing an external server, and the variable that opts them in.
# The SQLite sample is not here: that driver is bundled and writes a file in
# the temp directory below, so it runs anywhere.
declare -A NEEDS_SERVER=(
    ["examples/db_samples/mYcIkul_qaLam.qmz"]="ETAMIL_TEST_MYSQL"
)

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

pass=0
fail=0
skip=0
declare -a FAILURES=()

while IFS= read -r file; do
    rel="${file#"$ROOT"/}"

    # Opt-in examples: skipped, and said out loud, rather than counted as
    # passing — a silent skip reads as coverage that is not there.
    guard="${NEEDS_SERVER[$rel]:-}"
    if [[ -n "$guard" && -z "${!guard:-}" ]]; then
        echo "  skipped          $rel (set $guard=1 to run it)"
        ((skip++))
        continue
    fi

    # I/O examples create files; keep the repository clean.
    board=""
    [[ -n "${SIM_BOARD[$rel]:-}" ]] && board="sim"
    output="$(cd "$WORK" && echo "0" | ETAMIL_BOARD="$board" "$BIN" --vm "$file" 2>&1)"
    status=$?

    expected="${EXPECT_FAIL[$rel]:-}"

    if [[ -n "$expected" ]]; then
        if [[ $status -eq 0 ]]; then
            echo "  UNEXPECTED PASS  $rel"
            echo "                   expected it to fail with: $expected"
            FAILURES+=("$rel (unexpected pass)")
            ((fail++))
        elif grep -qF "$expected" <<<"$output"; then
            echo "  fails as designed  $rel"
            ((pass++))
        else
            echo "  WRONG ERROR      $rel"
            echo "                   wanted: $expected"
            echo "                   got:    $(head -n 3 <<<"$output")"
            FAILURES+=("$rel (wrong error)")
            ((fail++))
        fi
    else
        if [[ $status -eq 0 ]]; then
            echo "  ok               $rel"
            ((pass++))
        else
            echo "  FAILED           $rel"
            sed 's/^/                   /' <<<"$(head -n 5 <<<"$output")"
            FAILURES+=("$rel")
            ((fail++))
        fi
    fi
done < <(find "$ROOT/examples" "$ROOT/nUlakam" \
             -type f -name '*.qmz' -not -path '*/uqavi/*' | sort)

echo
echo "-------------------------------------------"
echo "  $pass as expected, $fail unexpected, $skip skipped"

if [[ $fail -gt 0 ]]; then
    echo
    echo "Unexpected results:"
    printf '  - %s\n' "${FAILURES[@]}"
    exit 1
fi

echo "  all examples behaved as expected"
