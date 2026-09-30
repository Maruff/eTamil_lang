#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
"""Prove that eTamil's PDF output shapes Tamil and Arabic, rather than assuming it.

`_pdf_ஆக்கு` converts a filled document through LibreOffice, which shapes with
HarfBuzz. That has always been the answer to "does eTamil produce correct Tamil
PDFs" and it has never been checked. It is the kind of claim that stays true
until a font, a LibreOffice version or a flag changes, and then stays *stated*
for a year afterwards -- because a PDF full of wrongly-ordered Tamil still
opens, still prints, and still extracts back to the right characters.

## Why not check the extracted text

Extracting text from a PDF reads the ToUnicode map, which gives back the
original codepoints whether or not anything was shaped. A PDF with every Tamil
letter drawn in logical order extracts identically to a correct one. So this
reads the *glyphs* instead, out of the page's content stream, where shaping
either happened or did not.

## The two things it asks

**Tamil reorders.** `கொ` is two codepoints -- க and the vowel sign ொ -- and it
renders as three glyphs, because ொ splits into a left part (ெ) and a right part
(ா) with the consonant between them. So the first glyph of `கொ` must be the same
glyph as the first glyph of `கெ`, and must *not* be the glyph for `க` alone.
Unshaped output puts க first, because that is the order the codepoints are in.

**Arabic joins.** `سسس` is the same letter three times, and shaped it is three
*different* glyphs: initial, medial and final. Unshaped it is one glyph three
times. Nothing else has to be understood about Arabic to read that result.

Both are properties of the writing systems, not of a particular font, so they
hold for whatever font LibreOffice picks.

## Running it

    python scripts/check_pdf_shaping.py
    python scripts/check_pdf_shaping.py --keep     # leave the PDF to look at

LibreOffice is found through `$ETAMIL_SOFFICE`, then `soffice` on PATH, then the
usual install locations. **Not finding it is not a failure**: it reports that it
skipped and exits 0, the same way `check_site_counts.py` does when there is no
site checkout, so this is a no-op for anyone who has only cloned the compiler.
"""
import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

KA = "\u0b95"  # க
KA_E = "\u0b95\u0bc6"  # கெ
KA_O = "\u0b95\u0bca"  # கொ
SEEN = "\u0633"  # س
SEEN_3 = "\u0633\u0633\u0633"  # سسس

ROWS = [KA, KA_E, KA_O, SEEN, SEEN_3]

FODT = """<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 office:version="1.2" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text>
{rows}
 </office:text></office:body>
</office:document>
"""

CANDIDATES = (
    "C:/Program Files/LibreOffice/program/soffice.exe",
    "C:/Program Files (x86)/LibreOffice/program/soffice.exe",
    "/usr/bin/soffice",
    "/usr/bin/libreoffice",
    "/Applications/LibreOffice.app/Contents/MacOS/soffice",
)


def find_soffice():
    stated = os.environ.get("ETAMIL_SOFFICE")
    if stated:
        return stated if Path(stated).exists() else None
    found = shutil.which("soffice") or shutil.which("libreoffice")
    if found:
        return found
    for candidate in CANDIDATES:
        if Path(candidate).exists():
            return candidate
    return None


def glyph_runs(pdf: Path):
    """Every text-showing operator in the page, as a list of glyph ids.

    The content stream is Flate-compressed. Rather than walking the object
    graph, every stream in the file is inflated and the ones that decompress to
    something with `Tj` or `TJ` in it are read -- which is enough here and has
    no dependency outside the standard library.
    """
    raw = pdf.read_bytes()
    runs = []
    for match in re.finditer(rb"stream\r?\n", raw):
        start = match.end()
        end = raw.find(b"endstream", start)
        try:
            body = zlib.decompress(raw[start:end]).decode("latin-1")
        except zlib.error:
            continue
        for line in body.splitlines():
            if not (line.endswith("Tj") or line.endswith("TJ")):
                continue
            shown = line[line.find("Tf") + 2 :] if "Tf" in line else line
            ids = re.findall(r"<([0-9A-Fa-f]+)>", shown)
            glyphs = []
            for chunk in ids:
                glyphs += [chunk[i : i + 4].lower() for i in range(0, len(chunk), 4)]
            if glyphs:
                runs.append(glyphs)
    return runs


def judge(runs):
    """The two questions, over glyph runs -- separated from the rendering so
    `--self-test` can put wrong answers in front of them. A check nothing ever
    fails is a check that has not been shown to be able to fail."""
    ka, ka_e, ka_o, _seen, seen_3 = runs
    failures = []

    # Tamil: ொ splits around the consonant, so கொ starts with the same glyph as
    # கெ. Unshaped, it would start with the glyph for க.
    if ka_o[0] != ka_e[0]:
        failures.append(
            f"கொ starts with {ka_o[0]} and கெ with {ka_e[0]}; they should be the same glyph, "
            "the left half of the vowel sign"
        )
    if ka_o[0] == ka[0]:
        failures.append(
            f"கொ starts with {ka_o[0]}, which is the glyph for க on its own — "
            "the codepoints were drawn in the order they were written, unshaped"
        )
    if len(ka_o) != 3:
        failures.append(
            f"கொ is {len(ka_o)} glyphs; two codepoints should render as three, "
            "the vowel sign having split in two"
        )

    # Arabic: the same letter three times, in three positions, is three glyphs.
    if len(set(seen_3)) != 3:
        failures.append(
            f"سسس is {' '.join(seen_3)}: the same letter three times should be three "
            "different glyphs — initial, medial and final"
        )
    return failures


def self_test():
    """Shaped output passes and unshaped output does not."""
    shaped = [["01"], ["02", "01"], ["02", "01", "03"], ["01"], ["02", "03", "04"]]
    if judge(shaped):
        print(f"self-test: shaped output was rejected: {judge(shaped)}")
        return 1

    # What a run with no shaping looks like: codepoints in the order written,
    # one glyph each, and the same letter always the same glyph.
    #
    # Three of the four complaints, not all four -- and which one stays quiet is
    # worth knowing. "கொ and கெ start with the same glyph" holds for unshaped
    # output too, because there they both start with க. It is a necessary
    # condition and not a sufficient one; the check that tells the two apart is
    # the next one, that the first glyph is *not* க standing alone.
    unshaped = [["01"], ["01", "02"], ["01", "03"], ["01"], ["01", "01", "01"]]
    missed = judge(unshaped)
    if len(missed) != 3:
        print(f"self-test: unshaped output raised {len(missed)} of 3 complaints: {missed}")
        return 1

    print("self-test: shaped output passes, unshaped output is caught three ways")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--keep", action="store_true", help="leave the PDF behind")
    parser.add_argument("--self-test", action="store_true", help="check the check")
    options = parser.parse_args()

    if options.self_test:
        return self_test()

    soffice = find_soffice()
    if not soffice:
        print("skipped: no LibreOffice found (set ETAMIL_SOFFICE to point at one)")
        return 0

    work = Path(tempfile.mkdtemp(prefix="etamil-pdf-"))
    source = work / "shaping.fodt"
    source.write_text(
        FODT.format(rows="\n".join(f"  <text:p>{row}</text:p>" for row in ROWS)),
        encoding="utf-8",
    )

    result = subprocess.run(
        [soffice, "--headless", "--convert-to", "pdf", "--outdir", str(work), str(source)],
        capture_output=True,
        text=True,
        timeout=300,
        check=False,
    )
    pdf = work / "shaping.pdf"
    if not pdf.exists():
        print(f"LibreOffice produced no PDF: {result.stdout}{result.stderr}")
        return 1

    runs = glyph_runs(pdf)
    if options.keep:
        print(f"the PDF is at {pdf}")

    for row, run in zip(ROWS, runs):
        print(f"  {row!r:<12} -> {' '.join(run)}")

    if len(runs) != len(ROWS):
        print(
            f"\nexpected {len(ROWS)} runs of glyphs, one per line, and found {len(runs)}."
            "\nA line split across fonts usually means the font LibreOffice picked does"
            "\nnot cover that script, which this cannot tell apart from a shaping change."
        )
        return 1

    failures = judge(runs)

    if failures:
        print("\nPDF output is not shaped:")
        for failure in failures:
            print(f"  - {failure}")
        return 1

    print("\nTamil reorders and Arabic joins: the PDF path shapes both")
    if not options.keep:
        shutil.rmtree(work, ignore_errors=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
