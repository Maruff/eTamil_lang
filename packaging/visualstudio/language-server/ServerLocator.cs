// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>

namespace EtamilLanguageServer;

using System;
using System.Collections.Generic;
using System.IO;

/// <summary>
/// Where the <c>etamil-lsp</c> binary is. Pure, with the file system and
/// environment passed in, so the order can be tested without Visual Studio.
/// </summary>
/// <remarks>
/// The order matches the other editors: the <c>ETAMIL_LSP</c> variable (an
/// explicit choice always wins), then the server carried beside this assembly,
/// then the <c>PATH</c>, then the folders the eTamil installers use.
/// </remarks>
internal static class ServerLocator
{
    public const string EnvironmentVariable = "ETAMIL_LSP";

    public const string BinaryName = "etamil-lsp.exe";

    /// <summary>The full path to the server, or <c>null</c> if it is nowhere we know to look.</summary>
    public static string? Locate(
        Func<string, string?> environment,
        string assemblyDirectory,
        string home,
        Func<string, bool> fileExists)
    {
        string? explicitPath = environment(EnvironmentVariable)?.Trim();
        if (!string.IsNullOrEmpty(explicitPath))
        {
            return explicitPath;
        }

        string carried = Path.Combine(assemblyDirectory, BinaryName);
        if (fileExists(carried))
        {
            return carried;
        }

        string path = environment("PATH") ?? environment("Path") ?? string.Empty;
        foreach (string directory in path.Split(';', StringSplitOptions.RemoveEmptyEntries))
        {
            string candidate = Path.Combine(directory.Trim(), BinaryName);
            if (fileExists(candidate))
            {
                return candidate;
            }
        }

        var installFolders = new List<string> { Path.Combine(home, ".local", "bin") };
        string? localAppData = environment("LOCALAPPDATA");
        if (!string.IsNullOrEmpty(localAppData))
        {
            installFolders.Add(Path.Combine(localAppData, "Programs", "eTamil"));
        }

        foreach (string folder in installFolders)
        {
            string candidate = Path.Combine(folder, BinaryName);
            if (fileExists(candidate))
            {
                return candidate;
            }
        }

        return null;
    }
}
