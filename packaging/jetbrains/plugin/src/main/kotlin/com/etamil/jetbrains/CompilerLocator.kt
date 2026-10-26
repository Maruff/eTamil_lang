// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import java.nio.file.Path

/**
 * Where the `etamil` compiler is: `ETAMIL_BIN` (the variable the project's scripts
 * already use for it), then the `PATH`, then the installer folders.
 */
object CompilerLocator {
    const val ENV_VAR = "ETAMIL_BIN"

    fun binaryName(windows: Boolean) = if (windows) "etamil.exe" else "etamil"

    /** The full path to the compiler, or `null` if it is nowhere we know to look. */
    fun locate(
        env: Map<String, String>,
        windows: Boolean,
        home: Path,
        exists: (Path) -> Boolean,
    ): String? = BinaryLocator.locate(ENV_VAR, binaryName(windows), env, windows, home, exists)
}
