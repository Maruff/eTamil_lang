#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
"""Keep the VS Code extension's README counts in step with the compiler.

The extension lives in Maruff/eTamil_vsCode, and its README states how many
keywords, spellings, builtins and `nUlakam` functions it knows about, and how
many example programs it carries. Its own `counts.test.js` fails when any of
them disagrees with the data this repository generates -- which is right, and
also meant that every regeneration after a builtin or a library function was
added broke the extension's CI until someone edited the README by hand. Twice
in one day.

The figures are derived, exactly as the generated grammar and completion data
beside them are, so they are fixed the same way: by a script, from the same
source. The publish workflow runs this after it regenerates, and the README
goes out in the same commit as the data it describes.

    python scripts/fix_extension_counts.py                    # fix in place
    python scripts/fix_extension_counts.py --check            # exit 1 if stale
    python scripts/fix_extension_counts.py --extension ../eTamil_vsCode

The extension is found by `--extension`, then `$ETAMIL_EXTENSION`, then
`eTamil_Code` inside this repository (where the publish workflow checks it out)
and `../eTamil_vsCode` beside it. Finding none is not a failure.

The claims are the ones `counts.test.js` reads, plus the `N-function nUlakam`
line that states the same figure without being tested. The example count is
written as a word, and is spelled here for any number rather than looked up in
a table that has to be extended when the count outgrows it.
"""

import importlib.util
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

UNITS = ["", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
         "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen",
         "seventeen", "eighteen", "nineteen"]
TENS = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"]


def spelled(number: int) -> str:
    """A whole number from 1 to 999 as English words: forty-three."""
    if number < 20:
        return UNITS[number]
    if number < 100:
        tens, units = divmod(number, 10)
        return TENS[tens] + ("-" + UNITS[units] if units else "")
    hundreds, rest = divmod(number, 100)
    return UNITS[hundreds] + " hundred" + (" and " + spelled(rest) if rest else "")


WORD = r"([a-z]+(?:-[a-z]+)?(?: hundred(?: and [a-z]+(?:-[a-z]+)?)?)?)"

# Each pattern, the figure its groups claim, and whether the figure is a word.
PATTERNS = [
    (re.compile(r"All (\d+) keywords across (\d+) spellings"), ("tokens", "spellings"), False),
    (re.compile(r"(\d+) host builtins"), ("builtins",), False),
    (re.compile(r"all (\d+) `செயல்` functions"), ("stdlib",), False),
    (re.compile(r"the (\d+)-function `nUlakam`"), ("stdlib",), False),
    (re.compile(WORD + r" example programs"), ("examples",), True),
    (re.compile(r"— " + WORD + r" programs, carried"), ("examples",), True),
]


def truth() -> dict[str, int]:
    """The figures, from the same source check_site_counts.py reads."""
    spec = importlib.util.spec_from_file_location("sitecounts", ROOT / "scripts" / "check_site_counts.py")
    sitecounts = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(sitecounts)
    figures = sitecounts.counts()
    # What counts.test.js counts: every .qmz under this repository's examples.
    figures["examples"] = sum(1 for _ in (ROOT / "examples").rglob("*.qmz"))
    return figures


def extension_root(argv: list[str]) -> Path | None:
    if "--extension" in argv:
        return Path(argv[argv.index("--extension") + 1]).resolve()
    for candidate in (os.environ.get("ETAMIL_EXTENSION"), ROOT / "eTamil_Code", ROOT.parent / "eTamil_vsCode"):
        if candidate and (Path(candidate) / "README.md").is_file() and (Path(candidate) / "package.json").is_file():
            return Path(candidate).resolve()
    return None


def reconcile(text: str, figures: dict[str, int]) -> tuple[str, list[str]]:
    """The README with every claim made true, and what was stale."""
    stale: list[str] = []
    for pattern, fields, as_word in PATTERNS:

        def truthful(match: re.Match[str], fields=fields, as_word=as_word) -> str:
            claim = match.group(0)
            offset = match.start()
            for group in range(len(fields), 0, -1):
                start, end = match.span(group)
                wanted = figures[fields[group - 1]]
                said = match.group(group)
                right = spelled(wanted) if as_word else str(wanted)
                if said != right:
                    stale.append(f"says {said} {fields[group - 1]}, the compiler has {right}")
                claim = claim[: start - offset] + right + claim[end - offset:]
            return claim

        text = pattern.sub(truthful, text)
    return text, stale


def main(argv: list[str]) -> int:
    root = extension_root(argv)
    if root is None:
        print("no extension checkout found -- skipped. Pass --extension or set ETAMIL_EXTENSION.")
        return 0
    readme = root / "README.md"
    raw = readme.read_bytes().decode("utf-8")
    fixed, stale = reconcile(raw, truth())
    for finding in stale:
        print(f"README.md  {finding}")
    if "--check" in argv:
        print(f"checked {readme} -- {len(stale)} stale")
        return len(stale)
    if fixed != raw:
        readme.write_bytes(fixed.encode("utf-8"))
        print(f"fixed {len(stale)} figure(s) in {readme}")
    else:
        print(f"{readme} already matches the compiler")
    return 0


if __name__ == "__main__":
    sys.exit(1 if main(sys.argv) else 0)
