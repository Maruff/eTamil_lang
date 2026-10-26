# eTamil as a .NET tool

The [eTamil](https://etamil.in) compiler and language server, installed as .NET global tools. Needs the .NET 8 SDK or runtime (or newer).

```bash
dotnet tool install -g etamil          # the compiler: etamil program.qmz
dotnet tool install -g etamil-lsp      # the language server, for editors
etamil --version
```

Each package carries the real, prebuilt binary for Windows x64, Linux x64/arm64 and macOS x64/arm64, plus a small launcher that picks the one for your machine and runs it with your arguments and your exit code. The `etamil` package also carries the standard library (`nUlakam`), and the launcher points `ETAMIL_PATH` at it unless you have set your own. Nothing is downloaded when you install, and no Rust is needed.

To update: `dotnet tool update -g etamil`. To remove: `dotnet tool uninstall -g etamil`.

Editors: set the language server's path to `etamil-lsp` (it is on your PATH after installing the tool).
