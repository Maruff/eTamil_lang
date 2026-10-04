# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
"""The Jupyter layer, against stand-ins for ipykernel and jupyter_client.

Neither is installed where these run, and the kernel is deliberately thin, so what is
tested is the part that is ours: which messages a cell turns into, what the replies
say, and what `install` writes. The stand-ins implement only the calls the kernel
makes, copied from ipykernel's documented API; running inside a real Jupyter is a
separate, manual check (see README.md).
"""

import json
import sys
import types
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))


class StubKernel:
    execution_count = 3

    def __init__(self, **kwargs):
        self.sent = []
        self.iopub_socket = "iopub"

    def send_response(self, socket, message_type, content):
        assert socket == "iopub"
        self.sent.append((message_type, content))


def install_stubs():
    ipykernel = types.ModuleType("ipykernel")
    kernelbase = types.ModuleType("ipykernel.kernelbase")
    kernelbase.Kernel = StubKernel
    ipykernel.kernelbase = kernelbase
    sys.modules["ipykernel"] = ipykernel
    sys.modules["ipykernel.kernelbase"] = kernelbase


install_stubs()

from etamil_kernel import __main__ as entry  # noqa: E402
from etamil_kernel.kernel import ETamilKernel  # noqa: E402
from etamil_kernel.session import ReplSession  # noqa: E402

FAKE = [sys.executable, str(HERE / "fake_repl.py")]


def kernel_with(factory=lambda: ReplSession(FAKE)):
    kernel = ETamilKernel()
    kernel.session_factory = factory
    return kernel


class Execute(unittest.TestCase):
    def setUp(self):
        self.kernel = kernel_with()
        self.addCleanup(self.kernel.do_shutdown, False)

    def test_output_becomes_a_stdout_stream(self):
        reply = self.kernel.do_execute("say வணக்கம்", silent=False)
        self.assertEqual(reply["status"], "ok")
        self.assertEqual(reply["execution_count"], 3)
        self.assertEqual(self.kernel.sent, [("stream", {"name": "stdout", "text": "வணக்கம்\n"})])

    def test_a_cell_with_no_output_sends_nothing(self):
        reply = self.kernel.do_execute("quiet", silent=False)
        self.assertEqual((reply["status"], self.kernel.sent), ("ok", []))

    def test_an_error_is_an_error_message_and_an_error_reply(self):
        reply = self.kernel.do_execute("say before\nfail", silent=False)
        self.assertEqual(reply["status"], "error")
        self.assertEqual(reply["ename"], "eTamilError")
        self.assertEqual(reply["evalue"], "it failed")
        kinds = [kind for kind, _ in self.kernel.sent]
        self.assertEqual(kinds, ["stream", "error"], "the output before the error is shown too")
        self.assertEqual(self.kernel.sent[1][1], {"ename": "eTamilError", "evalue": "it failed", "traceback": ["it failed"]})

    def test_a_silent_cell_runs_but_shows_nothing(self):
        self.kernel.do_execute("fail", silent=True)
        self.assertEqual(self.kernel.sent, [])

    def test_state_is_kept_in_one_session_across_cells(self):
        self.kernel.do_execute("say one", silent=False)
        session = self.kernel._session
        self.kernel.do_execute("say two", silent=False)
        self.assertIs(self.kernel._session, session)

    def test_a_loss_the_cell_already_reported_is_not_repeated(self):
        self.kernel.do_execute("say a\n:quit", silent=False)  # ends the shell, and says so
        self.kernel.sent.clear()
        self.kernel.do_execute("say again", silent=False)
        self.assertEqual(self.kernel.sent, [("stream", {"name": "stdout", "text": "again\n"})])

    def test_a_shell_that_died_between_cells_is_replaced_and_the_reader_told(self):
        self.kernel.do_execute("say a", silent=True)
        self.kernel._session._proc.kill()
        self.kernel._session._proc.wait()
        self.kernel.sent.clear()
        reply = self.kernel.do_execute("say again", silent=False)
        self.assertEqual(reply["status"], "ok")
        kinds = [(kind, content.get("name")) for kind, content in self.kernel.sent]
        self.assertEqual(kinds, [("stream", "stderr"), ("stream", "stdout")])
        self.assertIn("earlier variables", self.kernel.sent[0][1]["text"])


class CompilerMissing(unittest.TestCase):
    def test_the_reply_says_how_to_fix_it(self):
        def missing():
            raise FileNotFoundError("cannot find the eTamil compiler. Put `etamil` on the PATH or set ETAMIL_BIN")

        kernel = kernel_with(missing)
        reply = kernel.do_execute("1 + 1", silent=False)
        self.assertEqual((reply["status"], reply["ename"]), ("error", "CompilerNotFound"))
        self.assertIn("ETAMIL_BIN", reply["evalue"])
        self.assertEqual(kernel.sent[0][0], "error")


class Completeness(unittest.TestCase):
    def test_answers(self):
        kernel = kernel_with()
        self.assertEqual(kernel.do_is_complete("x = 1;"), {"status": "complete"})
        self.assertEqual(kernel.do_is_complete("செயல் f() {"), {"status": "incomplete", "indent": "    "})
        self.assertEqual(kernel.do_is_complete("செயல் f() {\n}"), {"status": "complete"})


class Shutdown(unittest.TestCase):
    def test_the_session_is_closed(self):
        kernel = kernel_with()
        kernel.do_execute("say hi", silent=True)
        session = kernel._session
        self.assertEqual(kernel.do_shutdown(restart=True), {"status": "ok", "restart": True})
        self.assertFalse(session.alive())
        self.assertIsNone(kernel._session)

    def test_shutting_down_before_any_cell_is_fine(self):
        self.assertEqual(kernel_with().do_shutdown(restart=False)["status"], "ok")


class Identity(unittest.TestCase):
    def test_language_info(self):
        info = ETamilKernel.language_info
        self.assertEqual(info["name"], "etamil")
        self.assertEqual(info["file_extension"], ".qmz")
        self.assertEqual(info["pygments_lexer"], "etamil")


class Install(unittest.TestCase):
    def test_the_kernel_spec(self):
        spec = entry.kernel_spec()
        self.assertEqual(spec["argv"][1:], ["-m", "etamil_kernel", "-f", "{connection_file}"])
        self.assertEqual((spec["display_name"], spec["language"]), ("eTamil", "etamil"))

    def test_install_hands_a_kernel_json_to_jupyter(self):
        calls = []
        seen = {}

        class Manager:
            def install_kernel_spec(self, folder, name, user=False, prefix=None):
                calls.append((name, user, prefix))
                seen["json"] = json.loads((Path(folder) / "kernel.json").read_text(encoding="utf-8"))
                return "/where/it/went"

        client = types.ModuleType("jupyter_client")
        spec_module = types.ModuleType("jupyter_client.kernelspec")
        spec_module.KernelSpecManager = Manager
        client.kernelspec = spec_module
        sys.modules["jupyter_client"] = client
        sys.modules["jupyter_client.kernelspec"] = spec_module
        self.addCleanup(sys.modules.pop, "jupyter_client", None)
        self.addCleanup(sys.modules.pop, "jupyter_client.kernelspec", None)

        self.assertEqual(entry.install(), "/where/it/went")
        self.assertEqual(calls[-1], ("etamil", True, None))
        self.assertEqual(seen["json"]["language"], "etamil")

        entry.install(prefix="/env")
        self.assertEqual(calls[-1], ("etamil", False, "/env"), "a prefix replaces --user")


if __name__ == "__main__":
    unittest.main()
