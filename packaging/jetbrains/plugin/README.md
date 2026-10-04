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

## Build

JDK 21 is needed. The Gradle wrapper does the rest:

```bash
./gradlew buildPlugin     # build/distributions/etamil-jetbrains-0.1.0.zip
./gradlew test            # 7 unit tests, and 3 that start a headless IDE
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
- Not exercised: highlighting and the language server inside a running IDE (a
  headless test does not run TextMate's startup registration, and the server
  needs `etamil-lsp` built). Try it once by hand before publishing.

## Not done yet

- A settings page for the server path (today: `PATH` or `ETAMIL_LSP`).
- A run configuration and a debugger hook. External Tools cover running a file
  (see `docs/getting-started/JETBRAINS.md`).
- Keyword and standard-library completion, which the server does not offer.
- Signing and publishing to the JetBrains Marketplace, and verifying against newer
  IDE builds (`create(...)` or `recommended()` in `build.gradle.kts`).
