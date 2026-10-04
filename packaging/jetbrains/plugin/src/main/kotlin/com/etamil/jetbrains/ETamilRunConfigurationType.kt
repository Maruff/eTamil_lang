// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
package com.etamil.jetbrains

import com.intellij.execution.configurations.ConfigurationFactory
import com.intellij.execution.configurations.ConfigurationTypeBase
import com.intellij.execution.configurations.ConfigurationTypeUtil
import com.intellij.execution.configurations.RunConfiguration
import com.intellij.execution.configurations.RunConfigurationOptions
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.IconLoader

/** "eTamil" in the Run | Edit Configurations list. */
class ETamilRunConfigurationType : ConfigurationTypeBase(
    ID,
    "eTamil",
    "Run, check or serve an eTamil program",
    IconLoader.getIcon("/META-INF/pluginIcon.svg", ETamilRunConfigurationType::class.java),
) {
    init {
        addFactory(ETamilConfigurationFactory(this))
    }

    companion object {
        const val ID = "ETamilRunConfiguration"

        fun factory(): ConfigurationFactory =
            ConfigurationTypeUtil.findConfigurationType(ETamilRunConfigurationType::class.java).configurationFactories.single()
    }
}

class ETamilConfigurationFactory(type: ETamilRunConfigurationType) : ConfigurationFactory(type) {
    override fun getId() = ETamilRunConfigurationType.ID

    override fun createTemplateConfiguration(project: Project): RunConfiguration =
        ETamilRunConfiguration(project, this, "eTamil")

    override fun getOptionsClass(): Class<out RunConfigurationOptions> = ETamilRunConfigurationOptions::class.java
}
