#!/usr/bin/env bash
#
# Build an RPM package (Fedora, RHEL, Rocky, openSUSE) from a Linux release archive.
#
#   bash packaging/build.sh                          # makes dist/etamil-linux-<arch>.tar.gz
#   bash packaging/build-rpm.sh dist/etamil-linux-x64.tar.gz
#   sudo dnf install ./dist/etamil-<version>-1.x86_64.rpm
#
# Needs `rpmbuild` (Debian/Ubuntu: apt install rpm; Fedora: dnf install rpm-build).
#
# The package is the same static musl binary as the archive, so it needs no other
# packages and runs on any distribution. Nothing is compiled here, so the aarch64
# package can be built on an x86-64 machine: rpmbuild only has to be told the target.
#
# The binary finds the standard library in /usr/share/etamil by itself (module.rs
# searches there), so no environment variable is set.
#
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>

set -euo pipefail

ARCHIVE="${1:?usage: build-rpm.sh <dist/etamil-linux-x64.tar.gz | dist/etamil-linux-arm64.tar.gz>}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_VERSION="$(grep -m1 '^version' "$ROOT/etamil_compiler/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
DIST="$ROOT/dist"

case "$(basename "$ARCHIVE")" in
    etamil-linux-x64.tar.gz)   SRC=etamil-linux-x64;   ARCH=x86_64 ;;
    etamil-linux-arm64.tar.gz) SRC=etamil-linux-arm64; ARCH=aarch64 ;;
    *) echo "error: expected etamil-linux-x64.tar.gz or etamil-linux-arm64.tar.gz" >&2; exit 1 ;;
esac

command -v rpmbuild >/dev/null || { echo "error: rpmbuild not found (apt install rpm, or dnf install rpm-build)" >&2; exit 1; }

# RPM forbids '-' in a version. A pre-release such as 1.5.0-rc1 becomes 1.5.0~rc1,
# which RPM sorts before 1.5.0, as a pre-release should.
VERSION="${CARGO_VERSION//-/\~}"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

tar -xzf "$ARCHIVE" -C "$WORK"
STAGE="$WORK/$SRC"
[ -x "$STAGE/etamil" ] || { echo "error: $ARCHIVE has no $SRC/etamil" >&2; exit 1; }

HAS_LSP=0
[ -f "$STAGE/etamil-lsp" ] && HAS_LSP=1

# Directories 0755 and files 0644, apart from the binaries: tar keeps whatever modes the
# archive had.
find "$STAGE/nUlakam" "$STAGE/examples" -type d -exec chmod 0755 {} +
find "$STAGE/nUlakam" "$STAGE/examples" -type f -exec chmod 0644 {} +

SPEC="$WORK/etamil.spec"
{
cat <<EOF
# Nothing is compiled, so there is no debug information to split out, no build-id
# links to make, and no stripping to do: stripping would also fail for a binary of
# another architecture.
%global debug_package %{nil}
%global _build_id_links none
%global __os_install_post %{nil}
# A static binary: let rpm find no library dependencies in it.
AutoReqProv: no

Name:       etamil
Version:    $VERSION
Release:    1
Summary:    eTamil compiler for Tamil FinTech programs
License:    AGPL-3.0-or-later
URL:        https://etamil.in
Packager:   Mohammed Maruff (Esan Maruff) <esan@etamil.in>

%description
A bilingual Tamil/English language and compiler for FinTech programs. The package
carries the standard library and examples. It is a static binary and depends on no
other packages.

%prep
%build

%install
install -D -m 0755 $STAGE/etamil %{buildroot}%{_bindir}/etamil
EOF
if [ "$HAS_LSP" = 1 ]; then
    echo "install -D -m 0755 $STAGE/etamil-lsp %{buildroot}%{_bindir}/etamil-lsp"
fi
cat <<EOF
mkdir -p %{buildroot}%{_datadir}/etamil
cp -r $STAGE/nUlakam $STAGE/examples %{buildroot}%{_datadir}/etamil/

%files
%{_bindir}/etamil
EOF
if [ "$HAS_LSP" = 1 ]; then
    echo "%{_bindir}/etamil-lsp"
fi
cat <<EOF
%{_datadir}/etamil

%changelog
* $(LC_ALL=C date -u '+%a %b %d %Y') Mohammed Maruff (Esan Maruff) <esan@etamil.in> - $VERSION-1
- Packaged from the release archive.
EOF
} >"$SPEC"

mkdir -p "$DIST"
rm -f "$DIST"/etamil-*."$ARCH".rpm
rpmbuild -bb --quiet \
    --target "$ARCH-linux" \
    --define "_topdir $WORK/rpmbuild" \
    "$SPEC"

RPM="$(find "$WORK/rpmbuild/RPMS" -name '*.rpm')"
NAME="$(basename "$RPM")"
cp "$RPM" "$DIST/$NAME"

echo "$DIST/$NAME"
ls -lh "$DIST/$NAME" | awk '{print "  size: " $5}'
(cd "$DIST" && sha256sum "$NAME" | tee "$NAME.sha256")
