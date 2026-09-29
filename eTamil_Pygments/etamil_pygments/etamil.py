"""
    pygments.lexers.etamil
    ~~~~~~~~~~~~~~~~~~~~~~

    Lexer for eTamil, a programming language whose keywords are Tamil words.

    The word lists live in `_etamil_builtins.py` and are generated from the
    compiler's own lexer, so the vocabulary here cannot drift from the language.
    Everything that decides how a word is *coloured* is in this file.

    :copyright: Copyright 2026 Mohammed Maruff (Esan Maruff).
    :license: BSD, see LICENSE for details.
"""

from pygments.lexer import RegexLexer, bygroups, words
from pygments.token import (Comment, Error, Keyword, Name, Number, Operator,
                            Punctuation, String, Whitespace)

from ._etamil_builtins import (BUILTIN, CONSTANT, CONTROL, DECLARE_FUNCTION,
                               DOMAIN, IMPORT, LOGICAL, NAMED_CONSTANT,
                               RESERVED, TYPE)

__all__ = ['ETamilLexer']

# Tamil is U+0B80-U+0BFF. The block includes the combining vowel signs and the
# virama, which is why `\b` is no use here: `எனில்` ends in U+0BCD, category Mn,
# which `\w` does not match, so `\bஎனில்\b` never fires. Word boundaries have to
# be spelled out against the identifier class the compiler's lexer accepts.
_ID = r'[a-zA-Z0-9_஀-௿]'
_IDENT = r'[a-zA-Z_஀-௿][a-zA-Z0-9_஀-௿]*'

_PRE = r'(?<!' + _ID + r')'
_SUF = r'(?!' + _ID + r')'


def _words(vocabulary):
    """A word alternation anchored on the identifier class."""
    return words(vocabulary, prefix=_PRE, suffix=_SUF)


class ETamilLexer(RegexLexer):
    """For eTamil source code."""

    name = 'eTamil'
    url = 'https://github.com/Maruff/eTamil_lang'
    aliases = ['etamil']
    filenames = ['*.qmz']
    mimetypes = ['text/x-etamil']
    version_added = ''

    tokens = {
        'root': [
            (r'\s+', Whitespace),

            # eTamil has line comments only.
            (r'//.*?$', Comment.Single),
            (r'"', String.Double, 'string'),

            # A percentage is a single literal in eTamil: 18% is one number,
            # not 18 followed by an operator. It has to precede the plain
            # number rules or they would eat the digits and leave the %.
            (r'\d+(?:\.\d+)?%', Number),
            (r'\d+\.\d+', Number.Float),
            (r'\d+', Number.Integer),

            # செயல் <name> — a declaration names the function that follows.
            (_PRE + r'(' + r'|'.join(DECLARE_FUNCTION) + r')(\s+)(' + _IDENT + r')',
             bygroups(Keyword.Declaration, Whitespace, Name.Function)),

            (_words(IMPORT), Keyword.Namespace),
            (_words(CONTROL), Keyword),
            (_words(TYPE), Keyword.Type),
            (_words(CONSTANT), Keyword.Constant),
            (_words(RESERVED), Keyword.Reserved),
            (_words(LOGICAL), Operator.Word),
            (_words(NAMED_CONSTANT), Name.Constant),
            (_words(BUILTIN), Name.Builtin),

            # Domain vocabulary — வரவு, பற்று, இருப்பு and the rest. The parser
            # accepts these as names ("தொகை is a perfectly good name for an
            # amount"), so colouring them as keywords would tell the reader the
            # opposite of the truth. Pseudo says: known word, not reserved.
            (_words(DOMAIN), Name.Builtin.Pseudo),

            (_IDENT, Name),
            (r'==|!=|>=|<=|&|\+|-|\*|/|>|<|=', Operator),
            (r'[{}()\[\];,.?:]', Punctuation),
        ],
        'string': [
            (r'\\[ntr"\\]', String.Escape),
            (r'\\.', Error),
            (r'"', String.Double, '#pop'),
            (r'[^"\\]+', String.Double),
        ],
    }
