// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import com.intellij.openapi.project.Project
import com.intellij.openapi.util.SystemInfo
import com.redhat.devtools.lsp4ij.LanguageServerFactory
import com.redhat.devtools.lsp4ij.server.ProcessStreamConnectionProvider
import com.redhat.devtools.lsp4ij.server.StreamConnectionProvider
import java.nio.file.Files
import java.nio.file.Path

/** Starts `etamil-lsp` for LSP4IJ. */
class ETamilLanguageServerFactory : LanguageServerFactory {
    override fun createConnectionProvider(project: Project): StreamConnectionProvider =
        ETamilConnectionProvider()
}

/**
 * Runs the server as a child process speaking LSP on its stdin and stdout.
 *
 * If the binary cannot be found the bare name is used, so LSP4IJ's own "cannot
 * start" message names `etamil-lsp` rather than showing an empty command.
 */
class ETamilConnectionProvider : ProcessStreamConnectionProvider(
    listOf(
        ServerLocator.locate(
            env = System.getenv(),
            windows = SystemInfo.isWindows,
            home = Path.of(System.getProperty("user.home")),
            exists = Files::isRegularFile,
        ) ?: ServerLocator.binaryName(SystemInfo.isWindows),
    ),
)
