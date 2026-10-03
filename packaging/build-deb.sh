#!/usr/bin/env bash
#
# Build a Debian/Ubuntu package from a Linux release archive.
#
#   bash packaging/build.sh                          # makes dist/etamil-linux-<arch>.tar.gz
#   bash packaging/build-deb.sh dist/etamil-linux-x64.tar.gz
#   sudo apt install ./dist/etamil_<version>_amd64.deb
#
# The package is the same static musl binary as the archive, so it needs no
# other packages and runs on any Debian or Ubuntu release. Because nothing is
# compiled here, the arm64 package can be built on an x86-64 machine.
#
# The binary finds the standard library in /usr/share/etamil by itself
# (module.rs searches there), so no environment variable is set.
#
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>

set -euo pipefail

ARCHIVE="${1:?usage: build-deb.sh <dist/etamil-linux-x64.tar.gz | dist/etamil-linux-arm64.tar.gz>}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(grep -m1 '^version' "$ROOT/etamil_compiler/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
DIST="$ROOT/dist"

case "$(basename "$ARCHIVE")" in
    etamil-linux-x64.tar.gz)   SRC=etamil-linux-x64;   ARCH=amd64 ;;
    etamil-linux-arm64.tar.gz) SRC=etamil-linux-arm64; ARCH=arm64 ;;
    *) echo "error: expected etamil-linux-x64.tar.gz or etamil-linux-arm64.tar.gz" >&2; exit 1 ;;
esac

PACKAGE="etamil_${VERSION}_${ARCH}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
STAGE="$WORK/stage"

tar -xzf "$ARCHIVE" -C "$WORK"
PKG="$WORK/$SRC"
[ -x "$PKG/etamil" ] || { echo "error: $ARCHIVE has no $SRC/etamil" >&2; exit 1; }

mkdir -p "$STAGE/DEBIAN" "$STAGE/usr/bin" "$STAGE/usr/share/etamil" "$STAGE/usr/share/doc/etamil"
install -m 0755 "$PKG/etamil" "$STAGE/usr/bin/etamil"
cp -r "$PKG/nUlakam" "$PKG/examples" "$STAGE/usr/share/etamil/"

cat >"$STAGE/usr/share/doc/etamil/copyright" <<'EOF'
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: eTamil
Upstream-Contact: Mohammed Maruff (Esan Maruff) <esan@etamil.in>
Source: https://github.com/Maruff/eTamil_lang

Files: *
Copyright: 2026 Mohammed Maruff (Esan Maruff)
License: AGPL-3.0-or-later
 The full text is in /usr/share/common-licenses/AGPL-3 on systems that carry
 it, and at https://www.gnu.org/licenses/agpl-3.0.html.
EOF

# Directories 0755, files 0644, apart from the binary: tar keeps whatever modes
# the archive had, and dpkg-deb rejects unusual directory permissions.
find "$STAGE" -type d -exec chmod 0755 {} +
find "$STAGE/usr/share" -type f -exec chmod 0644 {} +

SIZE="$(du -sk "$STAGE/usr" | cut -f1)"
cat >"$STAGE/DEBIAN/control" <<EOF
Package: etamil
Version: $VERSION
Section: devel
Priority: optional
Architecture: $ARCH
Installed-Size: $SIZE
Maintainer: Mohammed Maruff (Esan Maruff) <esan@etamil.in>
Homepage: https://etamil.in
Description: eTamil compiler for Tamil FinTech programs
 A bilingual Tamil/English language and compiler for FinTech programs. The
 package carries the standard library and examples. It is a static binary and
 depends on no other packages.
EOF

mkdir -p "$DIST"
rm -f "$DIST/$PACKAGE.deb"
dpkg-deb --build --root-owner-group "$STAGE" "$DIST/$PACKAGE.deb" >/dev/null

echo "$DIST/$PACKAGE.deb"
ls -lh "$DIST/$PACKAGE.deb" | awk '{print "  size: " $5}'
(cd "$DIST" && sha256sum "$PACKAGE.deb" | tee "$PACKAGE.deb.sha256")
