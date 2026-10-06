// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import com.intellij.execution.Executor
import com.intellij.execution.configurations.ConfigurationFactory
import com.intellij.execution.configurations.LocatableConfigurationBase
import com.intellij.execution.configurations.RunConfiguration
import com.intellij.execution.configurations.RuntimeConfigurationError
import com.intellij.execution.runners.ExecutionEnvironment
import com.intellij.openapi.options.SettingsEditor
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.SystemInfo
import java.nio.file.Files
import java.nio.file.Path

/** A saved way of running one `.qmz` file. */
class ETamilRunConfiguration(project: Project, factory: ConfigurationFactory, name: String) :
    LocatableConfigurationBase<ETamilRunConfigurationOptions>(project, factory, name) {

    public override fun getOptions(): ETamilRunConfigurationOptions =
        super.getOptions() as ETamilRunConfigurationOptions

    var scriptPath: String
        get() = options.scriptPath.orEmpty()
        set(value) { options.scriptPath = value }

    var mode: RunMode
        get() = options.mode
        set(value) { options.mode = value }

    var port: Int
        get() = options.port
        set(value) { options.port = value }

    var workingDirectory: String
        get() = options.workingDirectory.orEmpty()
        set(value) { options.workingDirectory = value }

    var compilerPath: String
        get() = options.compilerPath.orEmpty()
        set(value) { options.compilerPath = value }

    var env: Map<String, String>
        get() = options.env
        set(value) { options.env = value.toMutableMap() }

    var passParentEnvs: Boolean
        get() = options.passParentEnvs
        set(value) { options.passParentEnvs = value }

    /** The compiler to run: the one named here, else `ETAMIL_BIN`, the `PATH`, the installer folders. */
    fun resolveCompiler(): String? = compilerPath.trim().takeIf { it.isNotEmpty() } ?: CompilerLocator.locate(
        env = System.getenv(),
        windows = SystemInfo.isWindows,
        home = Path.of(System.getProperty("user.home")),
        exists = Files::isRegularFile,
    )

    /** The folder to run in: the one named here, else the folder of the file, so relative paths in it work. */
    fun resolveWorkingDirectory(): String =
        workingDirectory.trim().takeIf { it.isNotEmpty() } ?: Path.of(scriptPath).parent?.toString().orEmpty()

    override fun checkConfiguration() {
        if (scriptPath.isBlank()) throw RuntimeConfigurationError("Choose the .qmz file to run")
        if (!Files.isRegularFile(Path.of(scriptPath))) throw RuntimeConfigurationError("There is no file $scriptPath")
        if (mode == RunMode.SERVER && port !in 1..65535) throw RuntimeConfigurationError("The port must be 1 to 65535")
        val compiler = resolveCompiler()
            ?: throw RuntimeConfigurationError(
                "Cannot find the eTamil compiler. Put etamil on the PATH, set ETAMIL_BIN, or choose it in this configuration",
            )
        if (compilerPath.isNotBlank() && !Files.isRegularFile(Path.of(compiler))) {
            throw RuntimeConfigurationError("There is no compiler at $compiler")
        }
    }

    override fun suggestedName(): String? = scriptPath.takeIf { it.isNotBlank() }?.let { Path.of(it).fileName.toString() }

    override fun getConfigurationEditor(): SettingsEditor<out RunConfiguration> = ETamilRunConfigurationEditor(project)

    override fun getState(executor: Executor, environment: ExecutionEnvironment) = ETamilRunState(environment, this)
}
