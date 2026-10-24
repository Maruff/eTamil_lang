# Publishing tree-sitter-etamil

Nothing here has been published. This is what has been checked, and what only you can do (it needs your npm and crates.io logins).

## Where this grammar lives

package.json, Cargo.toml and the docs point at a standalone repository, `github.com/Maruff/tree-sitter-etamil`. Today the grammar is the `tree-sitter-etamil/` folder of the compiler repository. Two things need a decision:

- **A standalone repository.** nvim-treesitter and Helix install from a repository URL. Either create `Maruff/tree-sitter-etamil` and push this folder to its root (a subtree split keeps history: `git subtree split -P tree-sitter-etamil`), or point those entries at the compiler repository with a `location` of `tree-sitter-etamil`. The standalone repository matches the MIT licence this grammar deliberately has, separate from the compiler's AGPL.
- **The version.** It is still `0.4.0` in package.json, tree-sitter.json and Cargo.toml, and nothing at that version was ever published, so you may publish `0.4.0` as is or bump all three together (`0.5.0` would signal the queries and bindings).

## Checked

| Target | Check | Result |
|---|---|---|
| crates.io | `cargo publish --dry-run --allow-dirty` | packages 20 files, 90 KiB, rebuilds from the package; name `tree-sitter-etamil` is free |
| crates.io | `cargo test` | 3 tests and the doc test pass, including all four query files compiling |
| npm | `npm test` | corpus 41/41, tags 2 assertions, keyword coverage 56/56, Node binding 5/5 |
| npm | name `tree-sitter-etamil` | free (404 on the registry) |

## Not checked

- The npm package has no prebuilt binaries. `npm install` compiles the addon, so users need a C toolchain and Python. Publishing prebuilds means a CI matrix with `prebuildify` on Linux, macOS and Windows; not done.
- Linux and macOS builds of the Node and Rust bindings (only Windows was built here). CI runs them on Ubuntu.
- `npm publish --dry-run` was not run; check `files` in package.json against `npm pack --dry-run`.
- Neovim, Helix and Zed were not run.

## Steps

1. Decide the repository and version (above), commit, and let CI pass.
2. crates.io: `cargo login`, then `cargo publish` from this folder.
3. npm: `npm login`, then `npm publish --access public` from this folder.
4. Tag the release.

## Submitting upstream

**nvim-treesitter.** Add the parser to its `parsers.lua` (the shape depends on its current branch; read its `CONTRIBUTING.md` first):

```lua
etamil = {
  install_info = {
    url = 'https://github.com/Maruff/tree-sitter-etamil',
    files = { 'src/parser.c' },
    branch = 'main',
  },
  filetype = 'etamil',
  maintainers = { '@Maruff' },
}
```

It also wants the four query files under `runtime/queries/etamil/`, and a filetype for `.qmz` (`vim.filetype.add({ extension = { qmz = 'etamil' } })`, or an upstream filetype entry). A grammar in a subfolder needs a `location = 'tree-sitter-etamil'` entry.

**tree-sitter's list of parsers.** Add `eTamil` to the list in the tree-sitter wiki or documentation, linking the repository.

**GitHub code navigation and Linguist.** Both need the language recognised first; see `etamil-tmlanguage/LINGUIST-SUBMISSION.md`, whose open item is public usage of `.qmz`.
