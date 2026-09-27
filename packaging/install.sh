#!/usr/bin/env sh
# eTamil installer for Linux and macOS.
#
# Installs to ~/.local by default, or to $PREFIX. No root needed unless you
# point PREFIX somewhere that requires it. No Rust, no LLVM.
#
#   ./install.sh

set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
BIN="$PREFIX/bin"
LIB="$PREFIX/lib/etamil"

echo "Installing eTamil to $PREFIX"

mkdir -p "$BIN" "$LIB"
install -m 755 "$HERE/etamil" "$BIN/etamil"

for dir in nUlakam examples; do
    if [ -d "$HERE/$dir" ]; then
        rm -rf "$LIB/$dir"
        cp -r "$HERE/$dir" "$LIB/$dir"
    fi
done

echo "  binary   $BIN/etamil"
echo "  library  $LIB/nUlakam"
echo

# ETAMIL_PATH lets  இறக்கு "nUlakam/paNam/paNam.qmz"  resolve from any directory.
LINE="export ETAMIL_PATH=\"$LIB\""
for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
    [ -f "$rc" ] || continue
    if ! grep -qF "$LINE" "$rc"; then
        printf '\n# eTamil standard library\n%s\n' "$LINE" >> "$rc"
        echo "Added ETAMIL_PATH to $rc"
    fi
done

case ":$PATH:" in
    *":$BIN:"*) ;;
    *) echo
       echo "NOTE: $BIN is not on your PATH. Add this to your shell profile:"
       echo "    export PATH=\"\$PATH:$BIN\"" ;;
esac

echo
echo "Done. Open a new shell, then:"
echo "    etamil --version"
echo "    etamil --vm $LIB/examples/basic_samples/example.qmz"
echo
echo "To uninstall: rm -f $BIN/etamil && rm -rf $LIB"
