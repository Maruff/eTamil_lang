#!/usr/bin/env bash
#
# Build a release package for the current platform.
#
#   ./packaging/build.sh                 # native
#   TARGET=x86_64-unknown-linux-musl ./packaging/build.sh
#
# musl is worth it for the Linux package: the default gnu target links against
# the build machine's glibc, so a binary built on a current Ubuntu will not run
# on an older one. musl produces a fully static ELF that runs anywhere.
#
# The CRT is linked statically on Windows so the package does not depend on the
# Visual C++ Redistributable — otherwise the "no dependencies" claim is false on
# a clean machine.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(grep -m1 '^version' "$ROOT/etamil_compiler/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
DIST="$ROOT/dist"

case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) OS=windows; EXE=.exe ;;
    Darwin)               OS=macos;   EXE= ;;
    *)                    OS=linux;   EXE= ;;
esac

TARGET="${TARGET:-}"
# Neither the archive nor the directory inside it carries the version, so that
# every published instruction — the website, the VS Code extension, this repo's
# README — stays correct across releases without being edited. The version is
# still discoverable: it is in README.txt and in `etamil --version`. The stable
# download URL depends on the same thing:
#
#   https://github.com/Maruff/eTamil_lang/releases/latest/download/etamil-windows-x64.zip
# The architecture is derived, not assumed. macOS runners are Apple Silicon
# now, so a hardcoded -x64 would have shipped an arm64 binary under a name
# promising an Intel one — the kind of quiet wrongness this project refuses
# everywhere else.
case "${TARGET:-$(uname -m)}" in
    aarch64*|arm64*) ARCH=arm64 ;;
    *)               ARCH=x64   ;;
esac
NAME="etamil-$OS-$ARCH"
STAGE="$DIST/$NAME"

# Which cargo features the package is built with. Empty keeps a local build
# exactly as it was; the release workflow sets postgres and mysql, because
# someone downloading a binary cannot add a feature to it later. LLVM stays
# out: it needs LLVM installed on the machine that runs it.
FEATURES="${FEATURES:-}"

echo "Building eTamil $VERSION for $OS-$ARCH${TARGET:+ ($TARGET)}${FEATURES:+ [features: $FEATURES]}"

cd "$ROOT/etamil_compiler"
if [ "$OS" = windows ]; then
    RUSTFLAGS="-C target-feature=+crt-static" cargo build --release ${TARGET:+--target "$TARGET"} ${FEATURES:+--features "$FEATURES"}
else
    cargo build --release ${TARGET:+--target "$TARGET"} ${FEATURES:+--features "$FEATURES"}
fi

BUILT="target/${TARGET:+$TARGET/}release/etamil$EXE"
[ -f "$BUILT" ] || { echo "error: $BUILT not produced"; exit 1; }

# The language server rides in the same package, built the same way, so editors
# find it beside the compiler. It links the compiler library without its default
# features, so the cargo features above do not apply to it.
echo "Building etamil-lsp"
(
    cd "$ROOT/etamil_lsp"
    if [ "$OS" = windows ]; then
        RUSTFLAGS="-C target-feature=+crt-static" cargo build --release ${TARGET:+--target "$TARGET"}
    else
        cargo build --release ${TARGET:+--target "$TARGET"}
    fi
)
LSP_BUILT="$ROOT/etamil_lsp/target/${TARGET:+$TARGET/}release/etamil-lsp$EXE"
[ -f "$LSP_BUILT" ] || { echo "error: $LSP_BUILT not produced"; exit 1; }

rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$BUILT" "$STAGE/etamil$EXE"
cp "$LSP_BUILT" "$STAGE/etamil-lsp$EXE"
cp -r "$ROOT/nUlakam" "$STAGE/nUlakam"
cp -r "$ROOT/examples" "$STAGE/examples"
cp "$ROOT/packaging/README.txt" "$STAGE/README.txt"

if [ "$OS" = windows ]; then
    cp "$ROOT/packaging/install.ps1" "$STAGE/install.ps1"
else
    cp "$ROOT/packaging/install.sh" "$STAGE/install.sh"
    chmod +x "$STAGE/install.sh"
fi

sed -i.bak "s/@VERSION@/$VERSION/g; s/@OS@/$OS/g" "$STAGE/README.txt" && rm -f "$STAGE/README.txt.bak"

cd "$DIST"
if [ "$OS" = windows ]; then
    ARCHIVE="$NAME.zip"
    rm -f "$ARCHIVE"
    # Windows' own bsdtar, by absolute path. Two traps here: PowerShell's
    # Compress-Archive writes backslash separators, which the ZIP spec forbids
    # and unzip(1) rejects; and Git Bash's GNU tar ignores -a and would write a
    # tarball named .zip.
    /c/Windows/system32/tar.exe -a -c -f "$ARCHIVE" "$NAME"
else
    ARCHIVE="$NAME.tar.gz"
    rm -f "$ARCHIVE"
    tar czf "$ARCHIVE" "$NAME"
fi

echo
echo "  $DIST/$ARCHIVE"
ls -lh "$ARCHIVE" | awk '{print "  size: " $5}'
command -v sha256sum >/dev/null && sha256sum "$ARCHIVE" | tee "$ARCHIVE.sha256"
echo
echo "Contents:"
find "$NAME" -maxdepth 1 -mindepth 1 | sed 's|^|  |'
