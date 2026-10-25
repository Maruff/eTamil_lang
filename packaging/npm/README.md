# etamil

The [eTamil](https://etamil.in) compiler for Node environments.

```bash
npx etamil --version
npm install -g etamil
etamil program.qmz
```

The package also installs `etamil-lsp`, the language server editors use for
diagnostics, completion, hover and go to definition (an editor runs it; set its
"server path" to `etamil-lsp` or the full path from `npm bin -g`).

Installing downloads the prebuilt binary for your platform from the matching
GitHub release (Windows x64, Linux x64/arm64, macOS x64/arm64), checks its
SHA-256, and unpacks it inside the package. Node 18 or newer. Nothing is
compiled, and Rust is not needed.

If install scripts are disabled (`--ignore-scripts`), run `npm rebuild etamil`
afterwards. For a mirror or offline install, set `ETAMIL_RELEASE_BASE` to a
folder serving the release archive and its `.sha256`.
