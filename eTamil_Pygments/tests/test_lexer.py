"""Tests for the eTamil Pygments lexer.

The cases mirror eTamil_Code/test/grammar.test.js, so the two highlighters
cannot disagree about what a word is without one of these failing.
"""

import glob
import os
import unittest

from pygments.token import Error

from etamil_pygments import ETamilLexer

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def token_of(source, word):
    """The token type Pygments assigns to `word` in `source`."""
    lexer = ETamilLexer()
    for token, value in lexer.get_tokens(source):
        if value == word:
            return str(token)
    raise AssertionError(f"{word!r} was never produced as its own token")


class TestKeywords(unittest.TestCase):
    def test_control_flow(self):
        self.assertEqual(token_of("(x > 1) எனில் {", "எனில்"), "Token.Keyword")
        self.assertEqual(token_of("இன்றேல் {", "இன்றேல்"), "Token.Keyword")
        self.assertEqual(token_of("(x < 3) சுற்று {", "சுற்று"), "Token.Keyword")

    def test_romanized_forms_are_keywords_too(self):
        self.assertEqual(token_of("(x > 1) eZil {", "eZil"), "Token.Keyword")
        self.assertEqual(token_of("piZZam x = 1.5;", "piZZam"), "Token.Keyword.Type")

    def test_constants(self):
        self.assertEqual(
            token_of("ஈர்ம கொடி = மெய்;", "மெய்"), "Token.Keyword.Constant"
        )
        self.assertEqual(token_of("accu poy;", "poy"), "Token.Keyword.Constant")
        self.assertEqual(token_of("accu iZmY;", "iZmY"), "Token.Keyword.Constant")

    def test_import_and_logical_operator(self):
        self.assertEqual(
            token_of('இறக்கு "nUlakam/atippatY/col.qmz";', "இறக்கு"), "Token.Keyword.Namespace"
        )
        self.assertEqual(
            token_of("(a மற்றும் b) எனில் {", "மற்றும்"), "Token.Operator.Word"
        )


class TestNonKeywords(unittest.TestCase):
    """The parser accepts these as names, so they must not read as reserved."""

    def test_near_misses_are_names(self):
        self.assertEqual(token_of("(x > 1) enil {", "enil"), "Token.Name")
        self.assertEqual(token_of("pinnam x = 1.5;", "pinnam"), "Token.Name")


class TestLiterals(unittest.TestCase):
    def test_percentage_is_one_token(self):
        self.assertEqual(token_of("வரி = 18%;", "18%"), "Token.Literal.Number")
        self.assertEqual(token_of("accu 12.5%;", "12.5%"), "Token.Literal.Number")

    def test_numbers(self):
        self.assertEqual(
            token_of("accu 99.99;", "99.99"), "Token.Literal.Number.Float"
        )
        self.assertEqual(token_of("accu 1500;", "1500"), "Token.Literal.Number.Integer")

    def test_function_declaration_names_the_function(self):
        self.assertEqual(
            token_of("செயல் வரி_கணக்கு(வருவாய்) {", "வரி_கணக்கு"),
            "Token.Name.Function",
        )

    def test_bad_escape_is_an_error(self):
        lexer = ETamilLexer()
        tokens = list(lexer.get_tokens(r'accu "a\qb";'))
        self.assertTrue(any(token is Error for token, _ in tokens))


class TestCorpus(unittest.TestCase):
    def test_every_example_lexes_without_error(self):
        files = sorted(glob.glob(os.path.join(ROOT, "examples", "**", "*.qmz"),
                                 recursive=True))
        self.assertGreater(len(files), 0, "no example programs found")
        lexer = ETamilLexer()
        for path in files:
            with open(path, encoding="utf-8") as handle:
                source = handle.read()
            bad = [
                value
                for token, value in lexer.get_tokens(source)
                if token is Error and value.strip()
            ]
            self.assertEqual(bad, [], f"{os.path.basename(path)} produced Error tokens")


if __name__ == "__main__":
    unittest.main()
