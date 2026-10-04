// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.nio.file.Path

class CompilerLocatorTest {
    private val home = Path.of("/home/me")

    private fun locate(env: Map<String, String>, windows: Boolean = false, vararg existing: Path) =
        CompilerLocator.locate(env, windows, home) { it in existing }

    @Test
    fun theBinaryIsNamedForThePlatform() {
        assertEquals("etamil", CompilerLocator.binaryName(windows = false))
        assertEquals("etamil.exe", CompilerLocator.binaryName(windows = true))
    }

    @Test
    fun theEnvironmentVariableIsTheOneTheProjectScriptsUse() {
        assertEquals("ETAMIL_BIN", CompilerLocator.ENV_VAR)
        assertEquals("/opt/e/etamil", locate(mapOf("ETAMIL_BIN" to " /opt/e/etamil ")))
    }

    @Test
    fun theLanguageServerVariableDoesNotNameTheCompiler() {
        assertNull(locate(mapOf("ETAMIL_LSP" to "/opt/e/etamil-lsp")))
    }

    @Test
    fun thePathIsSearchedThenTheInstallerFolders() {
        val onPath = Path.of("/usr/local/bin/etamil")
        assertEquals(onPath.toString(), locate(mapOf("PATH" to "/usr/bin:/usr/local/bin"), false, onPath))
        val installed = home.resolve(".local").resolve("bin").resolve("etamil")
        assertEquals(installed.toString(), locate(mapOf("PATH" to "/usr/bin"), false, installed))
        assertNull(locate(mapOf("PATH" to "/usr/bin")))
    }

    @Test
    fun theCompilerAndTheServerAreFoundIndependently() {
        val server = Path.of("/usr/bin/etamil-lsp")
        assertNull(locate(mapOf("PATH" to "/usr/bin"), false, server))
    }
}
