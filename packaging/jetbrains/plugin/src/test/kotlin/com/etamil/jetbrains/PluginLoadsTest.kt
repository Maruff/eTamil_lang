// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import com.intellij.testFramework.fixtures.BasePlatformTestCase
import org.jetbrains.plugins.textmate.TextMateService
import org.jetbrains.plugins.textmate.api.TextMateBundleProvider

/**
 * Starts a headless IDE with this plugin, TextMate and LSP4IJ loaded, so a typo
 * in plugin.xml or a wrong class name fails here rather than on a user's machine.
 *
 * What this cannot show is TextMate's own startup registration of bundles: a light
 * test does not run it (its file-name table is empty, built-in languages included),
 * so these tests stop at "TextMate reads the bundle and finds `.qmz` in it".
 */
class PluginLoadsTest : BasePlatformTestCase() {
    fun testTheBundleProviderIsRegisteredWithTextMate() {
        val providers = TextMateBundleProvider.EP_NAME.extensionList
        assertTrue(
            "the eTamil provider is registered: $providers",
            providers.any { it is ETamilBundleProvider },
        )
    }

    fun testTheProviderWritesAnImportableBundle() {
        val bundle = ETamilBundleProvider().getBundles().single()
        assertEquals("eTamil", bundle.name)
        for (file in ETamilBundleProvider.FILES) {
            assertTrue("$file was written", bundle.path.resolve(file).toFile().isFile)
        }
    }

    fun testTextMateReadsTheBundleAndFindsQmzInIt() {
        val bundle = ETamilBundleProvider().getBundles().single()
        val reader = TextMateService.getInstance().readBundle(bundle.path)
        assertNotNull("TextMate can read the bundle's manifest", reader)

        val grammars = reader!!.readGrammars().toList()
        assertEquals("one grammar", 1, grammars.size)
        assertTrue(
            "the grammar claims .qmz files: ${grammars.single().fileNameMatchers}",
            grammars.single().fileNameMatchers.any { it.toString().contains("qmz") },
        )
    }
}
