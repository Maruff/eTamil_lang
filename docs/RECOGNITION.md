# Recognition as a programming language

What it takes for eTamil to be *listed* as a programming language rather than
described as one, which of it is done, and which of it cannot be hurried.

Last updated: 2026-09-06.

## The shape of the problem

Recognition is not one gate. It is a set of registries and indexes, and they
divide cleanly into two kinds:

- **Gated on work.** A package registry, a syntax-highlighting grammar, a
  citable identifier. Nobody has to agree that eTamil matters; the entry
  appears because the work was done. These are the ones to finish.
- **Gated on adoption or on other people writing about the language.** GitHub
  Linguist, Wikipedia, TIOBE. No amount of engineering substitutes, and the
  usual ways of faking the inputs — repositories created to inflate a file
  count, citations to one's own unrefereed pages — are both detectable and
  disqualifying.

The second kind is downstream of the first, so the order below is not
arbitrary.

## Dependency chain for the gated items

```
 packages + grammars + a DOI          real users writing .qmz
            │                                   │
            ▼                                   ▼
  independent write-ups                GitHub Linguist
            │                          (2,000 files across
            ▼                           many distinct owners)
   Wikipedia article
   (independent, reliable
    secondary sources)
            │
            ▼
      TIOBE index
  (needs the Wikipedia entry,
   Turing completeness, and
   5,000 hits for
   +"eTamil programming")
```

TIOBE's three criteria are quoted from its own
[definition page](https://www.tiobe.com/tiobe-index/programming-languages-definition/),
checked 2026-09-06. The Wikipedia entry is a hard prerequisite there, which is
why it sits upstream.

**Ezhil is the precedent.** The other Tamil programming language went
arXiv paper → Wikipedia article → general recognition. It is worth reading its
Wikipedia entry for what its sources actually are, because that is the standard
eTamil's would be held to.

## Status

### Done

| | |
|---|---|
| TextMate grammar, MIT, in its own repository | `Maruff/etamil-tmlanguage` — separate because Linguist's grammar licence list excludes AGPL |
| Linguist entry, samples and PR steps drafted | `etamil-tmlanguage/LINGUIST-SUBMISSION.md` |
| Website, manual, keyword reference | <https://etamil.in> |
| Prebuilt packages with installers | `packaging/`, GitHub Releases |
| Design paper written | `docs/reference/ETAMIL_STANDARD.md` |
| **Standard library travels inside the binary** | `build.rs`, `src/stdlib.rs` — see below |
| **Crate publishable to crates.io** | metadata, `include`, `scripts/vendor_for_publish.py` |
| **Citable** | `CITATION.cff` |
| **Pygments lexer** | `eTamil_Pygments/` — Sphinx, MkDocs, Jupyter and most Python doc pipelines |
| **Rouge lexer** | `eTamil_Rouge/` — what Jekyll and GitHub Pages use, so it highlights etamil.in itself |
| **highlight.js definition** | `eTamil_HighlightJS/` — web docs and the browser playground; 9 tests |
| **tree-sitter grammar** | `tree-sitter-etamil/` — Neovim, Helix, Zed and GitHub code navigation |

All four highlighters take their vocabulary from the same place the VS Code
grammar does — the compiler's own token table, via
`scripts/generate_editor_support.py` — so none of them can describe a language
the compiler does not accept. Only the *structure* of `tree-sitter-etamil/
grammar.js` is hand-written, because the shape of a statement is not
recoverable from a list of tokens. It parses all 41 standard library modules
and all 28 examples with no errors, which is what found the four constructs no
hand-written test happened to cover: multi-line string literals, the database
statements, `ஜேசான்_உரை` and `இடைவெளி`.

The standard library one was load-bearing and is worth spelling out. Every
lookup in `module::locate` needs someone to have *placed* `nUlakam/`
somewhere, and the installers in `packaging/` do exactly that. A package
manager does not: `cargo install`, `pip install`, `brew install` and a bare
`winget` drop an executable on the PATH and nothing else. Installed that way,
with `ETAMIL_PATH` unset, the compiler answered

```
✗ தொகுதி 'nUlakam/paNam/paNam.qmz' கண்டுபிடிக்க முடியவில்லை
```

to the first line of the README's own money example. Every package-registry
route was therefore blocked on it — a language whose standard library
disappears when installed the ordinary way cannot honestly be listed as
installable. The library is now compiled into the binary and tried last, so
anything on disk still overrides it. `tests/installed_binary.rs` runs the real
binary from a scratch directory with `ETAMIL_PATH` removed, which is the only
arrangement that can tell the difference.

### Ready, needs an account or a decision

| Step | Blocked on |
|---|---|
| Publish to crates.io | a crates.io token. `cargo package` verifies clean, 96 files, all 41 stdlib modules included |
| Publish the VS Code extension | a Marketplace publisher (`vsce`) |
| Publish to Open VSX | an Eclipse Foundation account — this is what VSCodium, Cursor and Windsurf install from |
| Zenodo DOI | linking the repository at zenodo.org, then cutting a release. `CITATION.cff` is already in place for it to read |
| PyPI installer package | a PyPI token — an account exists |

### Not started, no gate

| Step | Why it is worth doing |
|---|---|
| PLDB entry | <https://pldb.io> — a language database that accepts submissions by pull request, with no adoption threshold |
| Wikidata item | Structured, factual, low bar; feeds other tools. No Wikidata entity for eTamil exists as of 2026-09-06 |
| Rosetta Code tasks | Doubles as adoption: every task is a real `.qmz` file, which is the Linguist currency |
| Homebrew / Scoop / winget manifests | Distribution, and each is an indexed listing in its own right |

### Blocked on things that are not engineering

**GitHub Linguist.** The threshold is 2,000 `.qmz` files indexed in the last
year, spread across many distinct `:user/:repo` owners, with forks not
counting. As of the last check that query returned 80 files in one repository.
The entry, the grammar, the samples and the five PR steps are all prepared and
waiting in `etamil-tmlanguage/LINGUIST-SUBMISSION.md`. Filing early gets the PR
closed and makes the next attempt harder, so it waits.

The honest route to it is people writing eTamil. Rosetta Code tasks, worked
examples in the manual that readers copy into their own repositories, and the
`vvari` ITR application are what move that number.

**Wikipedia.** Needs significant coverage in independent, reliable secondary
sources — not the project's own site, repository or self-published paper. One
peer-reviewed publication plus press coverage is the realistic bar. Writing the
article before the sources exist gets it deleted, and a deletion makes a later
attempt harder.

**TIOBE.** Downstream of Wikipedia; nothing to do until that exists.

## The next thing that actually moves this

Get the design paper a **DOI and a permanent home**. It is the input every
gated item eventually needs, and it is the only one where the work is already
finished — the paper exists.

- **Zenodo** mints a DOI with no gate and no review. Cheapest real identifier.
- **arXiv** (`cs.PL`) is more visible and is the route Ezhil took, but a first
  submission to `cs` needs endorsement from an existing arXiv author. A
  supervisor with arXiv history can endorse.
- **A refereed venue** is what Wikipedia actually wants. The paper names an
  institutional affiliation and a supervisor, so this is the normal path
  rather than an obstacle.

Everything else on the "no gate" list can be done in any order, but none of it
substitutes for this one.
