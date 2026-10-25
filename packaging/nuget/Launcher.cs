// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>

// The whole of the .NET tool: find the binary that belongs to this machine inside
// the package, make sure it can run, and run it with the same arguments, the same
// standard streams and the same exit code. Shared by the `etamil` and `etamil-lsp`
// packages; which binary to run is baked in as assembly metadata ("binary").

using System.Diagnostics;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.Versioning;

internal static class Launcher
{
    private static int Main(string[] args)
    {
        string name = typeof(Launcher).Assembly
            .GetCustomAttributes<AssemblyMetadataAttribute>()
            .First(attribute => attribute.Key == "binary").Value!;

        string? rid = RuntimeIdentifier();
        if (rid is null)
        {
            Console.Error.WriteLine(
                $"{name}: no eTamil binary for {RuntimeInformation.OSDescription} " +
                $"on {RuntimeInformation.ProcessArchitecture}. " +
                "Supported: Windows x64, Linux x64/arm64, macOS x64/arm64.");
            return 1;
        }

        string exe = Path.Combine(
            AppContext.BaseDirectory, "bin", rid, OperatingSystem.IsWindows() ? name + ".exe" : name);
        if (!File.Exists(exe))
        {
            Console.Error.WriteLine($"{name}: this package carries no binary for {rid} ({exe}).");
            return 1;
        }

        // A NuGet package is a zip, and a zip does not keep the execute bit.
        if (!OperatingSystem.IsWindows())
        {
            EnsureExecutable(exe);
        }

        var info = new ProcessStartInfo(exe) { UseShellExecute = false };
        foreach (string argument in args)
        {
            info.ArgumentList.Add(argument);
        }

        // Lets  இறக்கு "nUlakam/..."  resolve from any directory, as the installers do.
        string library = Path.Combine(AppContext.BaseDirectory, "lib");
        if (Directory.Exists(library) && string.IsNullOrEmpty(Environment.GetEnvironmentVariable("ETAMIL_PATH")))
        {
            info.Environment["ETAMIL_PATH"] = library;
        }

        // Ctrl+C reaches the child too; stay alive until it has exited, so its exit
        // code, not ours, is what the caller sees.
        Console.CancelKeyPress += (_, e) => e.Cancel = true;

        using Process process = Process.Start(info)!;
        process.WaitForExit();
        return process.ExitCode;
    }

    /// <summary>The runtime identifier of this machine, among the five that are shipped.</summary>
    private static string? RuntimeIdentifier()
    {
        Architecture architecture = RuntimeInformation.ProcessArchitecture;
        if (OperatingSystem.IsWindows())
        {
            return architecture == Architecture.X64 ? "win-x64" : null;
        }

        string? prefix = OperatingSystem.IsLinux() ? "linux" : OperatingSystem.IsMacOS() ? "osx" : null;
        return prefix is null ? null : architecture switch
        {
            Architecture.X64 => prefix + "-x64",
            Architecture.Arm64 => prefix + "-arm64",
            _ => null,
        };
    }

    [UnsupportedOSPlatform("windows")]
    private static void EnsureExecutable(string path)
    {
        UnixFileMode mode = File.GetUnixFileMode(path);
        UnixFileMode wanted = mode | UnixFileMode.UserExecute | UnixFileMode.GroupExecute | UnixFileMode.OtherExecute;
        if (mode != wanted)
        {
            File.SetUnixFileMode(path, wanted);
        }
    }
}
