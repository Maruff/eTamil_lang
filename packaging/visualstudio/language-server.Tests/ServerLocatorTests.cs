// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>

namespace EtamilLanguageServer.Tests;

using System.Collections.Generic;
using System.IO;
using Xunit;

public class ServerLocatorTests
{
    private const string Home = @"C:\Users\me";
    private const string Assembly = @"C:\ext\etamil";

    private static string? Locate(Dictionary<string, string> env, params string[] existing)
    {
        var files = new HashSet<string>(existing);
        return ServerLocator.Locate(
            name => env.TryGetValue(name, out var value) ? value : null,
            Assembly,
            Home,
            files.Contains);
    }

    [Fact]
    public void NothingIsFoundWhenNothingIsThere() =>
        Assert.Null(Locate(new() { ["PATH"] = @"C:\Windows;C:\tools" }));

    [Fact]
    public void TheEnvironmentVariableWinsEvenOverACarriedServer()
    {
        var carried = Path.Combine(Assembly, "etamil-lsp.exe");
        Assert.Equal(@"D:\mine\etamil-lsp.exe", Locate(new() { ["ETAMIL_LSP"] = @"  D:\mine\etamil-lsp.exe " }, carried));
    }

    [Fact]
    public void ABlankEnvironmentVariableIsIgnored()
    {
        var carried = Path.Combine(Assembly, "etamil-lsp.exe");
        Assert.Equal(carried, Locate(new() { ["ETAMIL_LSP"] = "   " }, carried));
    }

    [Fact]
    public void TheServerBesideTheAssemblyBeatsThePath()
    {
        var carried = Path.Combine(Assembly, "etamil-lsp.exe");
        var onPath = Path.Combine(@"C:\tools", "etamil-lsp.exe");
        Assert.Equal(carried, Locate(new() { ["PATH"] = @"C:\tools" }, carried, onPath));
    }

    [Fact]
    public void ThePathIsSearchedInOrderAndEmptyEntriesAreSkipped()
    {
        var first = Path.Combine(@"C:\a", "etamil-lsp.exe");
        var second = Path.Combine(@"C:\b", "etamil-lsp.exe");
        Assert.Equal(first, Locate(new() { ["PATH"] = @";;C:\a;C:\b" }, first, second));
    }

    [Fact]
    public void ThePathVariableMayBeSpelledPath()
    {
        var onPath = Path.Combine(@"C:\tools", "etamil-lsp.exe");
        Assert.Equal(onPath, Locate(new() { ["Path"] = @"C:\tools" }, onPath));
    }

    [Fact]
    public void TheInstallersFoldersAreTheLastResort()
    {
        var installed = Path.Combine(@"C:\Users\me\AppData\Local", "Programs", "eTamil", "bin", "etamil-lsp.exe");
        Assert.Equal(installed, Locate(new() { ["PATH"] = @"C:\Windows", ["LOCALAPPDATA"] = @"C:\Users\me\AppData\Local" }, installed));
    }
}
