// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>

import org.jetbrains.intellij.platform.gradle.TestFrameworkType

plugins {
    kotlin("jvm") version "2.0.21"
    id("org.jetbrains.intellij.platform") version "2.19.0"
}

group = "in.etamil"
version = "0.1.0"

repositories {
    mavenCentral()
    intellijPlatform {
        defaultRepositories()
    }
}

dependencies {
    // Plain unit tests of the pure logic; nothing here needs a running IDE.
    testImplementation("junit:junit:4.13.2")
    testRuntimeOnly("org.opentest4j:opentest4j:1.3.0")

    intellijPlatform {
        // 2024.2 is the oldest platform LSP4IJ 0.21 supports (build 242), so the
        // plugin compiles against the floor and runs on everything after it.
        intellijIdeaCommunity("2024.2.5")
        bundledPlugin("org.jetbrains.plugins.textmate")
        plugin("com.redhat.devtools.lsp4ij", "0.21.0")
        testFramework(TestFrameworkType.Platform)
        pluginVerifier()
        zipSigner()
    }
}

kotlin {
    jvmToolchain(21)
}

// One copy of the grammar: the bundle that JetBrains users can import by hand is
// the same one this plugin carries, so it is copied in at build time rather than
// kept twice.
tasks.processResources {
    from("../etamil-textmate") {
        into("etamil-textmate")
    }
}

intellijPlatform {
    pluginConfiguration {
        name = "eTamil"
        ideaVersion {
            sinceBuild = "242"
        }
    }
    pluginVerification {
        ides {
            // The platform this is built against, already downloaded. Add newer ones
            // (create(...), latest(), or recommended()) before a release.
            current()
        }
    }
}
