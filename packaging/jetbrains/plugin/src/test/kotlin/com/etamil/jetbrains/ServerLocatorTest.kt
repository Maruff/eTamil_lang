// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.nio.file.Path

class ServerLocatorTest {
    private val home = Path.of("/home/me")

    private fun locate(env: Map<String, String>, windows: Boolean = false, vararg existing: Path) =
        ServerLocator.locate(env, windows, home) { it in existing }

    @Test
    fun theBinaryIsNamedForThePlatform() {
        assertEquals("etamil-lsp", ServerLocator.binaryName(windows = false))
        assertEquals("etamil-lsp.exe", ServerLocator.binaryName(windows = true))
    }

    @Test
    fun nothingIsFoundWhenNothingIsThere() {
        assertNull(locate(mapOf("PATH" to "/usr/bin:/usr/local/bin")))
    }

    @Test
    fun theEnvironmentVariableWinsEvenOverThePath() {
        val onPath = Path.of("/usr/bin/etamil-lsp")
        val found = locate(mapOf("ETAMIL_LSP" to "  /opt/mine/etamil-lsp ", "PATH" to "/usr/bin"), false, onPath)
        assertEquals("/opt/mine/etamil-lsp", found)
    }

    @Test
    fun aBlankEnvironmentVariableIsIgnored() {
        val onPath = Path.of("/usr/bin/etamil-lsp")
        val found = locate(mapOf("ETAMIL_LSP" to "   ", "PATH" to "/usr/bin"), false, onPath)
        assertEquals(onPath.toString(), found)
    }

    @Test
    fun thePathIsSearchedInOrderAndEmptyEntriesAreSkipped() {
        val first = Path.of("/a/etamil-lsp")
        val second = Path.of("/b/etamil-lsp")
        assertEquals(first.toString(), locate(mapOf("PATH" to "::/a:/b"), false, first, second))
    }

    @Test
    fun theInstallersFolderIsTheLastResort() {
        val installed = home.resolve(".local").resolve("bin").resolve("etamil-lsp")
        assertEquals(installed.toString(), locate(mapOf("PATH" to "/usr/bin"), false, installed))
    }

    @Test
    fun theWindowsInstallerFolderIsTheLastResort() {
        val installed = Path.of("C:\\Users\\me\\AppData\\Local").resolve("Programs").resolve("eTamil").resolve("etamil-lsp.exe")
        val found = locate(
            mapOf("Path" to "C:\\Windows", "LOCALAPPDATA" to "C:\\Users\\me\\AppData\\Local"),
            true,
            installed,
        )
        assertEquals(installed.toString(), found)
    }

    @Test
    fun windowsSplitsThePathOnSemicolonsAndLooksForTheExe() {
        val exe = Path.of("C:\\tools").resolve("etamil-lsp.exe")
        val found = locate(mapOf("Path" to "C:\\Windows;C:\\tools"), true, exe)
        assertEquals(exe.toString(), found)
    }
}
