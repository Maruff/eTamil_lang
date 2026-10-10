-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
--
-- Buffer settings for eTamil: // comments, four spaces, no tabs, as the examples and
-- the standard library are written.

vim.bo.commentstring = "// %s"
vim.bo.comments = "://"
vim.bo.expandtab = true
vim.bo.shiftwidth = 4
vim.bo.softtabstop = 4
vim.bo.tabstop = 4

-- Carry the comment leader onto a new line, and let gq reflow a comment.
vim.cmd("setlocal formatoptions+=croq")

vim.b.undo_ftplugin = table.concat({
  "setlocal commentstring< comments< expandtab< shiftwidth< softtabstop< tabstop< formatoptions<",
}, " | ")
