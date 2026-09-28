#!/usr/bin/env sh
#
# Check that a built package actually works.
#
#   ./packaging/smoke-test.sh dist/etamil-linux-x64.tar.gz
#
# Run by the release workflow before anything is published: a package that
# cannot execute its own examples is not a package, and finding that out after
# the release is worse than failing the build.
#
# This is a script rather than a block of YAML on purpose. The checks involve
# Tamil literals and a piped stdin, both of which are easy to get wrong through
# a workflow's quoting, and impossible to test without pushing a tag. Here it
# can be run locally exactly as CI runs it.

set -eu

ARCHIVE="${1:-}"
[ -n "$ARCHIVE" ] || { echo "usage: $0 <archive>" >&2; exit 2; }
[ -f "$ARCHIVE" ] || { echo "no such archive: $ARCHIVE" >&2; exit 2; }

ARCHIVE="$(cd "$(dirname "$ARCHIVE")" && pwd)/$(basename "$ARCHIVE")"
BASE="$(basename "$ARCHIVE")"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"

case "$BASE" in
    *.zip)    unzip -q "$ARCHIVE" ;;
    *.tar.gz) tar xzf "$ARCHIVE" ;;
    *)        echo "unrecognized archive: $BASE" >&2; exit 2 ;;
esac

DIR="${BASE%.tar.gz}"
DIR="${DIR%.zip}"
[ -d "$DIR" ] || { echo "archive did not contain $DIR" >&2; ls -la >&2; exit 1; }

BIN="$DIR/etamil"
if [ -f "$BIN.exe" ]; then
    BIN="$BIN.exe"
fi
[ -x "$BIN" ] || { echo "no runnable binary at $BIN" >&2; ls -la "$DIR" >&2; exit 1; }

# ETAMIL_PATH is what makes  இறக்கு "nUlakam/paNam/paNam.qmz"  resolve from anywhere,
# so testing without it would miss the thing most likely to be packaged wrong.
ETAMIL_PATH="$WORK/$DIR"
export ETAMIL_PATH

failures=0

# expect <description> <substring> <file-or-empty-for-no-stdin> <program...>
#
# Output is captured and matched with `case` rather than piped to grep: `grep -q`
# exits at the first match and closes the pipe, which under `set -o pipefail`
# turns a passing check into a SIGPIPE failure. Capturing also means the log
# shows what actually came out when a check fails.
expect() {
    description="$1"
    wanted="$2"
    stdin_file="$3"
    shift 3

    if [ -n "$stdin_file" ]; then
        output="$("$@" < "$stdin_file" 2>&1)" || {
            echo "FAIL  $description — exited non-zero"
            echo "$output" | sed 's/^/        /'
            failures=$((failures + 1))
            return 0
        }
    else
        output="$("$@" 2>&1)" || {
            echo "FAIL  $description — exited non-zero"
            echo "$output" | sed 's/^/        /'
            failures=$((failures + 1))
            return 0
        }
    fi

    case "$output" in
        *"$wanted"*)
            echo "ok    $description"
            ;;
        *)
            echo "FAIL  $description — expected to see: $wanted"
            echo "$output" | sed 's/^/        /'
            failures=$((failures + 1))
            ;;
    esac
}

echo "Testing $BASE"

expect "reports its version" "etamil" "" "$BIN" --version

echo "950000" > income.txt
expect "runs the income tax example" "High Tax Bracket" income.txt \
    "$BIN" --vm "$DIR/examples/basic_samples/example.qmz"

expect "runs the accounting cycle" "சொத்து" "" \
    "$BIN" --vm "$DIR/examples/finance/kaNakkiyal.qmz"

# The standard library, reached through ETAMIL_PATH from a directory that has
# no nUlakam of its own.
printf 'இறக்கு "nUlakam/paNam/paNam.qmz";\nஅச்சு ரூபாய்(12345678.5);\n' > money.qmz
expect "resolves nUlakam through ETAMIL_PATH" "1,23,45,678.50" "" \
    "$BIN" --vm money.qmz

# --check must reject a bad program, so a non-zero exit here is the pass. Run
# separately because `expect` treats that as a failure.
printf 'ஈர்ம கொடியா = [1,2];\n' > bad.qmz
if "$BIN" --check bad.qmz > check.out 2>&1; then
    echo "FAIL  --check accepts a type error"
    sed 's/^/        /' check.out
    failures=$((failures + 1))
else
    case "$(cat check.out)" in
        *"கொடியா"*) echo "ok    --check rejects a type error" ;;
        *)
            echo "FAIL  --check failed without naming the variable"
            sed 's/^/        /' check.out
            failures=$((failures + 1))
            ;;
    esac
fi

echo
if [ "$failures" -eq 0 ]; then
    echo "$BASE works"
else
    echo "$failures check(s) failed"
    exit 1
fi
