-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
--
-- The plugin's tests. Run from the repository root:
--
--   nvim --headless -u NONE -l eTamil_Neovim/tests/run.lua
--
-- The highlight query is checked against the real grammar when ETAMIL_PARSER names a
-- compiled one (CI builds it); without it that one test is skipped, and says so.

local here = debug.getinfo(1, "S").source:sub(2)
local plugin = vim.fn.fnamemodify(here, ":p:h:h")

vim.opt.runtimepath:prepend(plugin)
vim.cmd("filetype plugin indent on")
vim.cmd("runtime! ftdetect/*.lua")

local failures = {}
local passed = 0

local function test(name, body)
  local ok, err = pcall(body)
  if ok then
    passed = passed + 1
    print("ok      " .. name)
  else
    failures[#failures + 1] = name
    print("FAILED  " .. name .. "\n          " .. tostring(err))
  end
end

local function eq(actual, expected, what)
  if actual ~= expected then
    error(string.format("%s: expected %s, got %s", what or "value", vim.inspect(expected), vim.inspect(actual)), 2)
  end
end

test("a .qmz file is detected as eTamil", function()
  eq(vim.filetype.match({ filename = "kaNakku_qaLam.qmz" }), "etamil", "filetype")
end)

test("other extensions are not claimed", function()
  assert(vim.filetype.match({ filename = "notes.txt" }) ~= "etamil")
end)

test("an eTamil buffer gets // comments and four spaces", function()
  vim.cmd("edit " .. vim.fn.fnameescape(plugin .. "/tests/cOqaZY.qmz"))
  eq(vim.bo.filetype, "etamil", "filetype")
  eq(vim.bo.commentstring, "// %s", "commentstring")
  eq(vim.bo.expandtab, true, "expandtab")
  eq(vim.bo.shiftwidth, 4, "shiftwidth")
end)

test("the language server's configuration names the file type and the command", function()
  local config = require("etamil").lsp_config()
  eq(config.name, "etamil", "name")
  eq(config.cmd[1], "etamil-lsp", "command")
  eq(config.filetypes[1], "etamil", "file type")
end)

test("the command can be changed", function()
  local config = require("etamil").lsp_config({ cmd = { "/opt/etamil/etamil-lsp", "--stdio" } })
  eq(config.cmd[1], "/opt/etamil/etamil-lsp", "command")
  eq(config.cmd[2], "--stdio", "argument")
end)

test("setup with no language server on the PATH does nothing and does not fail", function()
  local started = require("etamil").setup({ cmd = { "etamil-no-such-server" }, treesitter = false })
  eq(started, false, "started")
end)

test("setup registers the server when its command exists", function()
  -- Neovim itself is a command that exists wherever this runs. Close every buffer first:
  -- Neovim 0.11 attaches a newly enabled server to the buffers already open, and an
  -- eTamil buffer left by an earlier test would start `nvim` as a language server.
  vim.cmd("silent! %bwipeout!")
  local started = require("etamil").setup({ cmd = { "nvim" }, treesitter = false })
  eq(started, true, "started")
  if vim.lsp.config ~= nil and vim.lsp.enable ~= nil then
    eq(vim.lsp.config["etamil"].cmd[1], "nvim", "registered command")
    eq(vim.lsp.config["etamil"].filetypes[1], "etamil", "registered file type")
  else
    eq(#vim.api.nvim_get_autocmds({ group = "etamil_lsp", event = "FileType" }), 1, "autocmd")
  end
end)

test("setup registers the grammar's file type with tree-sitter", function()
  require("etamil").setup({ lsp = false })
  eq(vim.treesitter.language.get_lang("etamil"), "etamil", "language for the file type")
end)

test("the grammar is a folder of the compiler's repository", function()
  local grammar = require("etamil").grammar
  eq(grammar.location, "tree-sitter-etamil", "location")
  eq(grammar.files[1], "src/parser.c", "source")
end)

test(":checkhealth has a check to run", function()
  eq(type(require("etamil.health").check), "function", "check")
end)

local parser = vim.env.ETAMIL_PARSER
if parser == nil or parser == "" then
  print("skipped the highlight query test: ETAMIL_PARSER is not set")
else
  test("the highlight query is valid for the grammar and marks keywords, strings and comments", function()
    -- The committed parser is built for tree-sitter's ABI 15, which Neovim 0.11 loads and
    -- Neovim 0.10 (ABI 13 and 14) refuses. That is a fact about the version, not a defect in
    -- the plugin, so it is reported and not failed. Everything else is still tested there.
    local loaded, why = pcall(vim.treesitter.language.add, "etamil", { path = parser })
    if not loaded and tostring(why):find("ABI version mismatch", 1, true) then
      print("skipped the highlight query test: this Neovim cannot load the grammar (" .. tostring(why) .. ")")
      return
    end
    assert(loaded, why)
    local lines = vim.fn.readfile(plugin .. "/tests/cOqaZY.qmz")
    local source = table.concat(lines, "\n") .. "\n"
    local tree = vim.treesitter.get_string_parser(source, "etamil"):parse()[1]
    local query = vim.treesitter.query.get("etamil", "highlights")
    assert(query ~= nil, "no highlights query was found on the runtimepath")

    local seen = {}
    for id in query:iter_captures(tree:root(), source) do
      seen[query.captures[id]] = true
    end
    for _, name in ipairs({ "keyword", "string", "comment", "number" }) do
      assert(seen[name], "nothing was captured as @" .. name)
    end
  end)
end

print(string.format("%d passed, %d failed", passed, #failures))
if #failures > 0 then
  os.exit(1)
end
os.exit(0)
