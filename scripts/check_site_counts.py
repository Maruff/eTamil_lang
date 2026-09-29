#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
"""Audit the keyword counts published on etamil.in against the lexer.

The website states how many keywords the language has in thirteen places --
the home page's statistics, the keyword reference, the manual, the status
table, the tour -- and every one of them is a number typed by hand. They go
stale the moment a keyword lands, and they did: a token and five standard
library functions were added and the site went on saying 202 tokens across 541
spellings when the answer was 203 and 545. Nobody reading the site can tell.

It also compares the version the site advertises with the compiler's own. That
is the one field a release must edit by hand -- the download URLs beside it are
unversioned, behind GitHub's /latest/ redirect -- and at 1.1.0 it was missed.

`eTamil_Code/test/counts.test.js` already guards the extension's figures for
this reason. This does the same for the site, from the same source: the token
table in `lexer.rs`, read through `generate_editor_support.py` rather than
parsed a second time here.

    python scripts/check_site_counts.py
    python scripts/check_site_counts.py --check          # exit 1 on a mismatch
    python scripts/check_site_counts.py --site ../eTamil # an explicit checkout

The site is a separate repository, so it has to be found before it can be
read: `--site`, then `$ETAMIL_SITE`, then `../eTamil_site` and `../eTamil`
beside this one. **Finding no checkout is not a failure** -- it reports that it
skipped and exits 0, so the check is a no-op for anyone who has only cloned the
compiler, and CI adds a second checkout to make it gate.

What is not checked, deliberately: a sentence recording what a past change did.
`524 spellings across 202 tokens, up from 505` is history and its numbers are
the point of it, so any line saying `up from` is left alone.

The patterns below are the phrasings the site actually uses. A new way of
writing the same claim needs a pattern adding, which is the honest limitation
of checking prose: this catches the sentences it knows, and says which ones
those are.
"""

from __future__ import annotations

import importlib.util
import os
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GENERATOR = ROOT / "scripts" / "generate_editor_support.py"
CARGO = ROOT / "etamil_compiler" / "Cargo.toml"

SKIP_DIRS = {"node_modules", "_site_preview", ".git", "worktrees", ".jekyll-cache"}
# Release notes record what was true at a release and must not be rewritten.
SKIP_FILES = {"CHANGELOG.md"}
HISTORICAL = "up from"

SUFFIXES = {".md", ".html"}


def counts() -> dict[str, int]:
    """The authoritative figures, from the lexer's own token table."""
    spec = importlib.util.spec_from_file_location("gen", GENERATOR)
    generator = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(generator)
    # classify() is what marks a token reserved -- read_tokens() only reads the
    # table. The generator does the same two calls before it counts.
    tokens = generator.classify(generator.read_tokens())
    reserved = sum(1 for token in tokens if token["reserved"])
    return {
        "tokens": len(tokens),
        "spellings": sum(len(token["forms"]) for token in tokens),
        "reserved": reserved,
        "usable": len(tokens) - reserved,
        # The host's own functions, from interpreter.rs, and the library's,
        # from the `செயல்` definitions in nUlakam. Both move more often than
        # the keyword table does: adding a module adds a dozen at a time.
        "builtins": len(generator.read_builtins()),
        "stdlib": len(generator.read_stdlib()),
    }


def released_version() -> str:
    """The compiler's own version, from the package it publishes."""
    with CARGO.open("rb") as handle:
        return tomllib.load(handle)["package"]["version"]


def site_version(root: Path) -> str | None:
    """`brand.version` from _config.yml, read without a YAML dependency.

    The file is Jekyll's and the key is two spaces under `brand:`, so a state
    machine over the lines is enough and keeps this script's dependencies at
    the standard library, as its neighbours in scripts/ do.
    """
    config = root / "_config.yml"
    if not config.is_file():
        return None
    in_brand = False
    for line in config.read_text(encoding="utf-8").splitlines():
        if re.match(r"^\S", line):
            in_brand = line.startswith("brand:")
            continue
        if in_brand:
            match = re.match(r'^  version:\s*"?([^"\s]+)"?', line)
            if match:
                return match.group(1)
    return None


def site_root(argv: list[str]) -> Path | None:
    if "--site" in argv:
        return Path(argv[argv.index("--site") + 1]).resolve()
    if os.environ.get("ETAMIL_SITE"):
        return Path(os.environ["ETAMIL_SITE"]).resolve()
    for name in ("eTamil_site", "eTamil"):
        candidate = ROOT.parent / name
        if (candidate / "_config.yml").is_file():
            return candidate
    return None


# Each pattern maps its capture groups onto the figures they claim to be.
PATTERNS: list[tuple[re.Pattern[str], tuple[str, ...]]] = [
    (re.compile(r"(\d+) tokens across (\d+) spellings"), ("tokens", "spellings")),
    (re.compile(r"(\d+) keywords across (\d+) spellings"), ("tokens", "spellings")),
    (re.compile(r"<b>(\d+)</b><span>keyword tokens"), ("tokens",)),
    (re.compile(r"all (\d+) (?:keywords|tokens)"), ("tokens",)),
    (re.compile(r"of the (\d+) keywords"), ("tokens",)),
    (re.compile(r"Lexer \((\d+) keywords"), ("tokens",)),
    (re.compile(r"defines (\d+) keywords"), ("tokens",)),
    (re.compile(r"Reserving all (\d+)"), ("tokens",)),
    (
        re.compile(r"Of the (\d+) keywords, \*\*(\d+) are reserved and (\d+) are"),
        ("tokens", "reserved", "usable"),
    ),
    (re.compile(r"(\d+) builtins"), ("builtins",)),
    (re.compile(r"(\d+) `nUlakam` functions"), ("stdlib",)),
    (re.compile(r"(\d+) standard library functions"), ("stdlib",)),
]


def check(root: Path, truth: dict[str, int]) -> list[str]:
    findings: list[str] = []
    for path in sorted(root.rglob("*")):
        if path.suffix not in SUFFIXES or not path.is_file():
            continue
        if path.name in SKIP_FILES or any(part in SKIP_DIRS for part in path.parts):
            continue
        rel = path.relative_to(root).as_posix()
        for number, line in enumerate(path.read_text(encoding="utf-8").split("\n"), 1):
            if HISTORICAL in line:
                continue
            for pattern, fields in PATTERNS:
                for match in pattern.finditer(line):
                    for value, field in zip(match.groups(), fields):
                        if int(value) != truth[field]:
                            findings.append(
                                f"{rel}:{number}  says {value} {field}, "
                                f"the lexer has {truth[field]}\n"
                                f"    {match.group(0)[:80]}"
                            )
    return findings


def main(argv: list[str]) -> int:
    truth = counts()
    findings: list[str] = []

    # This repository states the same figures the website does -- the feature
    # table in README.md, the roadmap, the architecture notes -- and nothing
    # was checking them. README.md claimed 62 builtins and 696 nUlakam
    # functions when the answer was 96 and 939, and had said so long enough
    # that nobody could say when it went wrong.
    #
    # Checked first and always, because it needs no second checkout: whoever
    # has only this repository still gets an answer about it.
    findings += check(ROOT, truth)

    root = site_root(argv)
    if root is None:
        print(
            "no website checkout found -- the site was not checked. "
            "Pass --site, set ETAMIL_SITE, or clone Maruff/eTamil.in beside "
            "this repository."
        )
        for finding in findings:
            print(finding)
        return len(findings)

    # The one field a release has to edit by hand. The download URLs beside it
    # in _config.yml are deliberately unversioned, behind GitHub's /latest/
    # redirect, so they need no release-time edit; brand.version does, and at
    # 1.1.0 it was missed -- the site went on saying 1.0.0 in four places while
    # the compiler was tagged v1.1.0.
    released = released_version()
    published = site_version(root)
    if published is None:
        findings.append(
            "_config.yml  brand.version could not be read -- the check cannot "
            "compare it with the compiler's"
        )
    elif published != released:
        findings.append(
            f"_config.yml  brand.version is {published}, "
            f"etamil_compiler/Cargo.toml is {released}"
        )

    findings += check(root, truth)
    print(
        f"checked {root.name} against the lexer: {truth['tokens']} tokens, "
        f"{truth['spellings']} spellings, {truth['reserved']} reserved, "
        f"{truth['usable']} usable as names, {truth['builtins']} builtins, "
        f"{truth['stdlib']} stdlib functions, version {released} "
        f"-- {len(findings)} stale\n"
    )
    for finding in findings:
        print(finding)
    return len(findings)


if __name__ == "__main__":
    if "--check" in sys.argv:
        sys.exit(0 if main(sys.argv) == 0 else 1)
    main(sys.argv)
