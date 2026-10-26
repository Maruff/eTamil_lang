// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

/** What a run configuration asks the compiler to do with the file. */
enum class RunMode(val label: String) {
    RUN("Run"),
    CHECK("Check only (never runs the program)"),
    SERVER("Run as an HTTP server"),
    ;

    override fun toString() = label
}

/** The compiler's command line for a run: pure, so it is tested without an IDE. */
object CompilerArguments {
    const val DEFAULT_PORT = 8080

    fun build(mode: RunMode, scriptPath: String, port: Int): List<String> = when (mode) {
        RunMode.RUN -> listOf("--vm", scriptPath)
        RunMode.CHECK -> listOf("--check", scriptPath)
        RunMode.SERVER -> listOf("--server", "--port", port.toString(), scriptPath)
    }
}
