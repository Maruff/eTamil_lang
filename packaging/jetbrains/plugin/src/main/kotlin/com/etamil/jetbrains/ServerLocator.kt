// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import java.nio.file.Path

/** Where the `etamil-lsp` binary is: `ETAMIL_LSP`, then the `PATH`, then the installer folders. */
object ServerLocator {
    const val ENV_VAR = "ETAMIL_LSP"

    fun binaryName(windows: Boolean) = if (windows) "etamil-lsp.exe" else "etamil-lsp"

    /** The full path to the server, or `null` if it is nowhere we know to look. */
    fun locate(
        env: Map<String, String>,
        windows: Boolean,
        home: Path,
        exists: (Path) -> Boolean,
    ): String? = BinaryLocator.locate(ENV_VAR, binaryName(windows), env, windows, home, exists)
}
