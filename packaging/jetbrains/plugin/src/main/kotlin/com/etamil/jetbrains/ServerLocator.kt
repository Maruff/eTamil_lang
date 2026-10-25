// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import java.nio.file.Path

/**
 * Where the `etamil-lsp` binary is.
 *
 * Pure, with the file system and environment passed in, so the order can be
 * tested without an IDE. The order is: the `ETAMIL_LSP` environment variable
 * (an explicit choice always wins), then the `PATH`, then the folders the eTamil
 * installers use.
 */
object ServerLocator {
    const val ENV_VAR = "ETAMIL_LSP"

    fun binaryName(windows: Boolean) = if (windows) "etamil-lsp.exe" else "etamil-lsp"

    /** The full path to the server, or `null` if it is nowhere we know to look. */
    fun locate(
        env: Map<String, String>,
        windows: Boolean,
        home: Path,
        exists: (Path) -> Boolean,
    ): String? {
        env[ENV_VAR]?.trim()?.takeIf { it.isNotEmpty() }?.let { return it }

        val name = binaryName(windows)
        val separator = if (windows) ';' else ':'

        // Windows spells the variable "Path" in some environments.
        val pathValue = env["PATH"] ?: env["Path"] ?: ""
        for (directory in pathValue.split(separator).filter { it.isNotEmpty() }) {
            val candidate = Path.of(directory).resolve(name)
            if (exists(candidate)) return candidate.toString()
        }

        val installDirectories = buildList {
            add(home.resolve(".local").resolve("bin"))
            if (windows) {
                env["LOCALAPPDATA"]?.let { add(Path.of(it).resolve("Programs").resolve("eTamil")) }
            }
        }
        for (directory in installDirectories) {
            val candidate = directory.resolve(name)
            if (exists(candidate)) return candidate.toString()
        }
        return null
    }
}
