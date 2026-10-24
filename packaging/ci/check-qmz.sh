#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
#
# Runs `etamil --check` on every .qmz file under the given folders (default: the
# current one). Nothing is executed. Exits non-zero if any file has an error,
# or if there is no file to check.
#
#   sh ci/check-qmz.sh [folder ...]
set -u
[ "$#" -gt 0 ] || set -- .
count=0
failed=0
list="$(mktemp)"
trap 'rm -f "$list"' EXIT
find "$@" -name '*.qmz' -type f -not -path '*/node_modules/*' -not -path '*/.git/*' | sort > "$list"
while IFS= read -r file; do
    count=$((count + 1))
    if ! etamil --check "$file"; then
        echo "FAILED: $file" >&2
        failed=$((failed + 1))
    fi
done < "$list"
[ "$count" -gt 0 ] || { echo "error: no .qmz files found under: $*" >&2; exit 1; }
echo "checked $count file(s), $failed with errors"
[ "$failed" -eq 0 ]
