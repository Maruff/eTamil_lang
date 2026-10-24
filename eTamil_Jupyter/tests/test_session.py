# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
"""The session driver: the protocol against a fake shell, then against the real compiler.

    python -m unittest discover -s tests          (from eTamil_Jupyter/)

The real-compiler tests run when `etamil` can be found (ETAMIL_BIN or the PATH), and
are skipped otherwise.
"""

import os
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from etamil_kernel.session import (  # noqa: E402
    ReplSession,
    SessionEnded,
    find_compiler,
    is_incomplete,
    open_braces,
    split_errors,
)

FAKE = [sys.executable, str(HERE / "fake_repl.py")]


class BraceRules(unittest.TestCase):
    """These mirror the unit tests of `open_braces` in repl.rs."""

    def test_unclosed_and_balanced(self):
        self.assertEqual(open_braces("செயல் f() {"), 1)
        self.assertEqual(open_braces("செயல் f() { }"), 0)

    def test_braces_inside_a_string_are_not_braces(self):
        self.assertEqual(open_braces('அச்சு("{{{");'), 0)
        self.assertEqual(open_braces('அச்சு("a\\"{");'), 0, "an escaped quote does not end the string")

    def test_a_stray_close_does_not_go_negative(self):
        self.assertEqual(open_braces("}}"), 0)

    def test_the_shells_quirk_is_kept(self):
        self.assertEqual(open_braces("// {"), 1, "a { in a comment counts, as in repl.rs")


class IsIncomplete(unittest.TestCase):
    def test_cells(self):
        self.assertFalse(is_incomplete("x = 1;"))
        self.assertFalse(is_incomplete("x = 1;\ny = 2;"))
        self.assertTrue(is_incomplete("செயல் f() {\n  திரும்பு 1;"))
        self.assertFalse(is_incomplete("செயல் f() {\n  திரும்பு 1;\n}"))
        self.assertFalse(is_incomplete(""))
        self.assertFalse(is_incomplete("\n\n"))

    def test_played_out_line_by_line_not_counted_over_the_cell(self):
        # A stray } closes nothing in the shell, so the later { is still open.
        self.assertTrue(is_incomplete("}\n{"))

    def test_crlf(self):
        self.assertTrue(is_incomplete("a {\r\nb"))


class SplitErrors(unittest.TestCase):
    def test_output_and_error_are_separated(self):
        self.assertEqual(split_errors("6\n"), ("6\n", None))
        self.assertEqual(split_errors("✗ boom\n"), ("", "boom"))
        self.assertEqual(split_errors("a\n✗ one\nb\n✗ two\n"), ("a\nb\n", "one\ntwo"))
        self.assertEqual(split_errors(""), ("", None))


class FindCompiler(unittest.TestCase):
    def test_order(self):
        home = Path("/home/me")
        none = lambda name: None  # noqa: E731
        never = lambda path: False  # noqa: E731
        self.assertEqual(find_compiler({"ETAMIL_BIN": " /x/etamil "}, none, home, never), "/x/etamil")
        self.assertEqual(find_compiler({}, lambda n: "/usr/bin/etamil", home, never), "/usr/bin/etamil")
        installed = str(home / ".local" / "bin" / "etamil")
        self.assertEqual(find_compiler({}, none, home, lambda p: p == installed), installed)
        self.assertIsNone(find_compiler({}, none, home, never))

    def test_the_windows_installer_folder(self):
        wanted = str(Path("C:/Users/me/AppData/Local") / "Programs" / "eTamil" / "compiler" / "etamil.exe")
        found = find_compiler({"LOCALAPPDATA": "C:/Users/me/AppData/Local"}, lambda n: None, Path("/h"), lambda p: p == wanted)
        self.assertEqual(found, wanted)


class AgainstAFakeShell(unittest.TestCase):
    def setUp(self):
        self.session = ReplSession(FAKE)
        self.addCleanup(self.session.close)

    def test_the_banner_is_read_and_a_cell_gets_its_own_output(self):
        self.session.start()
        self.assertIn("fake eTamil", self.session.banner)
        result = self.session.run_cell("say வணக்கம்")
        self.assertEqual((result.output, result.error), ("வணக்கம்\n", None))

    def test_a_line_with_no_output_gives_none(self):
        self.session.start()
        self.assertEqual(self.session.run_cell("quiet").output, "")

    def test_each_cell_gets_only_its_own_output(self):
        self.session.start()
        self.assertEqual(self.session.run_cell("say one").output, "one\n")
        self.assertEqual(self.session.run_cell("say two").output, "two\n")

    def test_lines_run_in_order_and_a_blank_line_is_harmless(self):
        self.session.start()
        result = self.session.run_cell("say a\n\nsay b\n")
        self.assertEqual(result.output, "a\nb\n")

    def test_a_cell_stops_at_its_first_error(self):
        self.session.start()
        result = self.session.run_cell("say before\nfail\nsay after")
        self.assertEqual(result.output, "before\n")
        self.assertEqual(result.error, "it failed")
        # and the session is intact for the next cell
        self.assertEqual(self.session.run_cell("say next").output, "next\n")

    def test_a_block_over_several_lines_is_one_input(self):
        self.session.start()
        result = self.session.run_cell("செயல் f() {\n  say hi\n}\nsay done")
        self.assertEqual(result.output, "block ran\ndone\n")

    def test_a_cell_that_leaves_a_block_open_runs_nothing(self):
        self.session.start()
        result = self.session.run_cell("say would-run\nசெயல் f() {")
        self.assertEqual(result.output, "")
        self.assertIn("never closed", result.error)
        self.assertEqual(self.session.run_cell("say fine").output, "fine\n", "the session was not disturbed")

    def test_a_session_that_ends_is_reported_and_replaced(self):
        self.session.start()
        gone = self.session.run_cell("say last\n:quit")
        self.assertEqual(gone.output, "last\n")
        self.assertIn("session ended", gone.error)
        self.assertTrue(gone.lost_state)
        again = self.session.run_cell("say back")
        self.assertEqual(again.output, "back\n")
        self.assertFalse(again.lost_state, "the loss was reported once, not on every cell after")

    def test_the_first_cell_starts_the_session_without_calling_it_a_restart(self):
        result = self.session.run_cell("say hello")  # no start() first
        self.assertEqual(result.output, "hello\n")
        self.assertFalse(result.lost_state)

    def test_windows_line_endings_become_plain_ones(self):
        os.environ["FAKE_CRLF"] = "1"
        self.addCleanup(os.environ.pop, "FAKE_CRLF", None)
        session = ReplSession(FAKE)
        self.addCleanup(session.close)
        session.start()
        self.assertNotIn("\r", session.banner)
        self.assertEqual(session.run_cell("say a\nsay b").output, "a\nb\n")
        self.assertEqual(session.run_cell("fail").error, "it failed")

    def test_closing_twice_is_harmless(self):
        self.session.start()
        self.session.close()
        self.session.close()
        self.assertFalse(self.session.alive())

    def test_large_output_arrives_whole(self):
        self.session.start()
        text = "x" * 200000
        self.assertEqual(self.session.run_cell("say " + text).output, text + "\n")


COMPILER = find_compiler()


@unittest.skipUnless(COMPILER, "no etamil on the PATH and ETAMIL_BIN is not set")
class AgainstTheRealCompiler(unittest.TestCase):
    def setUp(self):
        self.session = ReplSession([COMPILER, "--repl"])
        self.addCleanup(self.session.close)
        self.session.start()

    def run_cell(self, code):
        return self.session.run_cell(code)

    def test_a_result_and_an_expression(self):
        self.assertEqual(self.run_cell('அச்சு("வணக்கம்");').output.strip(), "வணக்கம்")
        self.assertEqual(self.run_cell("1 + 2").output.strip(), "3")

    def test_variables_survive_between_cells(self):
        self.assertEqual(self.run_cell("x = 5;").output, "")
        self.assertEqual(self.run_cell("x * 2").output.strip(), "10")

    def test_functions_survive_between_cells(self):
        self.run_cell("செயல் இரட்டிப்பு(a) {\n  திரும்பு a * 2;\n}")
        self.assertEqual(self.run_cell("இரட்டிப்பு(21)").output.strip(), "42")

    def test_an_error_is_an_error_and_the_session_carries_on(self):
        self.run_cell("y = 7;")
        result = self.run_cell("1 +")
        self.assertTrue(result.error, result)
        self.assertEqual(result.output, "")
        self.assertEqual(self.run_cell("y").output.strip(), "7", "state kept after an error")

    def test_a_cell_stops_at_its_first_error(self):
        result = self.run_cell('அச்சு("a");\n1 +\nஅச்சு("never");')
        self.assertEqual(result.output.strip(), "a")
        self.assertTrue(result.error)

    def test_an_unclosed_block_is_refused_before_anything_runs(self):
        result = self.run_cell('அச்சு("never");\nசெயல் f() {')
        self.assertEqual(result.output, "")
        self.assertIn("never closed", result.error)

    def test_a_multi_line_loop(self):
        result = self.run_cell("i = 0;\n(i < 3) சுற்று {\n  அச்சு(i);\n  i = i + 1;\n}")
        self.assertEqual(result.output.split(), ["0", "1", "2"])


if __name__ == "__main__":
    unittest.main()
