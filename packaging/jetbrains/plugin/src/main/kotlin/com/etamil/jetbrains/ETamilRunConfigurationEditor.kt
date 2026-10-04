// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import com.intellij.execution.configuration.EnvironmentVariablesData
import com.intellij.execution.configuration.EnvironmentVariablesTextFieldWithBrowseButton
import com.intellij.openapi.fileChooser.FileChooserDescriptorFactory
import com.intellij.openapi.options.SettingsEditor
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.ComboBox
import com.intellij.openapi.ui.TextBrowseFolderListener
import com.intellij.openapi.ui.TextFieldWithBrowseButton
import com.intellij.ui.components.JBTextField
import com.intellij.util.ui.FormBuilder
import javax.swing.DefaultComboBoxModel
import javax.swing.JComponent
import javax.swing.JSpinner
import javax.swing.SpinnerNumberModel

/** The form behind Run | Edit Configurations | eTamil. */
class ETamilRunConfigurationEditor(project: Project) : SettingsEditor<ETamilRunConfiguration>() {
    private val script = TextFieldWithBrowseButton().apply {
        addActionListener(TextBrowseFolderListener(FileChooserDescriptorFactory.createSingleFileDescriptor().withTitle("eTamil File"), project))
    }
    private val mode = ComboBox(DefaultComboBoxModel(RunMode.values()))
    private val port = JSpinner(SpinnerNumberModel(CompilerArguments.DEFAULT_PORT, 1, 65535, 1))
    private val workingDirectory = TextFieldWithBrowseButton().apply {
        addActionListener(TextBrowseFolderListener(FileChooserDescriptorFactory.createSingleFolderDescriptor().withTitle("Working Directory"), project))
        // Leaving it empty is the normal case; say what then happens.
        (textField as? JBTextField)?.emptyText?.text = "Default: the folder of the file"
    }
    private val env = EnvironmentVariablesTextFieldWithBrowseButton()
    private val compiler = TextFieldWithBrowseButton().apply {
        addActionListener(TextBrowseFolderListener(FileChooserDescriptorFactory.createSingleFileDescriptor().withTitle("eTamil Compiler"), project))
        (textField as? JBTextField)?.emptyText?.text = "Default: ETAMIL_BIN, then the PATH"
    }

    init {
        // The port matters only to a server.
        mode.addActionListener { port.isEnabled = mode.selectedItem == RunMode.SERVER }
        port.isEnabled = false
    }

    override fun resetEditorFrom(configuration: ETamilRunConfiguration) {
        script.text = configuration.scriptPath
        mode.selectedItem = configuration.mode
        port.value = configuration.port
        workingDirectory.text = configuration.workingDirectory
        compiler.text = configuration.compilerPath
        env.data = EnvironmentVariablesData.create(configuration.env, configuration.passParentEnvs)
        port.isEnabled = configuration.mode == RunMode.SERVER
    }

    override fun applyEditorTo(configuration: ETamilRunConfiguration) {
        configuration.scriptPath = script.text.trim()
        configuration.mode = mode.selectedItem as RunMode
        configuration.port = port.value as Int
        configuration.workingDirectory = workingDirectory.text.trim()
        configuration.compilerPath = compiler.text.trim()
        configuration.env = env.data.envs
        configuration.passParentEnvs = env.data.isPassParentEnvs
    }

    override fun createEditor(): JComponent = FormBuilder.createFormBuilder()
        .addLabeledComponent("File:", script)
        .addLabeledComponent("Mode:", mode)
        .addLabeledComponent("Port:", port)
        .addLabeledComponent("Working directory:", workingDirectory)
        .addLabeledComponent("Environment variables:", env)
        .addLabeledComponent("Compiler:", compiler)
        .panel
}
