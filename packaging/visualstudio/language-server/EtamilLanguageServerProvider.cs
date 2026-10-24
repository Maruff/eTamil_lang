// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>

namespace EtamilLanguageServer;

using System;
using System.Diagnostics;
using System.IO;
using System.IO.Pipelines;
using System.Reflection;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.VisualStudio.Extensibility;
using Microsoft.VisualStudio.Extensibility.Editor;
using Microsoft.VisualStudio.Extensibility.LanguageServer;
using Microsoft.VisualStudio.RpcContracts.LanguageServerProvider;
using Nerdbank.Streams;

/// <summary>
/// Starts <c>etamil-lsp</c> for eTamil (<c>.qmz</c>) files and connects Visual Studio to it
/// over the server's standard input and output.
/// </summary>
#pragma warning disable VSEXTPREVIEW_LSP // The language server provider API is in preview.
[VisualStudioContribution]
internal class EtamilLanguageServerProvider : LanguageServerProvider
{
    /// <summary>The document type for eTamil source files.</summary>
    [VisualStudioContribution]
    public static DocumentTypeConfiguration EtamilDocumentType => new("etamil")
    {
        FileExtensions = [".qmz"],
        BaseDocumentType = LanguageServerBaseDocumentType,
    };

    /// <inheritdoc/>
    public override LanguageServerProviderConfiguration LanguageServerProviderConfiguration => new(
        "%EtamilLanguageServerProvider.DisplayName%",
        [DocumentFilter.FromDocumentType(EtamilDocumentType)]);

    /// <inheritdoc/>
    public override Task<IDuplexPipe?> CreateServerConnectionAsync(CancellationToken cancellationToken)
    {
        string? server = ServerLocator.Locate(
            name => Environment.GetEnvironmentVariable(name),
            Path.GetDirectoryName(Assembly.GetExecutingAssembly().Location)!,
            Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            File.Exists);

        if (server is null)
        {
            // Nothing to start. Visual Studio reports the failed activation, and
            // OnServerInitializationResultAsync turns the provider off.
            return Task.FromResult<IDuplexPipe?>(null);
        }

        var info = new ProcessStartInfo
        {
            FileName = server,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            UseShellExecute = false,
            CreateNoWindow = true,
        };

#pragma warning disable CA2000 // The process is disposed after Visual Studio sends the stop command.
        var process = new Process { StartInfo = info };
#pragma warning restore CA2000

        try
        {
            if (process.Start())
            {
                return Task.FromResult<IDuplexPipe?>(new DuplexPipe(
                    PipeReader.Create(process.StandardOutput.BaseStream),
                    PipeWriter.Create(process.StandardInput.BaseStream)));
            }
        }
        catch (Exception ex) when (ex is System.ComponentModel.Win32Exception or InvalidOperationException)
        {
            // A path that is not an executable, or no permission to run it.
        }

        return Task.FromResult<IDuplexPipe?>(null);
    }

    /// <inheritdoc/>
    public override Task OnServerInitializationResultAsync(
        ServerInitializationResult serverInitializationResult,
        LanguageServerInitializationFailureInfo? initializationFailureInfo,
        CancellationToken cancellationToken)
    {
        if (serverInitializationResult == ServerInitializationResult.Failed)
        {
            // Do not retry on every file that is opened.
            this.Enabled = false;
        }

        return base.OnServerInitializationResultAsync(serverInitializationResult, initializationFailureInfo, cancellationToken);
    }
}
#pragma warning restore VSEXTPREVIEW_LSP
