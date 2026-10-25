-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
--
-- :checkhealth etamil

local M = {}

-- The names changed in Neovim 0.10.
local health = vim.health
local start = health.start or health.report_start
local ok = health.ok or health.report_ok
local warn = health.warn or health.report_warn
local info = health.info or health.report_info

function M.check()
  start("etamil")

  if vim.fn.has("nvim-0.9") == 1 then
    ok("Neovim " .. tostring(vim.version()))
  else
    warn("Neovim 0.9 or newer is needed for tree-sitter highlighting and :checkhealth")
  end

  if vim.filetype.match({ filename = "x.qmz" }) == "etamil" then
    ok("*.qmz files are detected as eTamil")
  else
    warn("*.qmz is not detected as eTamil: is the plugin on the runtimepath?")
  end

  local cmd = require("etamil").defaults.cmd[1]
  if vim.fn.executable(cmd) == 1 then
    ok("the language server is on the PATH: " .. vim.fn.exepath(cmd))
  else
    warn(
      "`" .. cmd .. "` is not on the PATH, so no language server will start",
      { "install the eTamil compiler release, which includes it", "or build it: cargo install --path etamil_lsp" }
    )
  end

  local found = pcall(vim.treesitter.language.add, "etamil")
  if found then
    ok("the tree-sitter grammar for eTamil is installed")
  else
    info("the tree-sitter grammar is not installed; with nvim-treesitter, run :TSInstall etamil")
  end

  if vim.fn.executable("etamil") == 1 then
    ok("the compiler is on the PATH: " .. vim.fn.exepath("etamil"))
  else
    info("`etamil` is not on the PATH; it is not needed for editing")
  end
end

return M
