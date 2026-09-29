// Scope tests for the generated TextMate grammar.
//
// The grammar is generated, so what needs testing is not the keyword list —
// that is derived from lexer.rs and checked by
// `generate_editor_support.py --check`. What needs testing is that the
// patterns *resolve* the way they are meant to, and one assumption in
// particular: that `\b` behaves correctly around Tamil text.
//
// It does, but not obviously. A Tamil letter is frequently a consonant plus a
// combining vowel sign or pulli, and Oniguruma counts marks as word
// characters — which is what makes `\bஇல்\b` refuse to match inside இல்லை,
// and what makes `\bஎண்\b` refuse to match inside எண்ணி (a real variable in
// nUlakam/atippatY/col.qmz). Both cases are asserted below, because if that
// assumption were wrong the grammar would mis-scope ordinary library code
// and nobody would notice from reading it.
//
//   node --test test/
//
// Requires the dev dependencies: vscode-textmate and vscode-oniguruma.

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { test, before, describe } = require('node:test');

const vsctm = require('vscode-textmate');
const oniguruma = require('vscode-oniguruma');

const GRAMMAR_PATH = path.join(__dirname, '..', 'syntaxes', 'etamil.tmLanguage.json');
const REPO_ROOT = path.join(__dirname, '..', '..');

let grammar;

before(async () => {
  const wasmPath = path.join(
    require.resolve('vscode-oniguruma'),
    '..',
    '..',
    'release',
    'onig.wasm'
  );
  await oniguruma.loadWASM(fs.readFileSync(wasmPath));

  const registry = new vsctm.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (patterns) => new oniguruma.OnigScanner(patterns),
      createOnigString: (s) => new oniguruma.OnigString(s),
    }),
    loadGrammar: async (scopeName) => {
      if (scopeName !== 'source.etamil') {
        return null;
      }
      const raw = fs.readFileSync(GRAMMAR_PATH, 'utf8');
      return vsctm.parseRawGrammar(raw, GRAMMAR_PATH);
    },
  });

  grammar = await registry.loadGrammar('source.etamil');
  assert.ok(grammar, 'grammar failed to load');
});

/** Every token on one line, as { text, scopes }. */
function tokenize(line) {
  const result = grammar.tokenizeLine(line, vsctm.INITIAL);
  return result.tokens.map((token) => ({
    text: line.substring(token.startIndex, token.endIndex),
    scopes: token.scopes,
  }));
}

/** The most specific scope the grammar gave to `text` on `line`. */
function scopeOf(line, text) {
  const token = tokenize(line).find((t) => t.text === text);
  assert.ok(token, `"${text}" was not a single token on: ${line}`);
  return token.scopes[token.scopes.length - 1];
}

/**
 * Whether each line's text is scoped English, tokenizing the lines in
 * sequence so a rule that spans them keeps its state — which is the whole
 * point of the block rule for Rule 2's marks.
 *
 * Returns one string per line: `E` where the character is inside
 * `meta.english.comment.etamil`, `.` where it is not.
 */
function englishMap(lines) {
  let stack = vsctm.INITIAL;
  return lines.map((line) => {
    const result = grammar.tokenizeLine(line, stack);
    stack = result.ruleStack;
    return result.tokens
      .map((token) => {
        const english = token.scopes.some((scope) => scope.includes('meta.english'));
        return (english ? 'E' : '.').repeat(token.endIndex - token.startIndex);
      })
      .join('');
  });
}

describe('an English comment across lines', () => {
  // The marks go at the ends of the sentence, not of each line. The previous
  // grammar nested a single-line `__.+?__` inside the line-comment rule, so a
  // sentence spanning two lines left each line holding one unmatched mark and
  // the English between them was coloured as ezuqqu.
  test('the example SCRIPT_RULES.md prints for Rule 2 is English on both lines', () => {
    const [first, second] = englishMap([
      '// __Rounded to paise once, at the end.',
      '// computed adds money that was never there.__',
    ]);
    assert.ok(first.includes('E'), 'nothing on the opening line was English');
    assert.ok(second.startsWith('E'), 'the continuation line was not English');
  });

  test('the region closes, and code after it is not English', () => {
    const map = englishMap(['// __English from here', '// to here.__', 'moqqam = 5;']);
    assert.ok(map[1].includes('E'));
    assert.equal(map[2].includes('E'), false);
  });

  test('an unclosed mark stops at the first line that is not a comment', () => {
    // Otherwise one stray `__` would colour the remainder of the file English.
    const map = englishMap(['// __opened and never closed', 'moqqam = 5;', '// kaZakku']);
    assert.ok(map[0].includes('E'));
    assert.equal(map[1].includes('E'), false);
    assert.equal(map[2].includes('E'), false, 'the comment after the code was English');
  });

  test('a marked run that does not start the comment still works', () => {
    assert.equal(
      scopeOf('// kaZakku __two words__ mudivu', '__two words__'),
      'meta.english.comment.etamil'
    );
  });
});

describe('keywords', () => {
  test('Tamil control flow', () => {
    assert.equal(
      scopeOf('(x > 1) எனில் {', 'எனில்'),
      'keyword.control.conditional.etamil'
    );
    assert.equal(scopeOf('இன்றேல் {', 'இன்றேல்'), 'keyword.control.conditional.etamil');
    assert.equal(scopeOf('(x < 3) சுற்று {', 'சுற்று'), 'keyword.control.loop.etamil');
  });

  // The romanization the compiler actually accepts. The previous grammar
  // shipped the pre-`Z` scheme, so these five were unhighlighted while the
  // spellings it did highlight were rejected by the lexer.
  test('romanized control flow uses the Z scheme', () => {
    assert.equal(
      scopeOf('(x > 1) eZil {', 'eZil'),
      'keyword.control.conditional.etamil'
    );
    assert.equal(scopeOf('iZREl {', 'iZREl'), 'keyword.control.conditional.etamil');
    assert.equal(scopeOf('piZZam x = 1.5;', 'piZZam'), 'storage.type.etamil');
    assert.equal(scopeOf('accu poy;', 'poy'), 'constant.language.boolean.etamil');
    assert.equal(scopeOf('accu iZmY;', 'iZmY'), 'constant.language.null.etamil');
  });

  test('the old romanization is not highlighted as a keyword', () => {
    // `enil` is not a keyword; the compiler rejects it. It must read as an
    // ordinary name so the mistake is visible rather than reassuring.
    assert.equal(scopeOf('(x > 1) enil {', 'enil'), 'variable.other.etamil');
    assert.equal(scopeOf('pinnam x = 1.5;', 'pinnam'), 'variable.other.etamil');
  });

  test('features the old grammar omitted entirely', () => {
    assert.equal(
      scopeOf('செயல் f() {', 'செயல்'),
      'keyword.declaration.function.etamil'
    );
    assert.equal(scopeOf('திரும்பு 1;', 'திரும்பு'), 'keyword.control.flow.return.etamil');
    assert.equal(scopeOf('ஒவ்வொரு உ இல் அ {', 'ஒவ்வொரு'), 'keyword.control.loop.etamil');
    assert.equal(scopeOf('ஒவ்வொரு உ இல் அ {', 'இல்'), 'keyword.control.loop.etamil');
    assert.equal(scopeOf('இறக்கு "nUlakam/atippatY/col.qmz";', 'இறக்கு'), 'keyword.control.import.etamil');
    assert.equal(scopeOf('(a மற்றும் b) எனில் {', 'மற்றும்'), 'keyword.operator.logical.etamil');
    assert.equal(scopeOf('ஈர்ம கொடி = மெய்;', 'ஈர்ம'), 'storage.type.etamil');
    assert.equal(scopeOf('ஈர்ம கொடி = மெய்;', 'மெய்'), 'constant.language.boolean.etamil');
  });

  // Financial nouns are *not* reserved — README is explicit that தொகை is a
  // fine name for an amount — so they must not be scoped as syntax.
  test('domain vocabulary is vocabulary, not syntax', () => {
    assert.equal(scopeOf('தொகை = 1500;', 'தொகை'), 'support.type.domain.etamil');
    assert.equal(scopeOf('வரி = 18%;', 'வரி'), 'support.type.domain.etamil');
    // மாறி and நிலை have no statement syntax and are ordinary names to the
    // parser. The old grammar scoped them keyword.declaration, which told the
    // reader `மாறி x = 5;` should work. It does not.
    assert.equal(scopeOf('மாறி = 5;', 'மாறி'), 'support.type.domain.etamil');
    assert.equal(scopeOf('நிலை = 5;', 'நிலை'), 'support.type.domain.etamil');
  });

  test('reserved words that only ever produce an error still read as reserved', () => {
    assert.equal(scopeOf('உடல்', 'உடல்'), 'keyword.other.http.etamil');
    assert.equal(scopeOf('வரிசை', 'வரிசை'), 'keyword.other.sql.etamil');
  });
});

// The assumption the whole \b-anchored approach rests on.
describe('word boundaries around Tamil', () => {
  test('a keyword is not matched inside a longer Tamil identifier', () => {
    // எண்ணி — the loop counter used throughout nUlakam/atippatY/col.qmz — begins with
    // the type keyword எண். It must scope as one identifier, not as a type
    // followed by a fragment.
    const tokens = tokenize('எண்ணி = 0;').filter((t) => t.text.trim());
    assert.equal(tokens[0].text, 'எண்ணி');
    assert.equal(tokens[0].scopes[tokens[0].scopes.length - 1], 'variable.other.etamil');
  });

  test('a short keyword does not shadow a longer one that contains it', () => {
    // இல் (In) is a prefix of இல்லை (Not), and they carry different scopes
    // from different patterns, with In's pattern tried first.
    assert.equal(scopeOf('(இல்லை a) எனில் {', 'இல்லை'), 'keyword.operator.logical.etamil');
  });

  test('an underscore keeps a compound keyword whole', () => {
    assert.equal(
      scopeOf('கோப்பு_திற "f.txt", "write";', 'கோப்பு_திற'),
      'support.function.fileio.etamil'
    );
    // கோப்பு alone is also a keyword; it must not win inside the compound.
    const tokens = tokenize('கோப்பு_திற "f.txt", "write";');
    assert.ok(
      tokens.some((t) => t.text === 'கோப்பு_திற'),
      'the compound was split at the underscore'
    );
  });
});

describe('literals', () => {
  // The old pattern was \b\d+(?:\.\d+)?%\b — `%` is not a word character, so
  // the trailing \b could never match before `;` or end of line and every
  // percentage literal fell through to the plain number rule.
  test('a percentage literal is scoped, including at end of statement', () => {
    assert.equal(scopeOf('வரி = 18%;', '18%'), 'constant.numeric.percentage.etamil');
    assert.equal(scopeOf('accu 20%', '20%'), 'constant.numeric.percentage.etamil');
    assert.equal(scopeOf('accu 12.5%;', '12.5%'), 'constant.numeric.percentage.etamil');
  });

  test('a plain number is a number', () => {
    assert.equal(scopeOf('accu 1500;', '1500'), 'constant.numeric.decimal.etamil');
    assert.equal(scopeOf('accu 99.99;', '99.99'), 'constant.numeric.decimal.etamil');
  });

  test('digit separators are not advertised', () => {
    // The lexer rejects 1_000. Highlighting it as a number promised a
    // literal the compiler does not have.
    const tokens = tokenize('accu 1_000;');
    const number = tokens.find((t) => t.text === '1_000');
    assert.equal(number, undefined, '1_000 should not scope as a single number');
  });

  test('string escapes distinguish the five the lexer decodes', () => {
    assert.equal(scopeOf('accu "a\\nb";', '\\n'), 'constant.character.escape.etamil');
    // An unknown escape keeps both characters rather than being decoded, so
    // it is worth showing as different.
    assert.equal(
      scopeOf('accu "C:\\kaNakku";', '\\k'),
      'invalid.illegal.unknown-escape.etamil'
    );
  });

  test('there is no block comment and no single-quoted string', () => {
    // Both were in the old grammar. The lexer has neither, so a /* */ region
    // greyed out code that then failed to lex, and a stray apostrophe
    // started a phantom string.
    const block = tokenize('/* not a comment */');
    assert.ok(
      !block.some((t) => t.scopes.some((s) => s.startsWith('comment'))),
      '/* */ must not scope as a comment'
    );
    const quoted = tokenize("accu 'x';");
    assert.ok(
      !quoted.some((t) => t.scopes.some((s) => s.startsWith('string'))),
      "'...' must not scope as a string"
    );
  });

  test('a line comment is a comment', () => {
    assert.ok(
      tokenize('// வணக்கம்')[1].scopes.includes('comment.line.double-slash.etamil')
    );
  });
});

describe('structure', () => {
  test('a definition names its function', () => {
    assert.equal(scopeOf('செயல் வரி_கணக்கு(வருவாய்) {', 'வரி_கணக்கு'), 'entity.name.function.etamil');
  });

  test('a builtin call keeps its own scope', () => {
    assert.equal(scopeOf('accu நீளம்("abc");', 'நீளம்'), 'support.function.builtin.etamil');
    assert.equal(scopeOf('accu nILam("abc");', 'nILam'), 'support.function.builtin.etamil');
  });

  test('a standard library call is marked as library', () => {
    assert.equal(scopeOf('accu ரூபாய்(1500);', 'ரூபாய்'), 'support.function.stdlib.etamil');
  });

  test('an unknown call is still a call', () => {
    assert.equal(scopeOf('என்_செயல்(1);', 'என்_செயல்'), 'entity.name.function.etamil');
  });

  test('a record key is a member, not a keyword', () => {
    // {வரி: 100} produces the field வரி — a field name is data, what the
    // author typed, so it must not read as the Tax keyword.
    assert.equal(scopeOf('ர = {வரி: 100};', 'வரி'), 'variable.other.member.etamil');
  });

  test('the try operator is an operator', () => {
    assert.equal(scopeOf('மதிப்பு = எண்ணாக்கு(உ)?;', '?'), 'keyword.operator.try.etamil');
  });
});

// The real programs in the repository are the useful end-to-end check: if any
// line of the standard library or the examples ends up with an unscoped run
// of Tamil text, the grammar has a hole.
describe('the repository tokenizes without holes', () => {
  const sources = [
    'nUlakam/atippatY/col.qmz',
    'nUlakam/paNam/paNam.qmz',
    'nUlakam/kaNakkiyal/pErEtu.qmz',
    'examples/basic_samples/example.qmz',
  ];

  for (const relative of sources) {
    test(relative, () => {
      const file = path.join(REPO_ROOT, relative);
      if (!fs.existsSync(file)) {
        return; // the example set moves; absence is not a grammar failure
      }
      const lines = fs.readFileSync(file, 'utf8').split(/\r?\n/);
      let ruleStack = vsctm.INITIAL;

      for (const [index, line] of lines.entries()) {
        const result = grammar.tokenizeLine(line, ruleStack);
        ruleStack = result.ruleStack;

        for (const token of result.tokens) {
          const text = line.substring(token.startIndex, token.endIndex);
          if (!text.trim()) {
            continue;
          }
          // `source.etamil` alone means no rule claimed the text.
          assert.ok(
            token.scopes.length > 1,
            `${relative}:${index + 1} left "${text}" unscoped`
          );
        }
      }
    });
  }
});
