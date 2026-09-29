#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
"""Audit the romanized names that are not keywords.

`transliterate.py --check` holds the lexer's keyword table to the scheme. It
reads `lexer.rs` and nothing else, which leaves every *other* romanized surface
unaudited — module and file names, SQL tables and columns, and the record keys
those columns become when a row is read.

That is the gap `viziqam` fell through. `விகிதம்` is `vikiqam`; `viziqam` is
`z`, ழ, and spells nothing. It was never a keyword, so nothing checked it, and
it spread from a module name into a table, a column and a second module before
anyone read it aloud.

## How it decides

The scheme assigns one Latin letter per Tamil letter, so it inverts. Run a name
back through `untransliterate` and one of three things happens:

    kaNakkiyal   -> கணக்கியல்      all Tamil. On-scheme; whether it is a *word*
                                    is a question only a reader can answer.
    products     -> ப்ரொdஉச்ட்ஸ்    Tamil with Latin still in it. `d` is not in
                                    the scheme, so it had nothing to become.
    calculator   -> சல்சுலடொர்      all Tamil, because c, a, l, u, t, o and r
                                    all happen to be scheme letters.

The second is the bug this catches: a letter outside the scheme sitting inside
a Tamil word. `b`, `d`, `f`, `g` and `D` are not in the scheme at all, so
`muDivu`, `kadaZ` and `kadai` cannot round-trip and are reported.

The third is why English names need an allowlist and why this audit cannot be
total: a name spelled only in scheme letters produces Tamil whether or not
Tamil was meant. Digraphs borrowed from ISO 15919 have the same problem — `ai`,
`zh` and `th` are all scheme letters in sequence — so they are checked by name.

## What it does not do

It cannot tell a real Tamil word from a plausible one. `viziqam` reverses
cleanly to விழிதம், which is on-scheme and means nothing; only a reader caught
that. This narrows the field to names a reader should look at, and stops the
letters that are wrong on their face.
"""
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from transliterate import untransliterate  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
SCANNED = ("nUlakam", "examples")

TAMIL = re.compile(r"[஀-௿]")
LATIN = re.compile(r"[A-Za-z]")

# English names that contain a letter the scheme does not assign, so they would
# otherwise be reported every run. English by intent, not Tamil gone wrong.
ALLOW = {
    "README", "README_EXAMPLES", "README_TEST_FILES",
    "fabric", "fabric_cOqaZY", "fifo", "gateway",
    "backend", "basic_samples", "db_samples", "db_commands_demo",
    "finance", "language", "io_samples",
    "data", "id", "numbers", "products", "students",
    "fileio_example", "simple_fileio", "loop_server", "student_management",
    # Field names on somebody else's wire format, which cannot be renamed:
    # "embeddings" is what Ollama calls the vectors it returns.
    "embeddings",
    # "category" is the state category Azure DevOps returns with each state.
    "category",
}

# ISO 15919 spellings that survive the round trip because their letters are all
# in the scheme, and so have to be named rather than derived.
DIGRAPHS = {
    "ai": "Y (ஐ)", "zh": "z (ழ)", "th": "q (த)", "ng": "w (ங)",
    "sh": "S (ஷ)", "nj": "W (ஞ)", "aa": "A (ஆ)", "ee": "E (ஏ)",
    "ii": "I (ஈ)", "oo": "O (ஓ)", "uu": "U (ஊ)",
}

KEY = re.compile(r'\["([A-Za-z_]+)"\]')
COLUMN = re.compile(r"^\s+([A-Za-z_]+)\s+(?:TEXT|REAL|INTEGER|NUMERIC|BLOB)", re.M)
TABLE = re.compile(
    r"(?:CREATE\s+TABLE(?:\s+IF\s+NOT\s+EXISTS)?|INSERT\s+INTO|FROM|UPDATE)"
    r"\s+([A-Za-z_]+)"
)


def names():
    """Every romanized name, with where it came from."""
    for top in SCANNED:
        # Sort the POSIX string rather than the Path. Path ordering is
        # case-insensitive on Windows and case-sensitive everywhere else, so
        # sorting Path objects makes the output depend on which machine ran
        # this — aNi.qmz and AvaNam.qmz swap places, and CI then rejects a
        # file that was correct when it was generated.
        for path in sorted((ROOT / top).rglob("*"), key=lambda p: p.as_posix()):
            if not path.is_file():
                continue
            rel = path.relative_to(ROOT).as_posix()
            for part in list(path.parent.relative_to(ROOT).parts)[1:] + [path.stem]:
                yield part, rel
            if path.suffix not in (".qmz", ".sql"):
                continue
            body = path.read_text(encoding="utf-8", errors="ignore")
            for pattern in (KEY, COLUMN, TABLE):
                for m in pattern.finditer(body):
                    yield m.group(1), rel


def check() -> int:
    findings: dict[tuple, set] = {}
    for name, where in names():
        if name in ALLOW or len(name) < 3:
            continue
        back = untransliterate(name)
        if TAMIL.search(back) and LATIN.search(back):
            stray = "".join(sorted({c for c in back if c.isascii() and c.isalpha()}))
            findings.setdefault((name, back, f"{stray} not in the scheme"), set()).add(where)
        elif TAMIL.search(back):
            for digraph, want in DIGRAPHS.items():
                if digraph in name:
                    findings.setdefault(
                        (name, back, f"'{digraph}' is one letter: {want}"), set()
                    ).add(where)

    print(f"checked {len(SCANNED)} trees, {len(findings)} names off-scheme\n")
    for (name, back, why) in sorted(findings):
        print(f"{name:<26} reads back as {back:<26} {why}")
        for where in sorted(findings[(name, back, why)])[:3]:
            print(f"    {where}")
    return len(findings)


if __name__ == "__main__":
    if sys.argv[1:2] == ["--check"]:
        sys.exit(0 if check() == 0 else 1)
    print(__doc__)
