# etamil-mode: eTamil for Emacs

A major mode for `.qmz` files. Needs Emacs 27.1 or later.

## Install

Put this folder on your `load-path` and require the mode:

```elisp
(add-to-list 'load-path "/path/to/eTamil_lang/eTamil_Emacs")
(require 'etamil-mode)
```

`.qmz` files then open in `etamil-mode`.

## What it does

- **Highlighting** of keywords, types, constants and built-in functions, in Tamil
  script and in Latin letters. The word lists in `etamil-words.el` are generated from
  the compiler's lexer by `scripts/generate_editor_support.py`, the same source the
  other editors use, so a keyword the compiler accepts is not missed, and CI fails
  if the file drifts. A name you chose yourself has no colour.
- **Literals and comments:** numbers and percentages (`50000`, `7.5%`), strings
  (which may span lines), and `//` line comments. There is no block comment in
  eTamil, so none is recognised.
- **Indentation** by bracket depth (`etamil-indent-offset`, 4 by default); a closing
  bracket sits one level out, and the inside of a multi-line string is left alone.
- **Run and check:** `C-c C-c` runs the file, `C-c C-k` checks it with `etamil --check`,
  which never runs the program. Both use a compilation buffer and run from the file's
  folder. `etamil-executable` names the compiler if it is not on your `PATH`.
- **Language server:** `M-x eglot` in an eTamil buffer starts `etamil-lsp` for
  diagnostics, completion and hover. Eglot is built in from Emacs 29; on 27 and 28
  install it from GNU ELPA. `etamil-lsp-executable` names the server if it is not on
  your `PATH`.

## Limits

- Indentation knows brackets only. A statement continued over several lines without a
  bracket is not given extra indentation.
- Compiler messages in the compilation buffer are not turned into jump-to-error links:
  they name a line and column but not the file.
- No Imenu, no electric pairs, no snippets. The VS Code extension has snippets.

## Tests

```bash
emacs -Q --batch -L eTamil_Emacs -l ert -l eTamil_Emacs/etamil-mode-tests.el \
      -f ert-run-tests-batch-and-exit
```

The tests cover the word lists, the syntax table (Tamil letters and marks, comments,
strings), highlighting, indentation, the run and check commands, and the Eglot
registration. CI also byte-compiles the mode with warnings as errors, under Emacs
27.2, 29.4 and 30.1.

**Note on how this was written:** Emacs is not installed on the machine it was written
on, so nothing here ran before it reached CI. The first CI run is its first real test.
