#!/usr/bin/env bash
#
# Print the environment that points cargo and cc at the NDK's clang for an
# Android target, for a workflow to append to $GITHUB_ENV:
#
#   bash packaging/android-env.sh "$NDK" aarch64-linux-android >> "$GITHUB_ENV"
#
# API level 24 is Android 7.0, the oldest Android that qos runs on, so a binary
# built here runs on every phone qos does.

set -euo pipefail

NDK="${1:?usage: $0 <ndk-path> <rust-target>}"
TARGET="${2:?usage: $0 <ndk-path> <rust-target>}"
API="${ANDROID_API:-24}"

BIN="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin"
CLANG="$BIN/$TARGET$API-clang"
[ -x "$CLANG" ] || { echo "no NDK clang at $CLANG" >&2; exit 1; }

# cargo wants the target upper-cased with underscores; cc wants it lower-cased.
UPPER="$(printf '%s' "$TARGET" | tr 'a-z-' 'A-Z_')"
LOWER="$(printf '%s' "$TARGET" | tr '-' '_')"

echo "CARGO_TARGET_${UPPER}_LINKER=$CLANG"
echo "CC_${LOWER}=$CLANG"
echo "AR_${LOWER}=$BIN/llvm-ar"
