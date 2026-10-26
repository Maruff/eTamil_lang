#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
"""Build the eTamil language-support VSIX for Visual Studio.

The extension carries no code: a .pkgdef registers the TextMate grammar and the
language configuration, and Visual Studio does the rest (highlighting, comment
toggling, bracket matching and pairing). A VSIX is a zip, so this needs neither
MSBuild nor the Visual Studio SDK, and it runs anywhere.

The grammar and configuration are the ones in ../jetbrains/etamil-textmate, the
single copy that sync-bundle.sh keeps current, so there is one place to update.

    python packaging/visualstudio/build-vsix.py            # writes dist/etamil-language-1.0.0.vsix
    python packaging/visualstudio/build-vsix.py --check    # validate the layout, write nothing
"""

import json
import sys
import zipfile
from pathlib import Path
from xml.etree import ElementTree

HERE = Path(__file__).resolve().parent
BUNDLE = HERE.parent / "jetbrains" / "etamil-textmate"
DIST = HERE / "dist"

VERSION = "1.0.0"
IDENTITY = "etamil.language"
SCOPE = "source.etamil"

VSIX_NS = "http://schemas.microsoft.com/developer/vsx-schema/2011"

# Community, Professional and Enterprise, 2022 (17.x) and later.
EDITIONS = ["Community", "Pro", "Enterprise"]

MANIFEST = f"""<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="{VSIX_NS}">
  <Metadata>
    <Identity Id="{IDENTITY}" Version="{VERSION}" Language="en-US" Publisher="eTamil" />
    <DisplayName>eTamil Language Support</DisplayName>
    <Description xml:space="preserve">Syntax highlighting, comment toggling and bracket matching for eTamil (.qmz) files. Add the eTamil language server extension for diagnostics, completion, hover and go to definition.</Description>
    <MoreInfo>https://etamil.in</MoreInfo>
    <Tags>eTamil, Tamil, FinTech, syntax highlighting</Tags>
  </Metadata>
  <Installation>
{chr(10).join(f'    <InstallationTarget Id="Microsoft.VisualStudio.{e}" Version="[17.0,)"><ProductArchitecture>amd64</ProductArchitecture></InstallationTarget>' for e in EDITIONS)}
  </Installation>
  <Dependencies />
  <Prerequisites>
    <Prerequisite Id="Microsoft.VisualStudio.Component.CoreEditor" Version="[17.0,)" DisplayName="Visual Studio core editor" />
  </Prerequisites>
  <Assets>
    <Asset Type="Microsoft.VisualStudio.VsPackage" Path="etamil.pkgdef" />
  </Assets>
</PackageManifest>
"""

CONTENT_TYPES = """<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="vsixmanifest" ContentType="text/xml" />
  <Default Extension="pkgdef" ContentType="text/plain" />
  <Default Extension="json" ContentType="application/json" />
</Types>
"""

# Folder "Grammars" holds the grammar; the language configuration is keyed by
# the grammar's scope name (the value of scopeName in the grammar itself).
PKGDEF = f"""[$RootKey$\\TextMate\\Repositories]
"etamil"="$PackageFolder$\\Grammars"

[$RootKey$\\TextMate\\LanguageConfiguration\\GrammarMapping]
"{SCOPE}"="$PackageFolder$\\language-configuration.json"
"""

GRAMMAR = BUNDLE / "syntaxes" / "etamil.tmLanguage.json"
LANGUAGE_CONFIGURATION = BUNDLE / "language-configuration.json"


def files() -> dict[str, bytes]:
    """Every file in the package, by its path inside the zip."""
    return {
        "[Content_Types].xml": CONTENT_TYPES.encode("utf-8"),
        "extension.vsixmanifest": MANIFEST.encode("utf-8"),
        "etamil.pkgdef": PKGDEF.encode("utf-8"),
        "language-configuration.json": LANGUAGE_CONFIGURATION.read_bytes(),
        "Grammars/etamil.tmLanguage.json": GRAMMAR.read_bytes(),
    }


def validate(package: dict[str, bytes]) -> list[str]:
    """What is wrong with the layout, if anything."""
    problems: list[str] = []

    manifest = ElementTree.fromstring(package["extension.vsixmanifest"])
    ns = {"v": VSIX_NS}
    assets = manifest.findall(".//v:Asset", ns)
    if not assets:
        problems.append("the manifest declares no assets")
    for asset in assets:
        path = asset.get("Path")
        if path not in package:
            problems.append(f"the manifest names {path}, which is not in the package")

    ElementTree.fromstring(package["[Content_Types].xml"])

    grammar = json.loads(package["Grammars/etamil.tmLanguage.json"])
    if grammar.get("scopeName") != SCOPE:
        problems.append(f"the grammar's scopeName is {grammar.get('scopeName')!r}, not {SCOPE!r}")
    if "qmz" not in grammar.get("fileTypes", []):
        problems.append("the grammar does not claim .qmz files in fileTypes")

    json.loads(package["language-configuration.json"])
    if not package["etamil.pkgdef"].decode("utf-8").count(SCOPE) == 1:
        problems.append("the pkgdef must map the grammar's scope name exactly once")
    if not package["language-configuration.json"].decode("utf-8").strip().endswith("}"):
        problems.append("the language configuration looks truncated")

    return problems


def main(argv: list[str]) -> int:
    package = files()
    problems = validate(package)
    for problem in problems:
        print(f"error: {problem}", file=sys.stderr)
    if problems:
        return 1
    if "--check" in argv:
        print(f"layout ok: {len(package)} files")
        return 0

    DIST.mkdir(exist_ok=True)
    target = DIST / f"etamil-language-{VERSION}.vsix"
    with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED) as archive:
        # The content-types part first, as a package reader expects.
        for name in ["[Content_Types].xml", *[n for n in package if n != "[Content_Types].xml"]]:
            archive.writestr(name, package[name])
    print(f"{target}  ({target.stat().st_size} bytes, {len(package)} files)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
