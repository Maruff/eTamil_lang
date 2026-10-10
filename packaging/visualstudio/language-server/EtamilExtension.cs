// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>

namespace EtamilLanguageServer;

using Microsoft.Extensions.DependencyInjection;
using Microsoft.VisualStudio.Extensibility;

/// <summary>
/// Entry point of the eTamil language server extension.
/// </summary>
[VisualStudioContribution]
internal class EtamilExtension : Extension
{
    /// <inheritdoc/>
    public override ExtensionConfiguration ExtensionConfiguration => new()
    {
        Metadata = new(
            id: "EtamilLanguageServer.c3ab8044-33bb-48e1-92ae-465ebb20fdd3",
            version: this.ExtensionAssemblyVersion,
            publisherName: "eTamil",
            displayName: "eTamil Language Server",
            description: "Diagnostics, completion, hover and go to definition for eTamil (.qmz) files, from the eTamil language server."),
    };

    /// <inheritdoc/>
    protected override void InitializeServices(IServiceCollection serviceCollection)
    {
        base.InitializeServices(serviceCollection);
    }
}
