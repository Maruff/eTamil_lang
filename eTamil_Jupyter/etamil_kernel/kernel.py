# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
"""The Jupyter side: ipykernel's `Kernel`, with the compiler's shell behind it.

Thin on purpose. What a cell does is decided by `etamil --repl` (see session.py);
this class only carries a cell to it and its output back, and answers the
notebook's "is this cell finished?" question.
"""

from ipykernel.kernelbase import Kernel

from . import __version__
from .session import is_incomplete, make_session


class ETamilKernel(Kernel):
    implementation = "etamil_kernel"
    implementation_version = __version__
    language_info = {
        "name": "etamil",
        "mimetype": "text/x-etamil",
        "file_extension": ".qmz",
        # The Pygments lexer in eTamil_Pygments registers this alias, so exported
        # notebooks and nbconvert colour the code when that package is installed.
        "pygments_lexer": "etamil",
    }
    banner = "eTamil"

    # Replaceable, so the kernel can be tested with a session that is not the real one.
    session_factory = staticmethod(make_session)

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self._session = None

    def _get_session(self):
        if self._session is None:
            self._session = self.session_factory()
        return self._session

    def do_execute(self, code, silent, store_history=True, user_expressions=None, allow_stdin=False):
        reply = {"execution_count": self.execution_count, "payload": [], "user_expressions": {}}
        try:
            result = self._get_session().run_cell(code)
        except FileNotFoundError as missing:
            return self._failed(reply, "CompilerNotFound", str(missing), silent)

        if not silent:
            if result.lost_state and not result.error:
                self._stream("stderr", "The eTamil session was restarted; earlier variables and functions are gone.\n")
            if result.output:
                self._stream("stdout", result.output)
        if result.error:
            return self._failed(reply, "eTamilError", result.error, silent)
        return {"status": "ok", **reply}

    def _failed(self, reply, name, message, silent):
        if not silent:
            self.send_response(
                self.iopub_socket,
                "error",
                {"ename": name, "evalue": message, "traceback": [message]},
            )
        return {"status": "error", "ename": name, "evalue": message, "traceback": [message], **reply}

    def _stream(self, name, text):
        self.send_response(self.iopub_socket, "stream", {"name": name, "text": text})

    def do_is_complete(self, code):
        if is_incomplete(code):
            return {"status": "incomplete", "indent": "    "}
        return {"status": "complete"}

    def do_shutdown(self, restart):
        if self._session is not None:
            self._session.close()
            self._session = None
        return {"status": "ok", "restart": restart}
