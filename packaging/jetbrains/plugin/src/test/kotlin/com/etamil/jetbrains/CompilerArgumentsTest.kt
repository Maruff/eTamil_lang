// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import org.junit.Assert.assertEquals
import org.junit.Test

class CompilerArgumentsTest {
    @Test
    fun runUsesTheVmAndNamesTheFile() {
        assertEquals(listOf("--vm", "/p/a.qmz"), CompilerArguments.build(RunMode.RUN, "/p/a.qmz", 8080))
    }

    @Test
    fun checkNeverRunsTheProgram() {
        assertEquals(listOf("--check", "/p/a.qmz"), CompilerArguments.build(RunMode.CHECK, "/p/a.qmz", 8080))
    }

    @Test
    fun aServerGetsItsPortAndTheFileLast() {
        assertEquals(
            listOf("--server", "--port", "9090", "/p/a.qmz"),
            CompilerArguments.build(RunMode.SERVER, "/p/a.qmz", 9090),
        )
    }

    @Test
    fun aPathWithSpacesOrTamilIsOneArgument() {
        val path = "/my files/வட்டி கணக்கு.qmz"
        assertEquals(path, CompilerArguments.build(RunMode.RUN, path, 8080).last())
    }

    @Test
    fun everyModeHasALabelForTheForm() {
        for (mode in RunMode.values()) assertEquals(mode.label, mode.toString())
    }
}
