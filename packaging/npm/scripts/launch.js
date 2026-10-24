// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//
// Runs a binary that scripts/install.js unpacked into vendor/, passing every
// argument through and exiting with its status. Shared by the `etamil` and
// `etamil-lsp` commands.
"use strict";

const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

function launch(name) {
  const vendor = path.join(__dirname, "..", "vendor");
  const exe = path.join(vendor, process.platform === "win32" ? `${name}.exe` : name);

  if (!fs.existsSync(exe)) {
    console.error(`etamil: ${name} is not installed. Run \`npm rebuild etamil\` (install scripts must be enabled).`);
    process.exit(1);
  }

  // Lets  இறக்கு "nUlakam/..."  resolve from any directory, as the installers do.
  const env = { ...process.env };
  if (!env.ETAMIL_PATH) env.ETAMIL_PATH = vendor;

  const result = spawnSync(exe, process.argv.slice(2), { stdio: "inherit", env });
  if (result.error) {
    console.error(`etamil: ${result.error.message}`);
    process.exit(1);
  }
  process.exit(result.status === null ? 1 : result.status);
}

module.exports = { launch };
