# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
"""A long-lived `etamil --repl`, driven one line at a time.

The compiler's own shell already does what a notebook needs: variables and
function definitions survive from one input to the next, a line that ends inside
an unclosed `{` keeps reading until the braces balance, and an error is reported
and the session carries on. So the kernel does not reimplement any of that. It
runs the shell as a child process and speaks to it the way a person at a terminal
does.

The protocol it relies on, all visible in `etamil_compiler/src/repl.rs`:

* a prompt is printed before **every** line is read: `» ` normally, `…  ` while a
  block is still open;
* a result is printed to stdout, an error to stderr starting with `✗ `;
* `open_braces` below is the shell's own rule for when a block is closed.

Sending one line and waiting for the next prompt therefore gives exactly that
line's output, with no marker text injected into the session. It also lets a cell
stop at its first error, which is what a notebook reader expects.

Everything here is plain Python with no Jupyter dependency, so it is tested on its
own (tests/).
"""

from __future__ import annotations

import codecs
import os
import queue
import shutil
import subprocess
import sys
import threading
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Mapping, Optional, Sequence

PROMPT = "» "
CONTINUE = "…  "
ERROR_MARK = "✗ "

ENV_VAR = "ETAMIL_BIN"


def open_braces(source: str) -> int:
    """How many `{` are still unclosed, ignoring any inside a string.

    The same rule, character for character, as `open_braces` in repl.rs, because
    the question it answers is the shell's: is this input finished? That includes
    its one quirk: a `{` inside a `//` comment counts.
    """
    depth = 0
    in_string = False
    escaped = False
    for ch in source:
        if in_string:
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == '"':
                in_string = False
            continue
        if ch == '"':
            in_string = True
        elif ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
    return max(depth, 0)


def is_incomplete(code: str) -> bool:
    """Would the shell still be waiting for more after this cell?

    Played out line by line, as the shell reads it, rather than counted over the
    whole cell: a stray `}` closes nothing in the shell, so a cell that has one
    and later opens a block can still end unclosed.
    """
    pending = ""
    for line in code.replace("\r\n", "\n").split("\n"):
        if not pending and line.strip() in ("", ":quit", ":q", ":help", ":h", ":vars"):
            continue
        pending += line + "\n"
        if open_braces(pending) > 0:
            continue
        pending = ""
    return pending != ""


def split_errors(text: str) -> "tuple[str, Optional[str]]":
    """Separate a line's output from its error, if it has one.

    Errors come on stderr, which is merged into the same pipe so that ordering is
    exact, and are told apart by the `✗ ` the shell puts in front of them.
    """
    output = []
    errors = []
    for line in text.splitlines(keepends=True):
        if line.startswith(ERROR_MARK):
            errors.append(line[len(ERROR_MARK):].rstrip("\r\n"))
        else:
            output.append(line)
    return "".join(output), ("\n".join(errors) if errors else None)


def find_compiler(
    env: Optional[Mapping[str, str]] = None,
    which: Callable[[str], Optional[str]] = shutil.which,
    home: Optional[Path] = None,
    is_file: Callable[[str], bool] = os.path.isfile,
) -> Optional[str]:
    """Where `etamil` is: `ETAMIL_BIN`, the PATH, then the installers' folders."""
    env = os.environ if env is None else env
    explicit = env.get(ENV_VAR, "").strip()
    if explicit:
        return explicit
    found = which("etamil")
    if found:
        return found
    home = Path.home() if home is None else home
    candidates = [home / ".local" / "bin" / "etamil", home / ".local" / "bin" / "etamil.exe"]
    local = env.get("LOCALAPPDATA")
    if local:
        base = Path(local) / "Programs" / "eTamil"
        candidates += [base / "etamil.exe", base / "compiler" / "etamil.exe"]
    for candidate in candidates:
        if is_file(str(candidate)):
            return str(candidate)
    return None


class SessionEnded(Exception):
    """The shell exited (a `:quit` in a cell, or a crash) while a line was running."""

    def __init__(self, partial: str = "") -> None:
        super().__init__("the eTamil session ended")
        self.partial = partial


@dataclass
class CellResult:
    output: str = ""
    error: Optional[str] = None
    # Variables and functions from earlier cells are gone: the session was
    # restarted, after an interrupt or because the shell had exited.
    lost_state: bool = False


class ReplSession:
    """One `etamil --repl` process and the conversation with it."""

    def __init__(
        self,
        command: Sequence[str],
        cwd: Optional[str] = None,
        env: Optional[Mapping[str, str]] = None,
    ) -> None:
        self.command = list(command)
        self.cwd = cwd
        self.env = dict(env) if env is not None else None
        self.banner = ""
        self._proc: Optional[subprocess.Popen] = None
        self._chunks: "queue.Queue[Optional[bytes]]" = queue.Queue()
        self._decoder = codecs.getincrementaldecoder("utf-8")("replace")

    # -- process ---------------------------------------------------------------

    def alive(self) -> bool:
        return self._proc is not None and self._proc.poll() is None

    def start(self) -> None:
        self._decoder = codecs.getincrementaldecoder("utf-8")("replace")
        self._chunks = queue.Queue()
        self._proc = subprocess.Popen(
            self.command,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            # One pipe for both, so an error cannot overtake the output before it.
            stderr=subprocess.STDOUT,
            cwd=self.cwd,
            env=self.env,
            bufsize=0,
        )
        # Reads on a thread because a pipe cannot be polled on every platform, and
        # a blocked read must not stop the kernel noticing an interrupt. The queue
        # is passed in, so a pump left over from a killed session cannot write
        # into the next one's.
        threading.Thread(target=self._pump, args=(self._proc.stdout, self._chunks), daemon=True).start()
        self.banner, _ = self._read_until_prompt()

    @staticmethod
    def _pump(stream, chunks: "queue.Queue[Optional[bytes]]") -> None:
        try:
            while True:
                data = stream.read(65536)
                if not data:
                    break
                chunks.put(data)
        except (OSError, ValueError):
            pass
        finally:
            # Closed here, by the only thread reading it, so closing cannot race a read.
            try:
                stream.close()
            except OSError:
                pass
        chunks.put(None)

    def close(self) -> None:
        proc, self._proc = self._proc, None
        if proc is None:
            return
        try:
            if proc.stdin:
                proc.stdin.close()
        except OSError:
            pass
        if proc.poll() is None:
            proc.kill()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass

    # -- one line --------------------------------------------------------------

    def _read_until_prompt(self) -> "tuple[str, str]":
        """Read until the shell asks for the next line. Returns (output, which prompt)."""
        text = ""
        while True:
            try:
                chunk = self._chunks.get(timeout=0.1)
            except queue.Empty:
                if self._proc is None or self._proc.poll() is not None:
                    raise SessionEnded(text)
                continue
            if chunk is None:
                raise SessionEnded(text)
            text += self._decoder.decode(chunk)
            for prompt in (PROMPT, CONTINUE):
                # A prompt starts a line (after a result's newline), or is all there is.
                if text.endswith(prompt) and (len(text) == len(prompt) or text[-len(prompt) - 1] == "\n"):
                    # Some Windows tools write CRLF; a notebook wants LF.
                    return text[: -len(prompt)].replace("\r\n", "\n"), prompt

    def run_line(self, line: str) -> "tuple[str, str]":
        assert self._proc is not None and self._proc.stdin is not None
        try:
            self._proc.stdin.write((line + "\n").encode("utf-8"))
            self._proc.stdin.flush()
        except OSError:
            raise SessionEnded() from None
        return self._read_until_prompt()

    # -- one cell --------------------------------------------------------------

    def run_cell(self, code: str) -> CellResult:
        """Run a cell's lines in order, stopping at the first error."""
        lost = False
        if not self.alive():
            lost = self._proc is not None or self.banner != ""
            self.close()
            self.start()

        # Nothing is sent for a cell that would leave a block open: there is no way
        # to take back half a block, and finishing it for the author would run code
        # they did not write.
        if is_incomplete(code):
            return CellResult(error="a { in this cell is never closed, so nothing was run", lost_state=lost)

        parts = []
        error = None
        try:
            for line in code.replace("\r\n", "\n").split("\n"):
                text, _prompt = self.run_line(line)
                output, error = split_errors(text)
                parts.append(output)
                if error:
                    break
        except KeyboardInterrupt:
            # The shell may be in the middle of anything. Starting afresh is the
            # only state that can be described, so say what was lost.
            self.close()
            self.banner = ""  # reported now; the next start is not news
            return CellResult("".join(parts), "interrupted: the session was restarted, so earlier variables and functions are gone", True)
        except SessionEnded as ended:
            self.close()
            self.banner = ""
            output, partial_error = split_errors(ended.partial)
            parts.append(output)
            return CellResult(
                "".join(parts),
                partial_error or "the eTamil session ended; the next cell starts a new one, without the earlier variables",
                True,
            )
        return CellResult("".join(parts), error, lost)

    def __enter__(self) -> "ReplSession":
        self.start()
        return self

    def __exit__(self, *exc) -> None:
        self.close()


def make_session(cwd: Optional[str] = None) -> ReplSession:
    """A session on the compiler this machine has, or an error saying how to get one."""
    compiler = find_compiler()
    if compiler is None:
        raise FileNotFoundError(
            f"cannot find the eTamil compiler. Put `etamil` on the PATH or set {ENV_VAR} to its full path."
        )
    return ReplSession([compiler, "--repl"], cwd=cwd)


# Kept so a stray `python -m etamil_kernel.session` explains itself.
if __name__ == "__main__":
    print(__doc__, file=sys.stderr)
