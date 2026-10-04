// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import java.nio.file.Path

/**
 * Where one of the eTamil binaries is.
 *
 * Pure, with the file system and environment passed in, so the order can be
 * tested without an IDE. The order is: an environment variable (an explicit
 * choice always wins), then the `PATH`, then the folders the eTamil installers use.
 */
object BinaryLocator {
    fun locate(
        envVar: String,
        binaryName: String,
        env: Map<String, String>,
        windows: Boolean,
        home: Path,
        exists: (Path) -> Boolean,
    ): String? {
        env[envVar]?.trim()?.takeIf { it.isNotEmpty() }?.let { return it }

        val separator = if (windows) ';' else ':'

        // Windows spells the variable "Path" in some environments.
        val pathValue = env["PATH"] ?: env["Path"] ?: ""
        for (directory in pathValue.split(separator).filter { it.isNotEmpty() }) {
            val candidate = Path.of(directory).resolve(binaryName)
            if (exists(candidate)) return candidate.toString()
        }

        val installDirectories = buildList {
            add(home.resolve(".local").resolve("bin"))
            if (windows) {
                env["LOCALAPPDATA"]?.let { add(Path.of(it).resolve("Programs").resolve("eTamil")) }
            }
        }
        for (directory in installDirectories) {
            val candidate = directory.resolve(binaryName)
            if (exists(candidate)) return candidate.toString()
        }
        return null
    }
}
