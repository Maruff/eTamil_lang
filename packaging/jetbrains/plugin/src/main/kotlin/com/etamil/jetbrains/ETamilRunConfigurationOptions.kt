// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import com.intellij.execution.configurations.LocatableRunConfigurationOptions

/** What a run configuration remembers, saved into the project's `.idea/runConfigurations`. */
class ETamilRunConfigurationOptions : LocatableRunConfigurationOptions() {
    var scriptPath by string("")
    var mode by enum(RunMode.RUN)
    var port by property(CompilerArguments.DEFAULT_PORT)
    var workingDirectory by string("")

    /** An explicit compiler, for when `ETAMIL_BIN` and the `PATH` are not what this run wants. */
    var compilerPath by string("")
    var env by map<String, String>()
    var passParentEnvs by property(true)
}
