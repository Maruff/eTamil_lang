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

# An Android target is its own OS: the binary links against Android's libc
# (bionic), so it is built on Linux but runs only on Android, where it resolves
# host names through the system resolver. The Linux musl binary runs there too,
# but musl looks for /etc/resolv.conf, which Android does not have, so a program
# calling an API by name cannot reach it. The NDK's clang must be the linker;
# the release workflow sets that up.
case "$TARGET" in
    *-linux-android*) OS=android ;;
esac
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

rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$BUILT" "$STAGE/etamil$EXE"
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
