// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import com.intellij.execution.RunManager
import com.intellij.execution.actions.ConfigurationContext
import com.intellij.execution.actions.ConfigurationFromContext
import com.intellij.execution.configurations.ConfigurationType
import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.configurations.RuntimeConfigurationError
import com.intellij.execution.executors.DefaultRunExecutor
import com.intellij.execution.runners.ExecutionEnvironmentBuilder
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.util.JDOMUtil
import com.intellij.execution.PsiLocation
import com.intellij.testFramework.fixtures.BasePlatformTestCase
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path

/** A headless IDE with the plugin loaded: the run configuration is registered and behaves. */
class RunConfigurationTest : BasePlatformTestCase() {
    private fun newConfiguration(): ETamilRunConfiguration =
        ETamilRunConfigurationType.factory().createTemplateConfiguration(project) as ETamilRunConfiguration

    private fun realFile(name: String = "a.qmz", text: String = "அச்சு(1);"): Path {
        val file = Files.createTempDirectory("etamil-run").resolve(name)
        Files.writeString(file, text)
        return file
    }

    fun testTheTypeIsRegisteredAndHasOneFactory() {
        val types = ConfigurationType.CONFIGURATION_TYPE_EP.extensionList
        val type = types.filterIsInstance<ETamilRunConfigurationType>().single()
        assertEquals("eTamil", type.displayName)
        assertEquals(1, type.configurationFactories.size)
    }

    fun testTheProducerMakesAConfigurationFromAQmzFile() {
        val psi = myFixture.configureByText("எளிய.qmz", "அச்சு(1);")
        val context = ConfigurationContext.createEmptyContextForLocation(PsiLocation(psi))
        val fromContext = context.configurationsFromContext.orEmpty().firstOrNull { it.configuration is ETamilRunConfiguration }
        assertNotNull("a configuration is offered for a .qmz file", fromContext)
        val configuration = fromContext!!.configuration as ETamilRunConfiguration
        assertEquals(psi.virtualFile.path, configuration.scriptPath)
        assertEquals("எளிய.qmz", configuration.name)
        assertEquals(RunMode.RUN, configuration.mode)
    }

    fun testTheProducerIgnoresOtherFiles() {
        val psi = myFixture.configureByText("notes.txt", "hello")
        val context = ConfigurationContext.createEmptyContextForLocation(PsiLocation(psi))
        val offered = context.configurationsFromContext.orEmpty().filter { it.configuration is ETamilRunConfiguration }
        assertTrue("nothing for a .txt file: $offered", offered.isEmpty())
    }

    fun testAnExistingConfigurationIsRecognisedAndNotDuplicated() {
        val psi = myFixture.configureByText("a.qmz", "அச்சு(1);")
        val context = ConfigurationContext.createEmptyContextForLocation(PsiLocation(psi))
        val first: ConfigurationFromContext = context.configurationsFromContext.orEmpty().first { it.configuration is ETamilRunConfiguration }
        val manager = RunManager.getInstance(project)
        val settings = manager.createConfiguration(first.configuration, ETamilRunConfigurationType.factory())
        manager.addConfiguration(settings)
        val again = ConfigurationContext.createEmptyContextForLocation(PsiLocation(psi))
        assertSame("the saved one is reused", settings, again.findExisting())
    }

    fun testSettingsSurviveBeingSavedAndLoaded() {
        val configuration = newConfiguration()
        configuration.scriptPath = "/p/வட்டி.qmz"
        configuration.mode = RunMode.SERVER
        configuration.port = 9191
        configuration.workingDirectory = "/p"
        configuration.compilerPath = "/opt/etamil"
        configuration.env = mapOf("ETAMIL_PATH" to "/lib")
        configuration.passParentEnvs = false

        val element = org.jdom.Element("configuration")
        configuration.writeExternal(element)
        val xml = JDOMUtil.write(element)

        val loaded = newConfiguration()
        loaded.readExternal(JDOMUtil.load(xml))
        assertEquals("/p/வட்டி.qmz", loaded.scriptPath)
        assertEquals(RunMode.SERVER, loaded.mode)
        assertEquals(9191, loaded.port)
        assertEquals("/p", loaded.workingDirectory)
        assertEquals("/opt/etamil", loaded.compilerPath)
        assertEquals(mapOf("ETAMIL_PATH" to "/lib"), loaded.env)
        assertFalse(loaded.passParentEnvs)
    }

    fun testCheckingReportsWhatIsWrong() {
        fun problem(configuration: ETamilRunConfiguration): String =
            try {
                configuration.checkConfiguration()
                "none"
            } catch (e: RuntimeConfigurationError) {
                e.message.orEmpty()
            }

        val configuration = newConfiguration()
        assertTrue(problem(configuration), problem(configuration).contains("Choose the .qmz file"))

        configuration.scriptPath = "/no/such/file.qmz"
        assertTrue(problem(configuration), problem(configuration).contains("There is no file"))

        configuration.scriptPath = realFile().toString()
        configuration.mode = RunMode.SERVER
        configuration.port = 70000
        assertTrue(problem(configuration), problem(configuration).contains("port"))

        configuration.mode = RunMode.RUN
        configuration.compilerPath = "/no/such/etamil"
        assertTrue(problem(configuration), problem(configuration).contains("There is no compiler at"))

        configuration.compilerPath = realFile("etamil").toString()
        assertEquals("none", problem(configuration))
    }

    fun testTheCommandLineHasTheCompilerTheArgumentsTheFolderAndUtf8() {
        val file = realFile("வட்டி.qmz")
        val configuration = newConfiguration()
        configuration.scriptPath = file.toString()
        configuration.compilerPath = "/opt/e/etamil"
        configuration.env = mapOf("ETAMIL_PATH" to "/lib")

        val line = commandLineOf(configuration)
        assertEquals("/opt/e/etamil", line.exePath)
        assertEquals(listOf("--vm", file.toString()), line.parametersList.list)
        assertEquals("the folder of the file", file.parent.toString(), line.workDirectory?.path?.let { Path.of(it).toString() })
        assertEquals(StandardCharsets.UTF_8, line.charset)
        assertEquals("/lib", line.environment["ETAMIL_PATH"])
        assertEquals(GeneralCommandLine.ParentEnvironmentType.CONSOLE, line.parentEnvironmentType)

        configuration.workingDirectory = "/elsewhere"
        configuration.passParentEnvs = false
        val other = commandLineOf(configuration)
        assertEquals(Path.of("/elsewhere").toString(), Path.of(other.workDirectory!!.path).toString())
        assertEquals(GeneralCommandLine.ParentEnvironmentType.NONE, other.parentEnvironmentType)
    }

    fun testTheEditorFillsAndAppliesEveryField() {
        val configuration = newConfiguration()
        configuration.scriptPath = "/p/a.qmz"
        configuration.mode = RunMode.SERVER
        configuration.port = 9000
        configuration.workingDirectory = "/p"
        configuration.compilerPath = "/opt/etamil"
        configuration.env = mapOf("K" to "V")
        configuration.passParentEnvs = false

        val editor = ETamilRunConfigurationEditor(project)
        editor.resetFrom(configuration)
        val applied = newConfiguration()
        editor.applyTo(applied)
        editor.component // builds the form
        assertEquals("/p/a.qmz", applied.scriptPath)
        assertEquals(RunMode.SERVER, applied.mode)
        assertEquals(9000, applied.port)
        assertEquals("/p", applied.workingDirectory)
        assertEquals("/opt/etamil", applied.compilerPath)
        assertEquals(mapOf("K" to "V"), applied.env)
        assertFalse(applied.passParentEnvs)
        Disposer.dispose(editor)
    }

    /** Only with a real compiler: set `ETAMIL_BIN`. It really runs a Tamil program and reads its output. */
    fun testARealRunPrintsTheProgramsOutput() {
        val compiler = System.getenv("ETAMIL_BIN")?.takeIf { Files.isRegularFile(Path.of(it)) }
        if (compiler == null) {
            println("skipped: ETAMIL_BIN does not name a compiler")
            return
        }
        val configuration = newConfiguration()
        configuration.scriptPath = realFile("hello.qmz", "அச்சு(\"வணக்கம்\");").toString()
        val output = StringBuilder()
        val process = commandLineOf(configuration).createProcess()
        val text = process.inputStream.readBytes().toString(StandardCharsets.UTF_8)
        output.append(text)
        process.waitFor()
        assertEquals(output.toString(), 0, process.exitValue())
        assertTrue(output.toString(), output.contains("வணக்கம்"))
    }

    private fun commandLineOf(configuration: ETamilRunConfiguration): GeneralCommandLine {
        val environment = ExecutionEnvironmentBuilder.create(DefaultRunExecutor.getRunExecutorInstance(), configuration).build()
        return ETamilRunState(environment, configuration).commandLine()
    }
}
