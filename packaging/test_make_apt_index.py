"""Tests for make-apt-index.py, with .deb files built here: python3 -m unittest packaging/test_make_apt_index.py"""

import datetime
import gzip
import hashlib
import importlib.util
import io
import re
import tarfile
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("make_apt_index", Path(__file__).with_name("make-apt-index.py"))
index = importlib.util.module_from_spec(spec)
spec.loader.exec_module(index)

DATE = datetime.datetime(2026, 10, 4, 12, 0, 0)


def control_text(arch: str, version: str = "1.4.2") -> str:
    return (
        f"Package: etamil\nVersion: {version}\nSection: devel\nPriority: optional\n"
        f"Architecture: {arch}\nInstalled-Size: 5000\n"
        "Maintainer: Mohammed Maruff (Esan Maruff) <esan@etamil.in>\n"
        "Description: eTamil compiler for Tamil FinTech programs\n"
        " A bilingual Tamil/English language and compiler.\n"
        " Second line of the description.\n"
    )


def ar_member(name: str, data: bytes) -> bytes:
    header = f"{name + '/':<16}{0:<12}{0:<6}{0:<6}{'100644':<8}{len(data):<10}`\n".encode("ascii")
    return header + data + (b"\n" if len(data) & 1 else b"")


def make_deb(path: Path, arch: str, version: str = "1.4.2", compression: str = "gz", control_name: str = "./control") -> None:
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:" + compression) as tar:
        body = control_text(arch, version).encode("utf-8")
        info = tarfile.TarInfo(control_name)
        info.size = len(body)
        tar.addfile(info, io.BytesIO(body))
    ext = {"gz": "gz", "xz": "xz", "bz2": "bz2"}[compression]
    path.write_bytes(
        b"!<arch>\n" + ar_member("debian-binary", b"2.0\n") + ar_member(f"control.tar.{ext}", buf.getvalue()) + ar_member("data.tar.xz", b"x" * 7)
    )


class IndexTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dist = Path(self.tmp.name) / "dist"
        self.out = Path(self.tmp.name) / "out"
        self.dist.mkdir()

    def test_two_architectures_are_indexed_in_order_with_the_right_hashes(self):
        make_deb(self.dist / "etamil_1.4.2_arm64.deb", "arm64")
        make_deb(self.dist / "etamil_1.4.2_amd64.deb", "amd64")
        index.build(self.dist, self.out, DATE)
        packages = (self.out / "Packages").read_text(encoding="utf-8")
        stanzas = packages.strip("\n").split("\n\n")
        self.assertEqual(len(stanzas), 2)
        self.assertIn("Architecture: amd64", stanzas[0])  # sorted by name, version, architecture
        self.assertIn("Filename: ./etamil_1.4.2_amd64.deb", stanzas[0])
        for stanza in stanzas:
            arch = re.search(r"Architecture: (\w+)", stanza).group(1)
            data = (self.dist / f"etamil_1.4.2_{arch}.deb").read_bytes()
            self.assertIn(f"Size: {len(data)}", stanza)
            self.assertIn(f"SHA256: {hashlib.sha256(data).hexdigest()}", stanza)
            self.assertIn(f"SHA1: {hashlib.sha1(data).hexdigest()}", stanza)
            self.assertIn(f"MD5sum: {hashlib.md5(data).hexdigest()}", stanza)
            self.assertTrue(stanza.startswith("Package: etamil\n"))
            self.assertIn(" A bilingual Tamil/English language and compiler.", stanza, "description continuation kept")

    def test_release_lists_both_index_files_with_their_hashes(self):
        make_deb(self.dist / "etamil_1.4.2_amd64.deb", "amd64")
        make_deb(self.dist / "etamil_1.4.2_arm64.deb", "arm64")
        index.build(self.dist, self.out, DATE)
        text = (self.out / "Release").read_text(encoding="utf-8")
        self.assertIn("Architectures: amd64 arm64", text)
        self.assertIn("Date: Sun, 04 Oct 2026 12:00:00 UTC", text)
        for name in ("Packages", "Packages.gz"):
            blob = (self.out / name).read_bytes()
            self.assertIn(f" {hashlib.sha256(blob).hexdigest()} {len(blob):>16} {name}", text)
            self.assertIn(f" {hashlib.md5(blob).hexdigest()} {len(blob):>16} {name}", text)
        self.assertEqual(gzip.decompress((self.out / "Packages.gz").read_bytes()), (self.out / "Packages").read_bytes())

    def test_same_packages_give_the_same_bytes(self):
        make_deb(self.dist / "etamil_1.4.2_amd64.deb", "amd64")
        index.build(self.dist, self.out, DATE)
        first = {n: (self.out / n).read_bytes() for n in ("Packages", "Packages.gz", "Release")}
        index.build(self.dist, self.out, DATE)
        self.assertEqual(first, {n: (self.out / n).read_bytes() for n in first})

    def test_xz_and_bzip2_control_archives_and_a_bare_control_name(self):
        make_deb(self.dist / "etamil_1.4.2_amd64.deb", "amd64", compression="xz")
        make_deb(self.dist / "etamil_1.4.2_arm64.deb", "arm64", compression="bz2", control_name="control")
        index.build(self.dist, self.out, DATE)
        self.assertEqual((self.out / "Packages").read_text(encoding="utf-8").count("Package: etamil"), 2)

    def test_files_are_unix_text(self):
        make_deb(self.dist / "etamil_1.4.2_amd64.deb", "amd64")
        index.build(self.dist, self.out, DATE)
        for name in ("Packages", "Release"):
            self.assertNotIn(b"\r", (self.out / name).read_bytes(), name)

    def test_bad_input_is_refused_by_name(self):
        with self.assertRaises(SystemExit):
            index.build(self.dist, self.out, DATE)  # no packages at all
        (self.dist / "junk.deb").write_bytes(b"not a package")
        with self.assertRaisesRegex(ValueError, "junk.deb: not a .deb"):
            index.build(self.dist, self.out, DATE)
        (self.dist / "junk.deb").write_bytes(b"!<arch>\n" + ar_member("debian-binary", b"2.0\n") + ar_member("control.tar.zst", b"zz"))
        with self.assertRaisesRegex(ValueError, "zstd"):
            index.build(self.dist, self.out, DATE)


if __name__ == "__main__":
    unittest.main()
