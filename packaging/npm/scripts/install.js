// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//
// Downloads the release archive that matches this package's version and this
// machine, checks it against the .sha256 published beside it, and unpacks it
// into vendor/. No dependencies: fetch, crypto and the system tar (Windows 10+
// ships bsdtar, which also reads .zip).
"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const PACKAGES = {
  "win32-x64": "etamil-windows-x64.zip",
  "linux-x64": "etamil-linux-x64.tar.gz",
  "linux-arm64": "etamil-linux-arm64.tar.gz",
  "darwin-x64": "etamil-macos-x64.tar.gz",
  "darwin-arm64": "etamil-macos-arm64.tar.gz",
};

function fail(message) {
  console.error(`etamil: ${message}`);
  process.exit(1);
}

async function get(url) {
  const response = await fetch(url);
  if (!response.ok) fail(`${url} returned ${response.status}`);
  return Buffer.from(await response.arrayBuffer());
}

async function main() {
  if (process.env.ETAMIL_SKIP_DOWNLOAD) return;

  const archive = PACKAGES[`${process.platform}-${process.arch}`];
  if (!archive) {
    fail(`no prebuilt binary for ${process.platform}-${process.arch}. Supported: ${Object.keys(PACKAGES).join(", ")}`);
  }

  const { version } = require("../package.json");
  // The override is for mirrors and offline installs; it must serve the archive
  // and its .sha256 under the same names the GitHub release uses.
  const base = process.env.ETAMIL_RELEASE_BASE || `https://github.com/Maruff/eTamil_lang/releases/download/v${version}`;

  console.log(`etamil: downloading ${archive} (v${version})`);
  const expected = (await get(`${base}/${archive}.sha256`)).toString("utf8").trim().split(/\s+/)[0].toLowerCase();
  const bytes = await get(`${base}/${archive}`);
  const actual = crypto.createHash("sha256").update(bytes).digest("hex");
  if (actual !== expected) fail(`checksum mismatch for ${archive}: expected ${expected}, got ${actual}`);

  const work = fs.mkdtempSync(path.join(os.tmpdir(), "etamil-"));
  try {
    const file = path.join(work, archive);
    fs.writeFileSync(file, bytes);
    // On Windows, name System32's bsdtar explicitly: with Git for Windows
    // installed, a bare "tar" is GNU tar, which reads "C:\..." as a remote host.
    const tar = process.platform === "win32" ? path.join(process.env.SystemRoot || "C:\\Windows", "System32", "tar.exe") : "tar";
    const unpacked = spawnSync(tar, ["-xf", file, "-C", work], { stdio: "inherit" });
    if (unpacked.status !== 0) fail("could not unpack the archive with tar");

    const inner = path.join(work, archive.replace(/\.(tar\.gz|zip)$/, ""));
    const vendor = path.join(__dirname, "..", "vendor");
    fs.rmSync(vendor, { recursive: true, force: true });
    fs.cpSync(inner, vendor, { recursive: true });

    const exe = path.join(vendor, process.platform === "win32" ? "etamil.exe" : "etamil");
    if (process.platform !== "win32") fs.chmodSync(exe, 0o755);
    const check = spawnSync(exe, ["--version"], { encoding: "utf8" });
    if (check.status !== 0) fail(`the installed binary did not run: ${check.stderr || check.error}`);
    console.log(`etamil: installed ${check.stdout.trim()}`);
  } finally {
    fs.rmSync(work, { recursive: true, force: true });
  }
}

main().catch((error) => fail(error.message));
