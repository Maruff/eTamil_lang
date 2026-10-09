-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
--
-- eTamil for Neovim.
--
--   require("etamil").setup()
--
-- Registers the tree-sitter grammar with nvim-treesitter (when it is installed) and the
-- language server `etamil-lsp`. File type detection and buffer settings need no setup:
-- they come from ftdetect/ and ftplugin/ when the plugin is on the runtimepath.

local M = {}

M.defaults = {
  -- Start the language server for eTamil buffers, when `cmd` is on the PATH.
  lsp = true,
  -- The command that starts it.
  cmd = { "etamil-lsp" },
  -- Tell nvim-treesitter where the grammar is, so :TSInstall etamil works.
  treesitter = true,
}

-- Where the grammar lives: a folder of the compiler's repository, not a repository of
-- its own, which nvim-treesitter supports through `location`.
M.grammar = {
  url = "https://github.com/Maruff/eTamil_lang",
  location = "tree-sitter-etamil",
  files = { "src/parser.c" },
  branch = "main",
}

--- The configuration a language server client needs.
function M.lsp_config(opts)
  opts = vim.tbl_deep_extend("force", M.defaults, opts or {})
  return {
    name = "etamil",
    cmd = opts.cmd,
    filetypes = { "etamil" },
    root_markers = { ".git" },
  }
end

local function register_parser()
  local parser_config = {
    install_info = vim.deepcopy(M.grammar),
    filetype = "etamil",
  }

  -- nvim-treesitter's rewrite reads parsers from a table it rebuilds on every update,
  -- so the entry is added again each time it announces one.
  vim.api.nvim_create_autocmd("User", {
    pattern = "TSUpdate",
    group = vim.api.nvim_create_augroup("etamil_treesitter", { clear = true }),
    callback = function()
      local ok, parsers = pcall(require, "nvim-treesitter.parsers")
      if ok then
        parsers.etamil = parser_config
      end
    end,
  })

  -- The original nvim-treesitter keeps a table of configurations behind a function.
  local ok, parsers = pcall(require, "nvim-treesitter.parsers")
  if ok and type(parsers.get_parser_configs) == "function" then
    parsers.get_parser_configs().etamil = parser_config
  elseif ok then
    parsers.etamil = parser_config
  end

  vim.treesitter.language.register("etamil", "etamil")
end

local function register_lsp(opts)
  local config = M.lsp_config(opts)

  -- Without the server there is nothing to start, and trying would put an error on
  -- every eTamil buffer. :checkhealth etamil says why it is not running.
  if vim.fn.executable(config.cmd[1]) == 0 then
    return false
  end

  -- Neovim 0.11 has a registry of servers and starts them for the file types listed.
  if vim.lsp.config ~= nil and vim.lsp.enable ~= nil then
    vim.lsp.config["etamil"] = {
      cmd = config.cmd,
      filetypes = config.filetypes,
      root_markers = config.root_markers,
    }
    vim.lsp.enable("etamil")
    return true
  end

  -- Before that, start it by hand when an eTamil buffer is opened.
  vim.api.nvim_create_autocmd("FileType", {
    pattern = "etamil",
    group = vim.api.nvim_create_augroup("etamil_lsp", { clear = true }),
    callback = function(args)
      local found = vim.fs.find(config.root_markers, {
        upward = true,
        path = vim.api.nvim_buf_get_name(args.buf),
      })[1]
      vim.lsp.start({
        name = config.name,
        cmd = config.cmd,
        root_dir = found and vim.fs.dirname(found) or vim.loop.cwd(),
      })
    end,
  })
  return true
end

function M.setup(opts)
  opts = vim.tbl_deep_extend("force", M.defaults, opts or {})
  if opts.treesitter then
    register_parser()
  end
  if opts.lsp then
    return register_lsp(opts)
  end
  return false
end

return M
