#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
"""Build the `etamil` and `etamil-lsp` NuGet tool packages from the release archives.

Nothing is compiled here. The archives the release workflow builds (one per
platform, see packaging/build.sh) are unpacked, the binaries are laid out under
staging/, and `dotnet pack` wraps them in a small launcher.

    python packaging/nuget/build-nuget.py                       # needs all five archives in dist/
    python packaging/nuget/build-nuget.py --allow-partial       # whichever archives are there (local testing)
    python packaging/nuget/build-nuget.py --dist D:/x --out D:/y

The package version is the compiler's own, from etamil_compiler/Cargo.toml, so a
`dotnet tool install` of version V is the binary released as v-V.
"""

import argparse
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent

# release archive -> .NET runtime identifier
ARCHIVES = {
    "etamil-windows-x64.zip": "win-x64",
    "etamil-linux-x64.tar.gz": "linux-x64",
    "etamil-linux-arm64.tar.gz": "linux-arm64",
    "etamil-macos-x64.tar.gz": "osx-x64",
    "etamil-macos-arm64.tar.gz": "osx-arm64",
}

PROJECTS = [
    ("etamil/Etamil.Tool.csproj", "etamil"),
    ("etamil-lsp/EtamilLsp.Tool.csproj", "etamil-lsp"),
]


def compiler_version() -> str:
    for line in (ROOT / "etamil_compiler" / "Cargo.toml").read_text(encoding="utf-8").splitlines():
        if line.startswith("version"):
            return line.split('"')[1]
    sys.exit("error: no version in etamil_compiler/Cargo.toml")


def unpack(archive: Path, into: Path) -> Path:
    """Unpack a release archive and return the folder inside it."""
    if archive.name.endswith(".zip"):
        with zipfile.ZipFile(archive) as zipped:
            zipped.extractall(into)
    else:
        with tarfile.open(archive) as tarred:
            tarred.extractall(into, filter="data")
    folders = [p for p in into.iterdir() if p.is_dir()]
    if len(folders) != 1:
        sys.exit(f"error: {archive.name} should hold one folder, found {[p.name for p in folders]}")
    return folders[0]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--dist", type=Path, default=ROOT / "dist", help="folder holding the release archives")
    parser.add_argument("--out", type=Path, default=HERE / "nupkgs", help="where the .nupkg files go")
    parser.add_argument("--allow-partial", action="store_true", help="build with only the archives that exist")
    args = parser.parse_args()

    found = {name: rid for name, rid in ARCHIVES.items() if (args.dist / name).is_file()}
    missing = sorted(set(ARCHIVES) - set(found))
    if missing and not args.allow_partial:
        sys.exit(f"error: missing in {args.dist}: {', '.join(missing)} (use --allow-partial to build anyway)")
    if not found:
        sys.exit(f"error: no release archives in {args.dist}")

    version = compiler_version()
    staging = HERE / "staging"
    shutil.rmtree(staging, ignore_errors=True)
    library_done = False

    for name, rid in found.items():
        with tempfile.TemporaryDirectory() as work:
            package = unpack(args.dist / name, Path(work))
            exe_suffix = ".exe" if rid.startswith("win") else ""

            compiler = package / f"etamil{exe_suffix}"
            if not compiler.is_file():
                sys.exit(f"error: {name} has no {compiler.name}")
            target = staging / "etamil" / "bin" / rid
            target.mkdir(parents=True)
            shutil.copy2(compiler, target / compiler.name)

            server = package / f"etamil-lsp{exe_suffix}"
            if server.is_file():
                target = staging / "etamil-lsp" / "bin" / rid
                target.mkdir(parents=True)
                shutil.copy2(server, target / server.name)
            else:
                print(f"note: {name} carries no etamil-lsp (an archive from before the server existed)")

            # The standard library is the same in every archive; keep one copy.
            if not library_done:
                shutil.copytree(package / "nUlakam", staging / "etamil" / "lib" / "nUlakam")
                library_done = True

    args.out.mkdir(parents=True, exist_ok=True)
    for _, folder in PROJECTS:  # a stale package of the same version must not be installed by mistake
        (args.out / f"{folder}.{version}.nupkg").unlink(missing_ok=True)
    for project, folder in PROJECTS:
        if not (staging / folder).is_dir():
            print(f"skipping {folder}: no binaries staged for it")
            continue
        subprocess.run(
            ["dotnet", "pack", str(HERE / project), "-c", "Release", f"-p:Version={version}",
             "-o", str(args.out), "-nodeReuse:false", "--nologo"],
            check=True,
        )

    print()
    for package in sorted(args.out.glob(f"*.{version}.nupkg")):
        print(f"{package}  ({package.stat().st_size / 1e6:.1f} MB)")
    print(f"platforms: {', '.join(found.values())}" + (f"  (missing: {', '.join(missing)})" if missing else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
