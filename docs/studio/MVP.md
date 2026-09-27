# eTamil Studio — MVP knowledge base (v1.1)

Status: **knowledge base, not a specification.** Captured 2026-09-13 from a planning
document supplied by the project owner. Kept here as the reference for what eTamil
Studio is meant to be. Parts of it do not match what this repository actually
contains — see [Where this document and the repository disagree](#where-this-document-and-the-repository-disagree)
at the end. Nothing here has been implemented.

## Purpose

Build a practical MVP of eTamil Studio that one developer can build, host and maintain
on a single DigitalOcean droplet.

The objective is not the final platform. It is an operational platform that can:

- Generate eTamil applications
- Compile and run eTamil applications
- Develop and test applications online
- Build Linux distributions of the eTamil compiler
- Host compiler downloads
- Host demo applications
- Serve as the foundation for the long-term eTamil ecosystem

## The loop

```
English requirement
  → AI assistant
    → eTamil source
      → eTamil compiler
        → working application
```

A user should be able to: create a project, generate code, compile, run, debug, and
generate documentation.

## Scope

### In

**Website — `etamil.in`**, hosting documentation, compiler downloads, sample projects
and Studio access.

**eTamil Studio** — online IDE, source management, compiler integration, AI assistance.

**AI assistance** — English input, code generation, code explanation, documentation
generation, error resolution.

**Compiler** — Linux build, compile applications, package Linux releases.

**Application hosting** — demo applications at `kelir.org`, `qos.ae`, `conf.ae`,
`ineo.in`, each an eTamil backend with a React frontend over PostgreSQL.

### Out

| Excluded | Reason |
|---|---|
| Tamil-language AI input | English only for the MVP; eTamil-language understanding is a later phase |
| Model training | No LoRA, no fine-tuning, no custom models |
| Multi-tenant workspaces | One developer, a handful of demos |

## Infrastructure

DigitalOcean droplet: 4 GB RAM, 2 vCPU, 80 GB SSD, Ubuntu 24.04.

### Build strategy per platform

| Platform | Where it is built |
|---|---|
| Linux | On the droplet — compiler, package, releases |
| Windows | Locally, in a Windows VM |
| Android | GitHub Actions |
| Raspberry Pi | GitHub Actions — `release.yml`, static aarch64 musl on the `ubuntu-24.04-arm` runner |
| macOS | Future phase — GitHub macOS runner, or a Mac Mini |

## System architecture

```
Internet
   │
   ▼
 Nginx
   │
   ├──────────────┬──────────────┐
   ▼              ▼              ▼
etamil.in    eTamil Studio    Demo apps
                   │
                   ▼
               OpenCode
                   │
                   ▼
              Hermes Agent
                   │
                   ▼
                 Ollama
                   │
                   ▼
             Qwen / Gemma
                   │
                   ▼
            eTamil compiler
                   │
                   ▼
              PostgreSQL
```

## AI stack

| Component | Purpose |
|---|---|
| **OpenCode** | Code generation, refactoring, code explanation, error resolution |
| **Hermes Agent** | Memory, patterns, reusable solutions, project templates |
| **Ollama** | Local model runtime |
| **Models** | Qwen and Gemma initially — small enough for 4 GB RAM |

All inference is local. No hosted model APIs.

### Operating modes

The droplet runs in one of two modes, because 4 GB will not hold both loads at once.

**Studio mode** — website, Studio, OpenCode, Hermes, Ollama and PostgreSQL running.
For development, code generation and testing.

**Build mode** — inference stopped, freeing memory and CPU for compiler builds,
Linux distribution packaging and application builds:

```bash
systemctl stop ollama
systemctl stop hermes
```

## Website structure

**Public area:** Home, Downloads, Documentation, Samples, Tutorials, Blog.

**Studio area:** Projects, IDE, Build, Run, AI Chat, Documentation Assistant.

### IDE

React. File explorer, code editor, Build button, Run button, output window,
compiler output, AI chat.

**The editor is not decided.** The intent is to adopt an open-source editor or IDE
and customise it for eTamil, which means the choice has to be made against eTamil's
own needs: Tamil script and the ezuqqu romanization in the same buffer, the `_`/`__`
script marks, a grammar we already generate from `lexer.rs` (the VS Code extension,
the tree-sitter grammar, and the Pygments, Rouge and Highlight.js lexers all derive
from it), and compiler diagnostics that carry line and column in Tamil.

Where it stands, 2026-09-22:

- **Recommendation on the table: code-server** — self-hosted Code-OSS, MIT, with the
  existing `etamil-support` VSIX sideloaded. It is the only candidate that runs what
  is already built. `eTamil_Code/src/extension.ts` registers completions, hover,
  signature help, go-to-definition and document symbols, all generated from
  `lexer.rs`, `parser.rs`, `interpreter.rs` and `nUlakam/` by
  `scripts/generate_editor_support.py` under a CI drift gate; diagnostics come from
  `etamil --check`, so there is no second parser to keep in step; the compiler travels
  inside the VSIX and `src/toolchain.ts` resolves it; a `linux-x64` VSIX is already
  built. It also has a real terminal, which is where a terminal-based agent such as
  OpenCode runs without a bridge being written for it. Argument against: it is the
  only option that costs the droplet memory — roughly 300–450 MB plus the extension
  host, competing directly with Ollama in Studio mode, which is what pushes the
  droplet from 4 GB to 8 GB.
- **Owner's preference: Monaco.** Recorded as the current leaning, not a decision.
  See [If we choose Monaco](#if-we-choose-monaco) for what it does and does not
  include, and what adopting it commits us to.
- **Fallbacks:** OpenVSCode Server (closer to upstream, no built-in auth),
  CodeMirror 6 (lightest, but our grammar is tree-sitter and CodeMirror wants Lezer),
  Eclipse Theia (fully rebrandable, but extension compatibility is not 1:1).

Nothing is final. To be settled in the Studio discussion.

#### If we choose Monaco

**Monaco is not a UI.** It is the editor widget extracted from VS Code — a text
editing surface that mounts in a `div`. It brings the editing experience: syntax
colouring, line numbers, gutter, minimap, folding, bracket matching, multi-cursor,
find and replace, the autocomplete dropdown, hover cards, signature help, error
squiggles with marker hovers, peek definition, a context menu, a command palette for
its own actions, and a side-by-side diff editor.

It brings none of the IDE around that: no file explorer, no editor tabs, no activity
bar, sidebar, status bar or panel, no terminal, no search across files, no source
control or debugger UI, no settings screen, and no concept of a project or workspace.
Files are "models" you create and swap into the editor yourself; tab state and
scroll position are yours to persist. Everything in the IDE list above — explorer,
Build and Run buttons, output window, AI chat, the layout that holds them — is ours
to build. That is the trade: total control of the UI, and we build all of it.

**Language support comes in three layers, and they are very different.**

| Layer | What Monaco ships |
|---|---|
| Colouring (Monarch grammars) | ~90 languages including JavaScript, TypeScript, JSX/TSX, Rust and Python. Regex-based; good enough to colour, not to understand |
| Real language service | Only TypeScript/JavaScript, JSON, HTML and CSS/SCSS/LESS. TS/JS is the actual TypeScript compiler in a web worker, so completions, type errors, rename and go-to-definition are real |
| Extensions | **None.** Monaco has no extension host, no marketplace, and cannot load a `.vsix`. VS Code extensions do not run in it |

So, concretely: **JavaScript and React work well.** JSX and TSX are handled by the
bundled TypeScript service — though it only knows about packages whose `.d.ts` files
we load into the worker via `addExtraLib`, so React types have to be supplied
deliberately. **Rust and Python get colouring only.** For real intelligence in those,
the pattern is to run the actual language server on the droplet — `rust-analyzer`,
Pyright — and bridge it to Monaco over a WebSocket with `monaco-languageclient`.

**What that means for eTamil.** eTamil is in neither of the first two rows, so
choosing Monaco commits us to three pieces of work:

1. A **Monarch grammar for eTamil**, which should be emitted by
   `scripts/generate_editor_support.py` alongside the TextMate grammar rather than
   hand-written — otherwise it becomes a fifth place the keyword list can drift.
2. An **eTamil language server** speaking LSP, wrapping `etamil --check` for
   diagnostics and the generated language data for completions, hover, symbols and
   definitions. None exists today; the VS Code extension does this in-process
   against VS Code's own APIs, so that code informs it but does not transfer.
3. The **IDE shell** described above.

The genuine upside, and the strongest argument for Monaco: an LSP server is a better
long-term asset than an extension. One server would serve Monaco, VS Code, Neovim,
Helix and Zed alike, and would replace the in-process half of `eTamil_Code` rather
than duplicating it. The cost is that it is real work standing between us and a
working Studio, where code-server has that work already done in a non-portable form.

#### Open questions before the editor is finalised

1. **How much IDE does the Studio actually need?** If the workflow is "describe it in
   English, read the generated code, press Run," a file tree, editor tabs and a
   terminal may be over-building. The lighter the answer, the stronger the case for
   Monaco or CodeMirror and the weaker the case for a full IDE shell.
2. **Is an eTamil language server MVP scope, or a follow-on?** It is the portable
   asset — one server would serve Monaco, VS Code, Neovim, Helix and Zed — but under
   Monaco it stands between us and a working Studio, whereas code-server has that
   work already done in a form that only VS Code can use. See the note below on
   `src/wasm.rs`, which changes what "from scratch" means here.
3. **How is the `_`/`__` script-mark font rendering done?** `eTamil_Code/src/fonts.ts`
   draws eTamil-script ASCII in the eTamil font and leaves English ASCII in the
   editor's own font, using VS Code decorations. Monaco and CodeMirror both have
   decoration APIs so it should port, but it needs checking rather than assuming —
   and in a browser the font must be served as a webfont, which makes
   `eTamil_Code/src/fontinstall.ts` (an OS-level font install) meaningless.

#### What already runs on etamil.in

Checked against the live site, 2026-09-22. This was not accounted for in the plan
above and it changes the starting position materially.

The editor at `etamil.in/start/` and on the language tour is **CodeMirror 6** — a
424 KB bundle at `/assets/ide/etamil-ide.js` — driving the **real compiler compiled
to WebAssembly** (`etamil-ide-etamil_compiler_bg.wasm`, 256 KB) through the bindings
in `etamil_compiler/src/wasm.rs`. Those bindings already export:

| Export | What it gives the editor |
|---|---|
| `diagnostics(source)` | The compiler's own bilingual errors, with 1-based line, column and length counted in characters |
| `symbols(source)`, `symbols_at(source, line, column)` | Scope-aware completions with kind and detail — what is visible from a position |
| `run(source)`, `run_with_input(source, input)` | Executes on the bytecode VM, in the browser |
| `version()` | Compiler version |

Those six are the whole surface. **Colouring is not among them** — it is done on the
JavaScript side by a CodeMirror `StreamParser` with the keyword vocabulary embedded in
the bundle, not by the compiler's lexer. Whether that vocabulary is generated at the
site's build time from `generate_editor_support.py`'s highlight.js output, or is a
hand-maintained copy, cannot be told from the served bundle and should be checked in
the website repository — a hand-maintained copy would be exactly the drift the
generator exists to prevent.

So the live editor has real diagnostics, scope-aware completion, hover tooltips,
colouring from the actual lexer, and Ctrl/Cmd-Enter to run — all client-side, with
nothing uploaded. Highlighting vocabulary is generated like everything else:
`scripts/generate_editor_support.py` emits the highlight.js definition explicitly
"for web documentation and the playground", alongside the TextMate grammar, the
Pygments and Rouge lexers, and `tree-sitter-etamil/keywords.js`.

What it does not have: multiple files or any project concept, a file explorer, tabs,
save or persistence (nothing is stored), a build step, and — since databases, the
HTTP server and `உள்ளிடு` need a real machine — the ability to run the kind of
application the Studio is meant to produce. `run` is called on the main thread with
no web worker, so a long-running program freezes the tab.

**Why this matters to the decision.** The analysis layer the Monaco route would
otherwise have to build is largely built, in a portable form, with the right shape of
API, and already proven against a browser editor. The gap between the live playground
and the Studio's editor is the shell and the project model, not the language
intelligence. It also costs the droplet nothing, because the compiler runs in the
visitor's browser rather than on the server — which is the opposite of code-server's
main drawback.

Two caveats found while checking: the module doc comment at the top of
`etamil_compiler/src/wasm.rs` still says "Only `lexer` -> `parser` -> `check` is
reachable from here", which `run` and `run_with_input` have since made untrue; and
`etamil --check`, which `eTamil_Code` depends on, is missing from
[COMMANDS.md](../reference/COMMANDS.md).

#### Dual mode — eTamil script and ISO, in both editors

"Dual mode" here means what `docs/reference/SCRIPT_RULES.md` specifies: a file holds
eTamil-script ASCII and English ASCII in the same byte range, the `_` and `__` marks
say which is which, and the editor draws the eTamil spans in the eTamil font while
leaving everything the marks call ISO in its ordinary Latin face.

**Tier 2 already has it.** `eTamil_Code/src/marks.ts` finds the spans that are eTamil
script — deliberately painting that side, so a span it misses renders eTamil as plain
Latin rather than rendering English as Tamil gibberish — and `src/fonts.ts` paints
them. Both `ican qamiz` faces travel in the VSIX, and `src/fontinstall.ts` puts one
where the OS can see it.

**Tier 1 does not.** Checked on the live site: there is no `@font-face` at all, and the
CodeMirror content font is a plain monospace stack with Unicode Tamil fallbacks
(`Noto Sans Tamil`, `Nirmala UI`, `Latha`). So Tamil *Unicode* renders correctly, but
ezuqqu ASCII draws as Latin — `ceyal` reads as `ceyal`, not செயல்.

**CodeMirror 6 can do it, and more directly than VS Code could.** `fonts.ts` documents
the VS Code obstacle: a theme cannot set a font family, so the font has to ride into
the page behind a decoration's `textDecoration` CSS — "a lever and not an API". CM6 has
no such limitation: `Decoration.mark` takes a class outright and a stylesheet supplies
the family and `font-feature-settings`. And in a browser the font is served as a
webfont, so the OS-install problem `fontinstall.ts` exists to solve does not arise.
Tier 1 is the easier of the two places to implement dual mode, not the harder.

Three practical risks:

1. **Metric compatibility.** The eTamil face and the base monospace face need matching
   advance widths, or columns drift between marked and unmarked spans. This matters
   more than cosmetics: diagnostics are positioned by character column and the gutter
   aligns to them.
2. **Contextual rules.** The Smart face carries `calt` rules for the pulli and vowel
   signs. Browsers enable `calt` by default, but site CSS must not disable it with
   `font-variant-ligatures: none`.
3. **Web embedding.** Check the font licence permits serving it as a woff2 before
   putting it on etamil.in.

**The architectural point: do not implement the mark rules a third time.** They are
normative and already implemented twice — `scripts/check_script_rules.py --check`
enforces them across the repository, `eTamil_Code/src/marks.ts` renders them for the
editor. A third copy in the browser bundle is a third thing that can disagree with the
specification. The fix that fits this project's existing discipline is a
`script_spans(source)` export in `etamil_compiler/src/wasm.rs`, beside `diagnostics`
and `symbols_at`, returning the spans from Rust so both browser and extension can
consume one implementation.

One caveat on that: the lexer skips comments (`#[logos(skip(r"//[^
]*"))]`), so
Rule 2 — English wrapped in `__ … __` inside a comment — is not derivable from the
token stream and needs its own small scan alongside it. The identifier side of Rule 1
is straightforward, since a leading `_` is already part of the token text.

##### A button to turn eTamil mode on and off

Wanted in both editors. Feasible in both, and it is a **view toggle, not a
conversion** — the bytes on disk are identical either way, only the face they are
drawn in changes. That is what makes it safe and instant.

**Tier 2: built, 2026-09-22.** `eTamil_Code/src/scriptMode.ts` — a status bar item
reading `eTamil` or `ISO`, shown only for eTamil files, which flips
`etamil.eTamilFont` between empty and the font stack. The command is
`etamil.toggleScriptFont`, also in the palette under `resourceLangId == etamil`.

It repaints nothing itself: `src/fonts.ts` already listens on
`onDidChangeConfiguration` for that key, so the button, the palette and hand-editing
`settings.json` all go down one path. Three things the implementation had to account
for:

- The setting is `scope: machine` — deliberately, because its value is interpolated
  into the decoration's CSS and a workspace should not choose it — so the write is
  `ConfigurationTarget.Global`. `Workspace` is refused, and the failure is reported
  rather than leaving a button that appears to do nothing.
- **The outgoing font stack is remembered** in `globalState`. Writing `''` would
  otherwise discard a stack the author had set by hand, and switching back on would
  silently substitute the bundled `ican qamiz`.
- Turning it on when the bundled font is not installed needs no new code: `fonts.ts`
  already offers `etamil.installFont` from its own configuration listener.

A status bar item, not an editor-title icon, because a toggle should show its state as
well as flip it. No unit test: the module imports `vscode`, which the `node --test`
harness cannot load — the same reason `fonts.ts` has none. `npm run verify` passes
(generated-data check, `tsc`, eslint, 128 tests).

**Tier 1 needs the dual-mode rendering first, but the toggle itself is trivial.** The
cheap shape is to compute the spans unconditionally and put a single class on the
CodeMirror root, letting CSS decide whether the eTamil face applies — no
recomputation on toggle, no `Compartment.reconfigure` needed. Persist the choice in
`localStorage`. Call `document.fonts.load()` before the first flip, or a cold webfont
makes the toggle flash.

The button belongs beside the controls already on the page — `▶ இயக்கு / Run` and
`⭳ பதிவிறக்கு / Download` — and should be labelled bilingually to match them.

**Decided, 2026-09-22: the rendering toggle only.** A button that *converts* the
buffer between Tamil script and ezuqqu (`எண்` ↔ `eN`) is explicitly not wanted. It
would be a much larger feature — rewriting the file, undoable, leaving string
literals and `_`-marked spans alone, and forcing a decision about which form is
canonical in version control. Not in scope. The toggle changes rendering and nothing
else; the bytes are untouched.

### Compiler integration

The Studio talks to the compiler over four endpoints:

```
POST /compile
POST /run
POST /build
POST /validate
```

## Knowledge repository

Held at `/knowledge`: language specification, framework documentation, compiler
documentation, compiler errors, sample applications, tutorials, best practices.

## Project templates

| Template | Entities |
|---|---|
| CRM | Customer, Lead, Invoice, Reports |
| Inventory | Products, Stock, Purchase, Sales, Reports |
| School | Students, Teachers, Attendance, Fees, Reports |
| Hospital | Patients, Doctors, Appointments, Billing |

## Workflows

**Create an application.** Input: *"Create a School Management System."*

```
OpenCode → template discovery → eTamil generation → compile → error resolution → build
```

**Generate documentation.** Input: *"Generate documentation."* Output: installation
guide, user guide, API guide, developer guide.

## Deployment

All five sites on the same droplet, behind an Nginx reverse proxy:

| Domain | Serves |
|---|---|
| `etamil.in` | Studio and the public site |
| `kelir.org` | Demo app 1 |
| `qos.ae` | Demo app 2 |
| `conf.ae` | Demo app 3 |
| `ineo.in` | Demo app 4 |

## Success criteria

The MVP succeeds when it can host `etamil.in` and the online IDE; generate eTamil code
with AI; compile, run and host applications; build and host Linux compiler releases;
use local models only; and do all of it on one 4 GB / 2 vCPU droplet.

## Deliverables

1. `etamil.in` website
2. Documentation portal
3. Compiler download portal
4. eTamil Studio
5. Editor integration — the editor itself to be chosen (see [IDE](#ide))
6. OpenCode integration
7. Hermes integration
8. Ollama integration
9. PostgreSQL integration
10. Linux compiler build pipeline
11. Project templates
12. AI-assisted development
13. Demo application hosting

---

## Measured on the droplet, 2026-09-25

Everything above this heading was written before anything was built. This
section is what a 4 GB / 2 vCPU droplet running Ubuntu 24.04 actually does,
measured rather than estimated. `201.79.9.90`, provisioned with swap, UFW,
fail2ban, unattended-upgrades, Ollama and the 1.1.0 compiler.

### What the parts cost

| component | measured |
|---|---|
| OS, sshd, fail2ban, unattended-upgrades | ~450 MB |
| Ollama with `qwen2.5:3b` loaded | **2,090 MB** (`llama-server` RSS) |
| Ollama with `qwen2.5-coder:1.5b` | ~1,090 MB peak during generation |
| **one eTamil HTTP server** | **3.5 MB**, unchanged after serving requests |
| four demo apps | ~14 MB |

The estimate this replaces guessed 50-100 MB per demo app. The real figure is
3.5 MB, so the four applications this document is built around cost less than a
rounding error. **The model is the entire memory question**; everything else
fits in about 700 MB.

So 4 GB holds the MVP as scoped: ~2.8 GB used with a 3B model, ~1.7 GB with the
1.5B one. The two-mode split is still needed for *builds* -- a `cargo build
--release` wants 1-1.5 GB and collides with a resident 3B model -- but nothing
else requires stopping inference.

### Swap is insurance, not capacity

`qwen2.5:7b` is 4.7 GB against ~1.3 GB free, so it must page. It was run once,
deliberately:

    888 seconds, 18 tokens, 0.02 tokens/sec, swap peak 2,141 MB

Fifteen minutes for eighteen tokens. Inference reads the whole weight set for
every token, so a model that overflows RAM does not degrade gracefully -- it
falls off a cliff. Keep the 4 GiB swap for what it is already doing, absorbing
cold pages so a spike does not reach the OOM killer, and treat the model size
as bounded by RAM. No amount of swap changes this.

### Can a local model write eTamil? Not yet

The workflow this document opens with -- English requirement, AI assistant,
eTamil source, compiler, working application -- was tested directly. Six small
tasks, generated by a local model given eTamil in its prompt, each answer put
through `etamil --check` and then *run* and compared against the expected
output.

| model | compiled | correct |
|---|---|---|
| `llama3.2:3b` | 2/6 | **2/6** |
| `qwen2.5-coder:1.5b` | 4/6 | 1/6 |
| `qwen2.5:3b` | 4/6 | 0/6 |
| `qwen2.5-coder:3b` | 1/6 | 0/6 |
| `gemma2:2b` | 0/6 | 0/6 |

The tasks were: count down from 5, cube a number, find the largest in an array,
double each element, three-term interest, and call one function twice. All six
were verified solvable first -- a reference solution for each was compiled and
run -- so a zero is a model failing, not a task that cannot be expressed.

Four findings, in the order they matter:

**Nothing composes.** No model solved the largest-in-an-array task (a
conditional inside a loop) or the call-a-function-twice task. Those are the two
that cannot be reached by pattern-matching a single example, and they are the
nearest thing here to what a Studio would be asked for.

**Code-specialised models do worse than general ones of the same size.**
`qwen2.5-coder:3b` compiled 1/6 where `qwen2.5:3b` compiled 4/6 -- same family,
same size, same prompt. Heavy Python and JavaScript training appears to bias
against following an unseen grammar. The AI stack section above names "Qwen and
Gemma" without distinguishing; it should say general instruct models, and
`gemma2:2b` should be struck -- it produced nothing compilable in 18 attempts.

**Feeding compiler errors back does not help.** A repair loop of three attempts
was tried on both models: **zero successful repairs in 18 attempts**. Every
program that compiled did so first time. eTamil reports an exact line and
column, which is unusually good material for this, and the models could not act
on it.

**Context helps, and it is the only thing that did.** Going from one worked
example to 109 reserved keywords plus 14 compiled examples was what produced the
first correct program at all. Two cautions learned the hard way: every example
must be compiled before it goes in the prompt, and the test tasks must not be
answerable by copying one -- a first attempt scored 3/6 that turned out to be
the prompt's own examples handed back.

### What follows from this

The infrastructure is fine and the premise is not. The editor at
`etamil.in/start/` already does the parts that work -- real diagnostics,
scope-aware completion, running programs -- and none of that depends on a
model. What does not work at 4 GB is English to working eTamil, at best 2 in 6
on tasks far below "create a School Management System".

Three ways forward, none of them free: ship the Studio without generation;
accept a human-in-the-loop flow where the model drafts and a person corrects;
or revisit the constraint that inference is local and free, which is a decision
about the project rather than about this droplet.

The harnesses are on the droplet under `/root/` -- `gentest.py`, `repairtest.py`,
`richtest.py`, `disjoint.py`, `context.txt` -- so any of this can be re-measured
against a new model with one command.

## Where this document and the repository disagree

Recorded so the gaps are visible, not to overrule the document. Each is a thing to
settle before building against it.

**"The compiler, runtime, framework, documentation and sample applications already
exist and are production-ready."** The compiler, runtime and standard library exist
and are tested. But [ROADMAP.md](../ROADMAP.md) records that no pilot is deployed
against a real product, the banking domain module is not started, there is no REPL,
and the LLVM backend still refuses I/O statements. "Exists and is usable" is
defensible; "production-ready" is ahead of the evidence.

**"Existing compiler APIs: `POST /compile`, `/run`, `/build`, `/validate`."** These do
not exist. The compiler is a CLI — see [COMMANDS.md](../reference/COMMANDS.md). Its
`--server` flag runs an *eTamil program* as an HTTP server; it does not expose
compilation over HTTP. A compile/run service is something the Studio would have to
build and sandbox, and it is the largest unscoped item in this document.

**4 GB RAM for the whole stack.** The two-mode split acknowledges the constraint, but
a Rust compiler build with the LLVM feature is itself demanding on 2 vCPU / 4 GB, and
PostgreSQL plus Nginx plus four demo applications have to stay resident in build mode.
Worth measuring before committing to the droplet size.

**Running untrusted generated code on the host that serves five production domains.**
The document does not say how compile-and-run is isolated. This needs an answer before
the first user-supplied program runs.
