#!/usr/bin/env python3
"""Write the index files of a flat apt repository for the .deb files in a directory.

    python3 packaging/make-apt-index.py dist/ out/

`out/` gets `Packages`, `Packages.gz` and an unsigned `Release`. Signing is a separate
step (see the `apt` job in .github/workflows/release.yml), because it needs a key this
script should not know about.

A flat repository has no pool/ or dists/ tree: the packages and their index sit in one
directory, and a source line ends in `./`. That is what lets the index files be
attached to a GitHub release next to the packages, with no server of our own:

    deb [signed-by=/usr/share/keyrings/etamil.gpg] \\
        https://github.com/Maruff/eTamil_lang/releases/latest/download ./

Written in Python, with no dependency on dpkg or apt-utils, so it runs anywhere the
release workflow does and in the tests on any machine. It reads each package's control
file itself, and understands the gzip, xz and bzip2 control archives (build-deb.sh asks
for xz). A package built with zstd is refused with a message rather than skipped.

SPDX-License-Identifier: AGPL-3.0-or-later
Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
"""

import argparse
import datetime
import gzip
import hashlib
import io
import sys
import tarfile
from pathlib import Path

HASHES = (("MD5sum", "md5"), ("SHA1", "sha1"), ("SHA256", "sha256"))


def read_control(deb: bytes, name: str) -> str:
    """The control file of a .deb: an `ar` archive holding a `control.tar.*` member."""
    if not deb.startswith(b"!<arch>\n"):
        raise ValueError(f"{name}: not a .deb (no ar header)")
    pos = 8
    while pos + 60 <= len(deb):
        header = deb[pos : pos + 60]
        member = header[:16].decode("ascii").strip().rstrip("/")
        size = int(header[48:58].decode("ascii").strip())
        data = deb[pos + 60 : pos + 60 + size]
        pos += 60 + size + (size & 1)  # members are padded to an even length
        if not member.startswith("control.tar"):
            continue
        if member.endswith(".zst"):
            raise ValueError(f"{name}: control archive is zstd; build it with dpkg-deb -Zxz")
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:*") as tar:
            for entry in tar:
                if entry.isfile() and entry.name.lstrip("./") == "control":
                    return tar.extractfile(entry).read().decode("utf-8")
        raise ValueError(f"{name}: control archive has no control file")
    raise ValueError(f"{name}: no control archive")


def field(control: str, key: str) -> str:
    prefix = key + ":"
    for line in control.splitlines():
        if line.startswith(prefix):
            return line[len(prefix) :].strip()
    raise ValueError(f"control file has no {key}")


def stanza(path: Path) -> tuple[tuple[str, str, str], str]:
    data = path.read_bytes()
    control = read_control(data, path.name).strip("\n")
    lines = [control]
    # Relative to the index, which is how a flat repository names its files.
    lines.append(f"Filename: ./{path.name}")
    lines.append(f"Size: {len(data)}")
    for label, algorithm in HASHES:
        lines.append(f"{label}: {hashlib.new(algorithm, data).hexdigest()}")
    key = (field(control, "Package"), field(control, "Version"), field(control, "Architecture"))
    return key, "\n".join(lines) + "\n"


def release(packages: bytes, packages_gz: bytes, architectures: list[str], date: datetime.datetime) -> str:
    out = [
        "Origin: eTamil",
        "Label: eTamil",
        "Description: eTamil compiler packages",
        "Architectures: " + " ".join(architectures),
        "Date: " + date.strftime("%a, %d %b %Y %H:%M:%S UTC"),
    ]
    for label, algorithm in HASHES:
        out.append(f"{label}:")
        for name, blob in (("Packages", packages), ("Packages.gz", packages_gz)):
            out.append(f" {hashlib.new(algorithm, blob).hexdigest()} {len(blob):>16} {name}")
    return "\n".join(out) + "\n"


def build(deb_dir: Path, out_dir: Path, date: datetime.datetime) -> list[str]:
    debs = sorted(deb_dir.glob("*.deb"))
    if not debs:
        raise SystemExit(f"error: no .deb files in {deb_dir}")
    stanzas = sorted(stanza(p) for p in debs)
    # Stanzas are separated by a blank line, and the file ends with one.
    packages = "\n".join(text for _, text in stanzas).encode("utf-8") + b"\n"
    # mtime 0 and no filename, so the same packages give the same bytes.
    packages_gz = gzip.compress(packages, compresslevel=9, mtime=0)
    architectures = sorted({key[2] for key, _ in stanzas})

    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "Packages").write_bytes(packages)
    (out_dir / "Packages.gz").write_bytes(packages_gz)
    (out_dir / "Release").write_text(release(packages, packages_gz, architectures, date), encoding="utf-8", newline="\n")
    return [f"{key[0]} {key[1]} {key[2]}" for key, _ in stanzas]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("deb_dir", type=Path, help="directory with the .deb files")
    parser.add_argument("out_dir", type=Path, help="where Packages, Packages.gz and Release go")
    parser.add_argument("--date", help="Release date, ISO 8601 UTC (default: now); for reproducible output")
    args = parser.parse_args()
    date = (
        datetime.datetime.fromisoformat(args.date).replace(tzinfo=None)
        if args.date
        else datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None)
    )
    for line in build(args.deb_dir, args.out_dir, date):
        print("indexed", line)


if __name__ == "__main__":
    sys.exit(main())
