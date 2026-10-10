# eTamil kernel for Jupyter

Run eTamil in a notebook. Each cell goes to the compiler's own interactive shell
(`etamil --repl`), so variables and functions defined in one cell are there in the
next, and an error stops the cell and leaves the session as it was.

## Install

You need Python 3.9 or later and the `etamil` compiler (see the main README, or the
extension for VS Code, which carries one).

```bash
pip install ./eTamil_Jupyter          # from a checkout of this repository
python -m etamil_kernel install       # registers the kernel for your user
jupyter lab                           # then choose "eTamil" in the launcher
```

`python -m etamil_kernel install --sys-prefix` registers it inside the current
virtual environment instead, and `--prefix PATH` under a prefix of your choice.

The kernel finds the compiler from the environment variable `ETAMIL_BIN` (its full
path), then the `PATH`, then `~/.local/bin` and, on Windows,
`%LOCALAPPDATA%\Programs\eTamil`. To run `இறக்கு "nUlakam/..."` imports, `ETAMIL_PATH`
must name the folder that holds `nUlakam`, as it does for the command line; the
release packages' installers set it.

Colouring in exported notebooks and `nbconvert` comes from the Pygments lexer in
`eTamil_Pygments`, if that is installed alongside.

## How a cell behaves

- **State is kept.** `x = 5;` in one cell, `x * 2` in the next gives `10`. So do
  functions: define a `செயல்` in one cell and call it in another.
- **An expression shows its value.** `1 + 2` on its own prints `3`, as in the shell.
  There is no separate "result" cell output: everything the cell prints, and every
  expression's value, appears as text output.
- **A cell stops at its first error,** shows what came before it, and reports the error
  in red. Later cells still see the state as it was before the failing line.
- **A cell with an unclosed `{` runs nothing.** The shell cannot take back half a
  block, and finishing it for you would run code you did not write. The notebook's
  "is this complete?" check also knows, so the editor keeps the cursor in the block.
- **Interrupting** a running cell (the stop button) restarts the session, because the
  shell may be in the middle of anything. The cell says so: earlier variables and
  functions are gone. The same notice appears if the shell exits on its own, for
  example if a cell contains `:quit`.

## Limits

- **A statement that spans several lines outside braces is not supported,** such as an
  array literal split over lines. The shell reads a line at a time and only keeps
  reading for an open `{`. Put it on one line, or inside a function.
- **Output appears when each input finishes,** not while it runs, so a long loop shows
  nothing until it ends.
- **No completion, hover or inspection yet,** and no rich output (tables, charts).
- A line your program prints that begins with `✗ ` is read as an error, because that
  is how the shell marks its own.

## Tests

```bash
cd eTamil_Jupyter
python -m unittest discover -s tests
```

43 tests. The session driver is tested against a fake shell that follows the same
prompt protocol (so dying mid-cell, CRLF line endings and large output are
reproducible) and against the real compiler, which is skipped when `etamil` cannot be
found. The Jupyter layer is tested against stand-ins for `ipykernel` and
`jupyter_client`, since the kernel only uses a handful of their calls.

**Not covered by the tests:** running inside a real Jupyter. Before relying on it,
install it as above, open a notebook with the eTamil kernel, run a cell, define a
function and call it from another, and press the stop button once.
