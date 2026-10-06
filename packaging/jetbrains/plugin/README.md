# eTamil plugin for JetBrains IDEs

One install that gives IntelliJ-based IDEs (IDEA, PyCharm, WebStorm, Rider, CLion
and the rest, Community editions included) support for `.qmz` files:

- highlighting, comment toggling and bracket pairing, from the same TextMate
  grammar the VS Code extension uses (`../etamil-textmate`, copied in at build time);
- diagnostics, completion of names in scope, hover and go to definition, from the
  eTamil language server `etamil-lsp`, through the **LSP4IJ** plugin.

It targets IDE builds 242 and later (2024.2+).

## Install

1. Install **LSP4IJ** from Settings → Plugins → Marketplace. Installing from disk
   does not fetch dependencies, so this comes first.
2. Settings → Plugins → ⚙ → **Install Plugin from Disk…**, and pick
   `build/distributions/etamil-jetbrains-0.1.0.zip`.
3. Make `etamil-lsp` findable: put it on the `PATH`, or set the `ETAMIL_LSP`
   environment variable to its full path. It is also looked for in `~/.local/bin`
   and, on Windows, `%LOCALAPPDATA%\Programs\eTamil`. It is in the
   release packages (the eTamil installers put it beside `etamil`); to build it yourself,
   run `cargo build --release` in `etamil_lsp/`.
4. Restart the IDE and open a `.qmz` file.

## Running a file

Open a `.qmz` file and use **Run | Run 'file.qmz'** (Ctrl+Shift+F10), or right-click
the file in the editor or the Project view. The plugin makes an **eTamil** run
configuration for it, which you can edit under Run | Edit Configurations:

- **Mode:** *Run* (`etamil --vm file`), *Check only* (`--check`, which never runs the
  program) or *Run as an HTTP server* (`--server --port N`, running until stopped).
- **Working directory:** the folder of the file unless set, so relative paths in the
  program work.
- **Environment variables:** for example `ETAMIL_PATH`, the folder that holds `nUlakam`
  for `இறக்கு` imports, and whether to pass the IDE's own environment on.
- **Compiler:** left empty it is `ETAMIL_BIN`, then the `PATH`, then `~/.local/bin` and,
  on Windows, `%LOCALAPPDATA%\Programs\eTamil`, the same order as for `etamil-lsp`.

Output goes to the Run window, read as UTF-8 so Tamil text is not garbled.

## Build

JDK 21 is needed. The Gradle wrapper does the rest:

```bash
./gradlew buildPlugin     # build/distributions/etamil-jetbrains-0.1.0.zip
./gradlew test            # 18 unit tests, and 12 that start a headless IDE
./gradlew verifyPlugin    # JetBrains Plugin Verifier against the build platform
```

The first build downloads the IntelliJ Platform (about 700 MB).

### Building on Windows

On some Windows machines JDK 21 cannot open its internal pipes through the default
Unix-domain socket folder, and Gradle fails at once with
`Unable to establish loopback connection` (or, from the daemon, "the first result
from the daemon was empty"). Pointing that folder at a short path fixes it, and it
has to reach the Gradle daemon too, so set it through `JAVA_TOOL_OPTIONS`:

```bash
mkdir -p /d/tools/tmp
export JAVA_TOOL_OPTIONS="-Djdk.net.unixdomain.tmpdir=D:/tools/tmp"
```

## What is verified, and what is not

- The Plugin Verifier reports the plugin **compatible** with IntelliJ IDEA 2024.2.
- A headless IDE loads the plugin with TextMate and LSP4IJ, the bundle provider is
  registered, and TextMate's own reader opens the bundle and finds `.qmz` in it.
- The run configuration is registered; the producer offers it for a `.qmz` file and for
  nothing else, and recognises a saved one instead of duplicating it; its settings
  survive being saved and loaded; the form fills and applies every field; a bad
  configuration says what is wrong; the command line has the right compiler,
  arguments, folder and UTF-8; and, with `ETAMIL_BIN` set to a compiler, a real run
  of a Tamil program returns its output (the test is skipped without it).
- Not exercised: the run configuration's form and the Run window on screen, highlighting and the language server inside a running IDE (a
  headless test does not run TextMate's startup registration, and the server
  needs `etamil-lsp` built). Try it once by hand before publishing.

## Not done yet

- A settings page for the server path (today: `PATH` or `ETAMIL_LSP`).
- A debugger hook. The compiler has no debugger or debug-adapter protocol to attach
  to, so there is nothing to hook yet.
- Keyword and standard-library completion, which the server does not offer.
- Signing and publishing to the JetBrains Marketplace, and verifying against newer
  IDE builds (`create(...)` or `recommended()` in `build.gradle.kts`).
