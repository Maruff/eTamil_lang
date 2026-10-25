# eTamil Studio: what to build first

Status: **a proposal for the owner to approve**, written 2026-10-04. It reconciles
two documents that use the word "Studio" for different products, and picks the first
piece of work. Nothing here is a commitment beyond Phase 0.

## Two documents, two products

| | [MVP.md](MVP.md) (v1.1, September) | The work breakdown, group 2 ("eTamil Studio (Visual IDE)") |
|---|---|---|
| What it is | An **online IDE with an AI assistant**: English requirement in, eTamil source out, compiled and run | A **low-code platform**: drag-and-drop forms, dashboards, workflows, rules, a data modeller, an API designer, reports, role management, a code generator, one-click deployment |
| Size | One developer, one 4 GB droplet | Eleven modules; each is a product feature |
| Exists today | A browser editor (below); nothing else | Nothing |
| Hardest unsolved part | A compile-and-run service, and isolating untrusted code from the droplet that serves the production domains | Deciding what a form or a workflow *compiles to*, and where its data lives |

Both need the same substrate: **an editor that talks to the real compiler.** Neither
can start without it, and one of them already exists.

## What exists

- **A browser editor** at `etamil.in/start` (CodeMirror 6, source in the site's `ide/`
  folder) runs the **real compiler front end as WebAssembly**: the compiler's own
  bilingual diagnostics, scope-aware completion of names in the file, keyword
  completion, and running a program on the VM, all in the page. Samples, a download
  button, a Tamil key row and the script switch are there too.
- **The language server** (`etamil-lsp`) now completes keywords, builtins and
  standard-library functions and documents them on hover, from a vocabulary file the
  compiler's generator writes (`etamil_lsp/data/language-data.json`).
- **Not there:** a compile-and-run service (the compiler is a CLI; `--server` runs an
  eTamil *program* as a web server, it does not compile on request), any isolation for
  running untrusted code, any AI assistant, saved projects, accounts.

## A constraint that shapes Phase 0

The browser build cannot import anything. `module.rs`, which reads imported files, is
compiled out of the wasm build (`lib.rs`), along with databases, sockets and files. So
**the standard library (`nUlakam/`) does not run in the browser**: an `இறக்கு` fails
at run time with "not implemented in the VM yet". The browser editor can
honestly offer keywords, statement templates and the host builtins, and should say
so for library functions rather than offer something that cannot run.

## Proposal: Phase 0 extends the editor that is already live

Add to the existing browser editor, **entirely in the browser**:

1. **Builtin-function completion and hover**, and **statement templates** for
   keywords, from the same generated vocabulary the language server uses, so the web
   editor and the desktop editors teach the same language. Library functions are
   documented but marked as needing the desktop compiler.
2. **A share link:** the program in the URL, so a snippet can be sent in a message.
3. **Autosave and restore** in the browser, so closing the tab loses nothing.

**Why this first.**

- It serves **both** meanings of Studio, since both need this editor.
- Everything runs client-side, so it needs **no sandbox, no new service and nothing on
  the droplet**. The two risks MVP.md itself records (an unscoped compile service, and
  untrusted code beside production) are not touched.
- It builds on assets that now exist, and on a site that is already deployed.
- It is small and reversible.

**Deliberately not in Phase 0:** the compile-and-run service, the AI assistant, and any
low-code module. The reasons are below.

## Decisions only the owner can make

These block anything beyond Phase 0.

1. **Which Studio is the product, and in what order.** The online IDE with an
   assistant, the low-code platform, or the IDE first and the low-code platform as a
   generator on top of it. The last is the cheapest path and the one this note assumes
   in the phases below; it is a recommendation, not a decision already taken.
2. **How untrusted programs run on a server, if they do.** Options: stay browser-only
   (the VM already runs in wasm, with a step limit); a separate machine or container
   with no route to production; or a stronger sandbox. MVP.md notes the question is
   unanswered. Running generated code on the host that serves five production domains
   should not happen until it is.
3. **The AI assistant's model.** MVP.md's own measurement found a local model cannot
   write eTamil yet, so this means a hosted model: a cost, a privacy and a key-handling
   decision.
4. **For low-code: what a form or workflow compiles to**, and where its data lives.
   The demo applications are an eTamil backend with a React frontend over PostgreSQL;
   a visual designer would presumably emit the same, but that has not been chosen.
5. **Accounts.** Saving projects across devices, role management (WBS 2.7), and
   anything multi-user need sign-in, which nothing has today.

## Decisions taken, 2026-10-11

The owner accepted the recommendation on each of the five.

| # | Decision | Taken |
|---|---|---|
| 1 | Which Studio, and in what order | **The IDE first, with low-code built on top as a generator.** |
| 2 | Untrusted programs on a server | **Browser only.** The VM runs in the page under a step limit, and since `run_project` it runs whole projects and the standard library. A server-side run is not planned; nothing visitors write runs on the droplet that serves the production domains. |
| 3 | The AI assistant's model | **No generation for now.** Later, as an opt-in and clearly experimental feature, **bring your own key**: the user's own provider key, kept in their browser, with the page running the compiler's check on what comes back. No hosted model, no key held by the project. |
| 4 | What a form compiles to | **A first slice that generates an eTamil program and an HTML form, running in the browser, with no storage.** A generated backend over PostgreSQL stays the destination, after the slice works. |
| 5 | Accounts | **None.** Projects stay in the browser, with export and import. GitHub sign-in, with projects in the user's own GitHub, is the later option. |

What follows from them: Phase 4's form builder starts, as a browser-only first slice (a form with fields,
calculations and checks, whose calculations are real eTamil run by the real VM), and Phases 2 and 3 as
written here do not start.

## Later phases, if the decisions go that way

| Phase | What | Entry condition |
|---|---|---|
| 1 | Named projects with several files, kept in the browser; export and import | Phase 0 shipped |
| 2 | Run on a server: a compile-and-run service on an isolated host | Decision 2 |
| 3 | AI assistant: English to eTamil, explain, fix an error | Decision 3 and Phase 2, or a browser-only variant |
| 4 | One low-code module end to end (a form builder that emits an eTamil backend and a React form) | Decisions 1 and 4 |

Phase 4 is the first WBS module worth doing, because a form builder exercises the
whole chain (design, generate, compile, run, deploy) at the smallest size.

## Where the two documents still disagree

MVP.md lists compile and run endpoints (`POST /compile`, `/run`) among "existing
compiler APIs". They do not exist; see its own section on where it and the repository
disagree. This note changes nothing in MVP.md.
