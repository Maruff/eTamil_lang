// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import com.intellij.execution.actions.ConfigurationContext
import com.intellij.execution.actions.LazyRunConfigurationProducer
import com.intellij.execution.configurations.ConfigurationFactory
import com.intellij.openapi.util.Ref
import com.intellij.psi.PsiElement

/**
 * Makes a run configuration from a `.qmz` file: Run | Run 'file.qmz', Ctrl+Shift+F10,
 * or right-click in the editor or the Project view.
 */
class ETamilRunConfigurationProducer : LazyRunConfigurationProducer<ETamilRunConfiguration>() {
    override fun getConfigurationFactory(): ConfigurationFactory = ETamilRunConfigurationType.factory()

    private fun qmzPath(context: ConfigurationContext): String? {
        val file = context.psiLocation?.containingFile?.virtualFile ?: return null
        return file.path.takeIf { file.extension == "qmz" }
    }

    override fun setupConfigurationFromContext(
        configuration: ETamilRunConfiguration,
        context: ConfigurationContext,
        sourceElement: Ref<PsiElement>,
    ): Boolean {
        val path = qmzPath(context) ?: return false
        configuration.scriptPath = path
        configuration.setGeneratedName()
        return true
    }

    override fun isConfigurationFromContext(configuration: ETamilRunConfiguration, context: ConfigurationContext): Boolean =
        qmzPath(context)?.let { it == configuration.scriptPath } ?: false
}
