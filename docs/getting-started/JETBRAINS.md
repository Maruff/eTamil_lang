# eTamil in JetBrains IDEs

IntelliJ IDEA, PyCharm, WebStorm, Rider, CLion and the rest can edit eTamil
today, with no custom plugin: a TextMate bundle gives highlighting, and the free
**LSP4IJ** plugin connects the eTamil language server. This works in the
Community editions too.

## 1. Highlighting, comments and brackets

The bundle is the folder `packaging/jetbrains/etamil-textmate/` in this
repository: the generated TextMate grammar and the language configuration the VS
Code extension uses.

1. Settings → **Editor → TextMate Bundles**.
2. Click **+** and choose the `etamil-textmate` folder.
3. Apply. Files ending in `.qmz` are highlighted, `//` comments toggle with
   Ctrl+/ (Cmd+/), and brackets and quotes pair.

If `.qmz` opens as plain text, check Settings → Editor → File Types: `.qmz`
should be listed under *TextMate*, and must not also be registered to another
file type.

## 2. Diagnostics, completion, hover and go to definition

The language server is `etamil-lsp`. It is in the release packages, and the
eTamil installers put it beside `etamil`, so installing eTamil installs it (a
package from before it existed does not have it). To build it from this
repository instead:

```bash
cd etamil_lsp
cargo build --release          # produces target/release/etamil-lsp (.exe on Windows)
```

Then connect it:

1. Install **LSP4IJ** (by Red Hat) from Settings → Plugins → Marketplace, and
   restart the IDE.
2. Settings → **Languages & Frameworks → Language Servers**, click **+**
   (New Language Server).
3. Name: `eTamil`. Command: the full path to `etamil-lsp` (`etamil-lsp.exe` on
   Windows).
4. On the **Mappings** tab, add a *File name pattern* `*.qmz` with the language
   id `etamil`.
5. Open a `.qmz` file. Errors are underlined with the compiler's own
   bilingual messages; hover shows a name's signature or type, or the
   documentation of a keyword, builtin or standard library function; Ctrl+click
   (Cmd+click) goes to where a name is first written; completion offers the
   names in scope, eTamil's keywords in Tamil and Latin spellings (with a
   statement template for each that has one), and the builtin and standard
   library functions, which add their `இறக்கு` import for you.

LSP4IJ has a **Language Servers** tool window that shows whether the server is
running and its log, which is the first place to look if nothing appears.

## 3. Running a file

Add two External Tools (Settings → **Tools → External Tools** → **+**), with the
program set to the path of `etamil`, and the working directory `$FileDir$`:

| Name | Arguments |
|---|---|
| eTamil: run | `$FilePath$` |
| eTamil: check | `--check $FilePath$` |

They appear under Tools → External Tools, and can be given keyboard shortcuts.
`--check` parses and type-checks and never runs the program.

For `இறக்கு "nUlakam/..."` imports to resolve, set the environment variable
`ETAMIL_PATH` to the folder that holds `nUlakam`, in the tool's settings or in
your system environment. The release packages' installers set it for you.

## What this does not give you

- **Signature help** (the parameter hint that follows you inside a call's
  parentheses). Completion inserts the parameters as tab stops, and hover shows
  the signature, but nothing tracks the argument you are typing.
- **A debugger**, and a run configuration with its own tool window. Run output
  appears in the External Tools console.
- **Semantic highlighting.** TextMate colours by pattern, so an identifier is
  coloured the same whether it is a function or a variable.

A plugin that wires highlighting and the language server up in one install is in
`packaging/jetbrains/plugin/` (see its README); it is built and verified but not
published, so the manual steps above still apply until it is.

## Keeping the bundle current

The grammar is generated from the compiler's keyword tables. After adding a
keyword and regenerating the editor support, refresh the bundle, and check it in
CI:

```bash
packaging/jetbrains/sync-bundle.sh ../eTamil_vsCode           # update
packaging/jetbrains/sync-bundle.sh ../eTamil_vsCode --check   # exit 1 if stale
```
