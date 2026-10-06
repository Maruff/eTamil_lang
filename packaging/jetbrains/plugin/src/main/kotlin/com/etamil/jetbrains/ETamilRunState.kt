// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import com.intellij.execution.ExecutionException
import com.intellij.execution.configurations.CommandLineState
import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.process.ProcessHandler
import com.intellij.execution.process.ProcessHandlerFactory
import com.intellij.execution.process.ProcessTerminatedListener
import com.intellij.execution.runners.ExecutionEnvironment
import java.nio.charset.StandardCharsets

/** Starts the compiler and streams its output to the Run window. */
class ETamilRunState(environment: ExecutionEnvironment, private val configuration: ETamilRunConfiguration) :
    CommandLineState(environment) {

    fun commandLine(): GeneralCommandLine {
        val compiler = configuration.resolveCompiler()
            ?: throw ExecutionException(
                "Cannot find the eTamil compiler: put etamil on the PATH, set ETAMIL_BIN, or choose it in the run configuration",
            )
        return GeneralCommandLine(compiler)
            .withParameters(CompilerArguments.build(configuration.mode, configuration.scriptPath, configuration.port))
            .withWorkDirectory(configuration.resolveWorkingDirectory().ifBlank { null })
            .withEnvironment(configuration.env)
            .withParentEnvironmentType(
                if (configuration.passParentEnvs) GeneralCommandLine.ParentEnvironmentType.CONSOLE
                else GeneralCommandLine.ParentEnvironmentType.NONE,
            )
            // Tamil output: the compiler writes UTF-8, whatever the platform's default is.
            .withCharset(StandardCharsets.UTF_8)
    }

    override fun startProcess(): ProcessHandler {
        val handler = ProcessHandlerFactory.getInstance().createColoredProcessHandler(commandLine())
        ProcessTerminatedListener.attach(handler)
        return handler
    }
}
