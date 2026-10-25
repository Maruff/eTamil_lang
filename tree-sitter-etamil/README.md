# tree-sitter-etamil

A [tree-sitter](https://tree-sitter.github.io) grammar for
**[eTamil](https://etamil.in)** — a programming language whose vocabulary is
Tamil, aimed at Indian accounting, tax and FinTech.

This is what Neovim, Helix and Zed parse with, and what GitHub uses for code
navigation.

```etamil
எண் வருவாய் = 950000;
வரி_வீதம் = 20%;

(வருவாய் > 800000) எனில் {
    அச்சு "Tax: " & வட்டமிடு((வருவாய் - 800000) * வரி_வீதம், 2);
}
```

## Status

Parses **69 of 69** real eTamil programs in the compiler repository — all 41
standard library modules and all 28 examples — with no errors, plus 41 corpus
tests pinning individual constructs. Every keyword the grammar uses appears in a
corpus case, and `node test/keyword-coverage.mjs --check` fails if one does not.

```bash
npm install
npm test              # corpus and tags tests, keyword coverage, Node binding
npm run parse:corpus  # every real program in the repository
```

## The vocabulary is generated

`keywords.js` holds every spelling the compiler accepts for each keyword —
Tamil script, the romanized form, and where one exists an English alias — and
is generated from the compiler's own token table:

```bash
python ../scripts/generate_editor_support.py
```

So the grammar cannot accept a spelling the compiler rejects, or miss one it
accepts. `queries/highlights.scm` is generated the same way, and CI fails if
either drifts.

`grammar.js` is **hand-written**, because the shape of a statement is not
recoverable from a list of tokens. Two shapes are worth knowing:

- **The condition comes before the keyword.** `(வருவாய் > 800000) எனில் {` —
  not `எனில் (…)`. Editor integrations that assume the C ordering silently
  never fire.
- **A name is stored exactly as written.** `{வரி: 1}` and `{vari: 1}` are
  different fields. Field names are data, not syntax, so `queries/` captures
  them as `@property` rather than as keywords.

One more, in the queries: the financial vocabulary — `வரவு`, `பற்று`, `தொகை` —
is deliberately **not** reserved. The parser accepts those as ordinary names,
so they are captured as `@variable.member`, not `@keyword`. Colouring them as
syntax would tell the reader the opposite of the truth.

## Queries

| File | For |
|---|---|
| `queries/highlights.scm` | colouring (generated from the compiler's keyword tables) |
| `queries/locals.scm` | scopes and bindings, for rename and reference highlighting. Only a function or lambda opens a scope: eTamil has no block-scoped binding form. |
| `queries/tags.scm` | function and shape definitions and calls, for symbol outlines; the `//` comment above a function is its documentation |
| `queries/injections.scm` | SQL inside the database statements, and comments |

`locals.scm` uses the plain `@local.definition` capture rather than a kind suffix,
because tree-sitter's own tags loader rejects `@local.definition.function`.
`injections.scm` and `tags.scm` use the editor-level predicates `#offset!`,
`#strip!` and `#select-adjacent!`: Neovim, Helix, Zed, the CLI and the Rust crate
understand them, but the Node binding does not, so its tests check only
`highlights.scm` and `locals.scm`.

## Bindings

C, Node and Rust. **Not published yet**, so for now install from a checkout.

- **Rust:** `cargo test` runs the binding's tests, including that all four query
  files compile; `cargo publish --dry-run` checks the crate packages and rebuilds.
- **Node:** `npm install` builds the addon with node-gyp, so it needs a C
  toolchain and Python. On Windows the build must run MSVC in UTF-8 mode
  (`binding.gyp` does this): the keywords are Tamil, and without it every Tamil
  node name becomes `?`. A test pins that.
- **C:** `make` builds the library; `bindings/c/` has the header and pkg-config file.

Go, Python and Swift bindings are switched off in `tree-sitter.json` because they
are not tested.

## Ambiguity

`வரிசை[0]` begins an index assignment if a `=` follows and is an index
expression otherwise, and one token of lookahead cannot tell. The compiler's
recursive-descent parser decides by peeking; tree-sitter is GLR, so the grammar
declares the ambiguity in `conflicts` and lets the parser carry both readings
until the `=` settles it.

## Editor setup

**Neovim** (nvim-treesitter), until this is upstreamed:

```lua
require('nvim-treesitter.parsers').get_parser_configs().etamil = {
  install_info = {
    url = 'https://github.com/Maruff/tree-sitter-etamil',
    files = { 'src/parser.c' },
    branch = 'main',
  },
  filetype = 'etamil',
}
vim.filetype.add({ extension = { qmz = 'etamil', etamil = 'etamil' } })
```

Then `:TSInstall etamil`, and copy the four `queries/*.scm` files to
`queries/etamil/` in your config.

## Licence

MIT — deliberately, and separately from the compiler's AGPL. Grammar consumers
(nvim-treesitter, Helix, Zed, GitHub Linguist) all expect a permissive licence,
and the same reasoning put the TextMate grammar in its own MIT repository.
