# eTamil in Visual Studio

Visual Studio 2022 and later, every edition. Two extensions, because they are
different kinds of extension and are useful separately:

| Extension | Gives you | Needs |
|---|---|---|
| **eTamil Language Support** (`etamil-language-1.0.0.vsix`) | highlighting, comment toggling (Ctrl+K, Ctrl+C), bracket matching and pairing | Visual Studio 17.0 or later |
| **eTamil Language Server** | diagnostics, completion of names in scope, hover, go to definition | Visual Studio **17.14** or later, and `etamil-lsp.exe` |

## 1. Language support (highlighting, comments, brackets)

Build the package, or take it from a release:

```bash
python packaging/visualstudio/build-vsix.py        # writes packaging/visualstudio/dist/etamil-language-1.0.0.vsix
```

Close Visual Studio, double-click the `.vsix`, and confirm the installer. Open a
`.qmz` file. The package carries no code: a `.pkgdef` registers the TextMate
grammar and the language configuration the VS Code extension also uses, so what
you see matches VS Code.

If the file is not coloured, check **Tools → Options → Text Editor → File
Extension** and make sure `.qmz` is not mapped to another editor.

## 2. Language server (diagnostics, completion, hover, definition)

This extension is built with the .NET SDK, and needs the server binary. The eTamil
release package and installer already provide it (`etamil-lsp.exe`, beside
`etamil.exe`), and the extension finds it on the `PATH`. To carry it inside the
extension instead, take it from a release package or build it:

```bash
cd etamil_lsp && cargo build --release          # target/release/etamil-lsp.exe
cd ../packaging/visualstudio/language-server
copy ..\..\..\etamil_lsp\target\release\etamil-lsp.exe .    # carried inside the extension
dotnet build -c Release                         # produces the .vsix
```

Install the resulting `.vsix` the same way. If you skip the copy, the extension
looks for the server in this order: the `ETAMIL_LSP` environment variable, beside
the extension, the `PATH`, then `%LOCALAPPDATA%\Programs\eTamil` and
`~\.local\bin`. If it cannot start the server it turns itself off rather than
retrying on every file.

Errors are underlined with the compiler's own bilingual messages; hover shows a
name's signature or type, or the documentation of a keyword, builtin or standard
library function; F12 goes to where a name is first written; completion offers the
names in scope, eTamil's keywords in Tamil and Latin spellings, and the builtin
and standard library functions, which add their import for you.

## What this does not give you

- **Signature help** while typing a call's arguments. Completion inserts the
  parameters as tab stops, and hover shows the signature.
- **A debugger or a project system.** Run a file from a terminal with
  `etamil file.qmz`, or add an **External Tool** (Tools → External Tools).
- **Semantic highlighting.** TextMate colours by pattern.

## Keeping the grammar current

The VSIX takes its grammar from `packaging/jetbrains/etamil-textmate`, the single
copy that `packaging/jetbrains/sync-bundle.sh` keeps in step with the generated
editor support. After a keyword is added, run that script, then rebuild the VSIX.
