#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
"""Build VS Code extension packages that carry the compiler.

Installing the extension used to be half an installation. The other half was a
release page, a download, an archive, an install script and a PATH edit — five
steps between someone deciding to try eTamil and seeing a program run, each one
a place to stop. The extension now carries the `etamil` binary and the nUlakam
standard library inside the VSIX, so the half is the whole.

That means one VSIX per platform, which is what `vsce package --target` is for.
A user installing from the Marketplace gets the build for their machine and
never sees the others; a user installing a `.vsix` by hand picks the one whose
name says their platform.

## The layout

    eTamil_Code/bin/<platform>-<arch>/etamil[.exe]     one per target
    eTamil_Code/runtime/nUlakam/**                     shared
    eTamil_Code/runtime/examples/**                    shared

`src/bundle.ts` computes those paths and `src/toolchain.ts` reads them, keyed
on `process.platform` and `process.arch` — the same pair VS Code's own target
names are built from, which is why the directory is named that way rather than
after the target string.

The library is not repeated per target. It is eTamil source, identical on every
platform, and `ETAMIL_PATH` is what points the compiler at it.

It is carried even where the compiler has the library built in, because Go to
Definition opens a file and a copy compiled into an executable is not one.

Both directories are git-ignored. They are build output, and the source of both
is already in this repository.

## Where the binaries come from

    --binary <path>       a file you built, for the current platform
    --from-dist           the packages ./packaging/build.sh left in dist/
    --from-release <tag>  downloaded from a GitHub release

A release build uses `--from-release`, because those archives are the artifacts
that were tested, and because no one machine can build all four targets.

## Usage

    python scripts/package_extension.py --stage                # current platform only
    python scripts/package_extension.py --from-dist            # stage and package
    python scripts/package_extension.py --from-release v1.0.0  # every target

`--stage` stops after copying the files in, which is what you want when you are
about to press F5 in the extension host: the carried compiler is then the one
your debug session uses.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXT = ROOT / "eTamil_Code"
BIN = EXT / "bin"
RUNTIME = EXT / "runtime"
RELEASES = "https://github.com/Maruff/etamil_compiler/releases"

# VS Code's `--target` name, the `bin/` directory it maps to, and the release
# asset it is built from. The middle column is what `src/bundle.ts` computes
# from `process.platform` and `process.arch`, so these two have to agree; the
# test in eTamil_Code/test/bundle.test.js asserts the same pairs from the other
# side.
TARGETS = {
    "win32-x64": ("win32-x64", "etamil-windows-x64.zip"),
    "linux-x64": ("linux-x64", "etamil-linux-x64.tar.gz"),
    "linux-arm64": ("linux-arm64", "etamil-linux-arm64.tar.gz"),
    "darwin-x64": ("darwin-x64", "etamil-macos-x64.tar.gz"),
    "darwin-arm64": ("darwin-arm64", "etamil-macos-arm64.tar.gz"),
}


def remove_tree(directory: Path) -> None:
    """`shutil.rmtree`, on Windows too.

    `copytree` copies permission bits, so a second staging run meets whatever
    the first one wrote — and on Windows a read-only bit makes `os.rmdir` fail
    with "Access is denied" rather than with anything that names the cause.
    Clearing the bit and retrying is the standard answer, and doing it only in
    the error path keeps the common case a plain rmtree.
    """
    def retry(function, path, _excinfo):
        os.chmod(path, stat.S_IWRITE)
        function(path)

    if directory.exists():
        shutil.rmtree(directory, onexc=retry)


def die(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def here() -> str:
    """This machine's `bin/` key, in the spelling node reports."""
    platform = {"win32": "win32", "darwin": "darwin"}.get(sys.platform, "linux")
    machine = os.uname().machine.lower() if hasattr(os, "uname") else "x86_64"
    arch = "arm64" if machine in ("arm64", "aarch64") else "x64"
    if sys.platform == "win32":
        arch = "arm64" if os.environ.get("PROCESSOR_ARCHITECTURE") == "ARM64" else "x64"
    return f"{platform}-{arch}"


# ---------------------------------------------------------------------------
# Staging
# ---------------------------------------------------------------------------


def stage_library() -> None:
    """Copy nUlakam in, whole.

    The obvious economy here is to leave out `*_cOqaZY.qmz`, each module's own
    test file, on the grounds that nobody importing the library wants them.
    They stay, for two reasons. Three of them define a `செயல்` that the
    generated language data records, so dropping them would break Go to
    Definition on exactly those names and on nothing else — the kind of hole
    that is found a year later. And the whole library is a few hundred
    kilobytes of text next to a sixteen-megabyte binary, so the economy buys
    nothing worth having.
    """
    target = RUNTIME / "nUlakam"
    remove_tree(target)
    shutil.copytree(ROOT / "nUlakam", target)
    count = sum(1 for _ in target.rglob("*.qmz"))
    print(f"  runtime/nUlakam  {count} modules")


def stage_examples() -> None:
    """Copy the repository's examples in.

    They are what **eTamil: Open an example** offers. Somebody who has just
    installed the extension has a language they have never seen and an empty
    buffer; twenty-nine programs that run is a better first minute than a blank
    file and a link.
    """
    target = RUNTIME / "examples"
    remove_tree(target)
    shutil.copytree(ROOT / "examples", target)
    count = sum(1 for _ in target.rglob("*.qmz"))
    print(f"  runtime/examples  {count} programs")


def stage_binary(key: str, source: Path) -> None:
    """Put one built compiler under `bin/<key>/`."""
    name = "etamil.exe" if key.startswith("win32") else "etamil"
    directory = BIN / key
    directory.mkdir(parents=True, exist_ok=True)
    destination = directory / name
    shutil.copyfile(source, destination)
    if not key.startswith("win32"):
        destination.chmod(0o755)
    size = destination.stat().st_size / (1024 * 1024)
    print(f"  bin/{key}/{name}  {size:.1f} MB")


def from_dist(key: str) -> Path | None:
    """The binary `packaging/build.sh` left in `dist/`, if it is there."""
    _, asset = TARGETS[key]
    stem = asset.replace(".tar.gz", "").replace(".zip", "")
    candidate = ROOT / "dist" / stem / ("etamil.exe" if key.startswith("win32") else "etamil")
    return candidate if candidate.exists() else None


def from_release(key: str, tag: str, into: Path) -> Path | None:
    """Download a release archive and return the binary inside it."""
    _, asset = TARGETS[key]
    url = f"{RELEASES}/download/{tag}/{asset}"
    archive = into / asset
    print(f"  fetching {url}")
    try:
        with urllib.request.urlopen(url) as response:  # noqa: S310 — a fixed host
            archive.write_bytes(response.read())
    except Exception as error:  # noqa: BLE001 — any failure means "not available"
        print(f"  skipped {key}: {error}")
        return None

    extract = into / key
    extract.mkdir(parents=True, exist_ok=True)
    if asset.endswith(".zip"):
        with zipfile.ZipFile(archive) as zipped:
            zipped.extractall(extract)
    else:
        with tarfile.open(archive) as tarred:
            # `filter="data"` is required from Python 3.14 and is the right
            # answer anyway: it refuses absolute paths, `..` and device files.
            tarred.extractall(extract, filter="data")

    name = "etamil.exe" if key.startswith("win32") else "etamil"
    found = next(extract.rglob(name), None)
    if found is None:
        print(f"  skipped {key}: no {name} in {asset}")
    return found


# ---------------------------------------------------------------------------
# Packaging
# ---------------------------------------------------------------------------


def package(target: str) -> None:
    """Run vsce for one target, from the extension directory."""
    version = json.loads((EXT / "package.json").read_text(encoding="utf-8"))["version"]
    output = EXT / f"etamil-support-{version}-{target}.vsix"
    command = [
        "npx",
        "vsce",
        "package",
        "--target",
        target,
        "--out",
        str(output),
    ]
    print(f"  vsce package --target {target}")
    result = subprocess.run(command, cwd=EXT, shell=(sys.platform == "win32"))
    if result.returncode != 0:
        die(f"vsce failed for {target}")
    print(f"  {output.name}  {output.stat().st_size / (1024 * 1024):.1f} MB")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--stage", action="store_true", help="stage the files and stop, no .vsix"
    )
    parser.add_argument("--binary", type=Path, help="a compiler for this platform")
    parser.add_argument(
        "--from-dist", action="store_true", help="use what packaging/build.sh produced"
    )
    parser.add_argument("--from-release", metavar="TAG", help="download a release")
    parser.add_argument(
        "--clean", action="store_true", help="remove bin/ and runtime/ and stop"
    )
    options = parser.parse_args()

    if options.clean:
        for directory in (BIN, RUNTIME):
            remove_tree(directory)
            print(f"removed {directory.relative_to(ROOT)}")
        return 0

    print("Staging the toolchain into the extension")
    stage_library()
    stage_examples()

    # Outside the extension directory, and outside the repository. It used to
    # be `EXT / ".package-downloads"`, which put four extracted release
    # archives *inside* the thing vsce packages: 576 extra files and 36 MB in
    # every VSIX, on the one code path a real release takes. `--binary` and
    # `--from-dist` never touch it, so it packaged clean every time it was
    # tested by hand.
    scratch = Path(tempfile.mkdtemp(prefix="etamil-vsix-"))
    try:
        sources = collect(options, scratch)
        if not sources:
            die(
                "no compiler to carry. Build one (packaging/build.sh), pass "
                "--binary, or pass --from-release <tag>."
            )
        print(f"found: {', '.join(sorted(sources))}")

        if options.stage:
            # Every one that was found, so pressing F5 in the extension host
            # uses the carried compiler on whatever machine that is.
            remove_tree(BIN)
            for key, source in sorted(sources.items()):
                stage_binary(key, source)
            return 0

        # One target at a time, with bin/ emptied in between. Packaging them
        # all together would put four binaries in every VSIX — a Windows user
        # downloading three compilers that cannot run on their machine, and
        # the reason vsce grew --target in the first place.
        for target, (key, _) in TARGETS.items():
            if key not in sources:
                continue
            remove_tree(BIN)
            stage_binary(key, sources[key])
            package(target)
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
    return 0


def collect(options, scratch: Path) -> dict[str, Path]:
    """Every compiler available to carry, keyed by `bin/` directory."""
    if options.binary:
        if not options.binary.exists():
            die(f"{options.binary} does not exist")
        return {here(): options.binary}

    if options.from_release:
        found = {}
        for key in TARGETS:
            binary = from_release(key, options.from_release, scratch)
            if binary:
                found[key] = binary
        return found

    # --from-dist, and the default: whatever is already built locally.
    found = {key: binary for key in TARGETS if (binary := from_dist(key))}
    if not found:
        built = ROOT / "etamil_compiler" / "target" / "release" / (
            "etamil.exe" if sys.platform == "win32" else "etamil"
        )
        if built.exists():
            found[here()] = built
    return found


if __name__ == "__main__":
    raise SystemExit(main())
