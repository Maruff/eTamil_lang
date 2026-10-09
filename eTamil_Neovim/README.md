# eTamil for Neovim

File type detection, buffer settings, tree-sitter highlighting and the language server
for eTamil (`.qmz`) in Neovim 0.9 or newer.

**What is checked:** CI runs the plugin's tests under Neovim 0.10 and the current stable
release. They cover file type detection, the buffer settings, the language server's
registration, and the highlight query against the real grammar, built in the job.
**What is not checked:** a real editing session, `:TSInstall etamil` through
nvim-treesitter, and the language server started by Neovim. Those follow the documented
APIs of Neovim and nvim-treesitter, and the first person to use them will find out.
Please report what breaks.

## Install

With lazy.nvim:

```lua
{
  "Maruff/eTamil_lang",
  -- The plugin is a folder of the repository, not the repository's root.
  config = function(plugin)
    vim.opt.runtimepath:append(plugin.dir .. "/eTamil_Neovim")
    require("etamil").setup()
  end,
}
```

Or by hand, from a checkout of the repository:

```lua
vim.opt.runtimepath:append("/path/to/eTamil_lang/eTamil_Neovim")
require("etamil").setup()
```

File type detection and the buffer settings work as soon as the folder is on the
runtimepath. `setup()` adds the grammar to nvim-treesitter and starts the language server.

## Highlighting

With nvim-treesitter, after `setup()`:

```vim
:TSInstall etamil
```

The grammar is a folder of this repository (`tree-sitter-etamil`), which nvim-treesitter
fetches and compiles; it needs a C compiler. The highlight queries ship with the plugin, in
`queries/etamil/highlights.scm`. They are generated from the compiler's lexer by
`scripts/generate_editor_support.py`, as the other editors' keyword lists are, and a CI
check fails when they drift. **Do not edit that file by hand.**

## Language server

`etamil-lsp` comes with the eTamil releases. `setup()` starts it for eTamil buffers when it is
on the PATH, and does nothing, without an error, when it is not.

```lua
require("etamil").setup({
  cmd = { "/opt/etamil/etamil-lsp" },   -- where it is, if not on the PATH
  lsp = true,                            -- false to leave the server to you
  treesitter = true,                     -- false to leave the grammar to you
})
```

## Check it

```vim
:checkhealth etamil
```

says whether the file type is detected, the server is on the PATH and the grammar is installed.

## Run the tests

From the repository root:

```bash
nvim --headless -u NONE -l eTamil_Neovim/tests/run.lua
```

To include the highlight query test, build the grammar and name it:

```bash
cc -shared -fPIC -O1 -I tree-sitter-etamil/src tree-sitter-etamil/src/parser.c -o /tmp/etamil.so
ETAMIL_PARSER=/tmp/etamil.so nvim --headless -u NONE -l eTamil_Neovim/tests/run.lua
```
