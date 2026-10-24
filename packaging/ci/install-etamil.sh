#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
#
# Installs eTamil in any CI system that has sh, curl, tar and sha256sum
# (or shasum). Copy this file into your repository; it needs no other file.
#
#   sh ci/install-etamil.sh [version]       # default: latest
#
# Installs to $PREFIX (default $HOME/.local): bin/etamil and lib/etamil/nUlakam.
# Afterwards:  export PATH="$PREFIX/bin:$PATH" ETAMIL_PATH="$PREFIX/lib/etamil"
#
# ETAMIL_RELEASE_BASE replaces the GitHub release URL (a mirror, or a test);
# ETAMIL_PLATFORM (for example linux-arm64) overrides the detected platform.
set -eu

REPO="${ETAMIL_REPO:-Maruff/eTamil_lang}"
VERSION="${1:-latest}"
VERSION="${VERSION#v}"
PREFIX="${PREFIX:-$HOME/.local}"

if [ -n "${ETAMIL_PLATFORM:-}" ]; then
    platform="$ETAMIL_PLATFORM"
else
    case "$(uname -s)-$(uname -m)" in
        Linux-x86_64)               platform=linux-x64 ;;
        Linux-aarch64|Linux-arm64)  platform=linux-arm64 ;;
        Darwin-x86_64)              platform=macos-x64 ;;
        Darwin-arm64)               platform=macos-arm64 ;;
        *) echo "error: no eTamil package for $(uname -s) $(uname -m)" >&2; exit 1 ;;
    esac
fi
pkg="etamil-$platform.tar.gz"

if [ -n "${ETAMIL_RELEASE_BASE:-}" ]; then
    base="$ETAMIL_RELEASE_BASE"
else
    if [ "$VERSION" = latest ]; then
        # The /releases/latest redirect names the tag: no API call, no rate limit.
        final="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest")"
        VERSION="${final##*/v}"
        [ "$VERSION" != "$final" ] || { echo "error: could not resolve the latest release" >&2; exit 1; }
    fi
    base="https://github.com/$REPO/releases/download/v$VERSION"
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
echo "Installing eTamil $VERSION ($pkg) to $PREFIX"
curl -fsSL -o "$work/$pkg" "$base/$pkg"
curl -fsSL -o "$work/$pkg.sha256" "$base/$pkg.sha256"

expected="$(awk '{print tolower($1)}' "$work/$pkg.sha256")"
if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$work/$pkg" | awk '{print $1}')"
else
    actual="$(shasum -a 256 "$work/$pkg" | awk '{print $1}')"
fi
[ "$actual" = "$expected" ] || { echo "error: checksum mismatch for $pkg (expected $expected, got $actual)" >&2; exit 1; }

tar -xzf "$work/$pkg" -C "$work"
src="$work/etamil-$platform"
mkdir -p "$PREFIX/bin" "$PREFIX/lib/etamil"
cp "$src/etamil" "$PREFIX/bin/etamil"
chmod 755 "$PREFIX/bin/etamil"
# The language server, in every package built since it existed.
if [ -f "$src/etamil-lsp" ]; then
    cp "$src/etamil-lsp" "$PREFIX/bin/etamil-lsp"
    chmod 755 "$PREFIX/bin/etamil-lsp"
fi
rm -rf "$PREFIX/lib/etamil/nUlakam"
cp -r "$src/nUlakam" "$PREFIX/lib/etamil/nUlakam"

"$PREFIX/bin/etamil" --version
echo "Add to your job:  export PATH=\"$PREFIX/bin:\$PATH\" ETAMIL_PATH=\"$PREFIX/lib/etamil\""
