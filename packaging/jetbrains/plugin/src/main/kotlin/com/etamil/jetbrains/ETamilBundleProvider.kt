// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import com.intellij.openapi.application.PathManager
import org.jetbrains.plugins.textmate.api.TextMateBundleProvider
import java.nio.file.Files
import java.nio.file.Path

/**
 * Registers the eTamil TextMate bundle, which gives `.qmz` files highlighting,
 * comment toggling and bracket pairing.
 *
 * The TextMate plugin reads a bundle from a directory, not from inside a jar, so
 * the copy this plugin carries (see `processResources` in build.gradle.kts) is
 * written out under the IDE's system directory. It is rewritten whenever the
 * bytes differ, so an updated plugin brings its updated grammar.
 */
class ETamilBundleProvider : TextMateBundleProvider {
    override fun getBundles(): List<TextMateBundleProvider.PluginBundle> {
        val target = PathManager.getSystemDir().resolve("etamil-textmate")
        extract(ETamilBundleProvider::class.java.classLoader, target)
        return listOf(TextMateBundleProvider.PluginBundle("eTamil", target))
    }

    companion object {
        /** The files of the bundle, relative to its root. */
        val FILES = listOf(
            "package.json",
            "language-configuration.json",
            "syntaxes/etamil.tmLanguage.json",
        )

        /** Copy the bundle's resources into [target], skipping files that are already identical. */
        fun extract(loader: ClassLoader, target: Path) {
            for (relative in FILES) {
                val bytes = loader.getResourceAsStream("etamil-textmate/$relative")?.use { it.readBytes() }
                    ?: error("the plugin is missing its resource etamil-textmate/$relative")
                val destination = target.resolve(relative)
                if (Files.exists(destination) && Files.readAllBytes(destination).contentEquals(bytes)) {
                    continue
                }
                Files.createDirectories(destination.parent)
                Files.write(destination, bytes)
            }
        }
    }
}
