// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//
// Which ASCII gets drawn in the eTamil font.
//
// The scanner decides where one font stops and the other starts, and it
// decides it from two marks in the file rather than from anything the compiler
// knows. What is asserted here is the boundary in both directions: that eTamil
// ASCII is found, and — the half that matters more — that English ASCII is
// never returned, because a false positive renders an English word in Tamil
// glyphs and a false negative renders eTamil in Latin, which is only the
// ordinary view of the file.
//
//   node --test test/marks.test.js
//
// Requires `npm run build`; this loads out/marks.js, which imports nothing
// from the vscode API so that it can be loaded here at all.

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { test, describe } = require('node:test');

const BUILT = path.join(__dirname, '..', 'out', 'marks.js');
const available = fs.existsSync(BUILT);
const { scanETamilScript } = available ? require(BUILT) : { scanETamilScript: null };

/** The text each span covers, which is what the assertions are really about. */
function painted(source) {
  const lines = source.split('\n');
  return scanETamilScript(source).map((span) =>
    lines[span.line].slice(span.start, span.end)
  );
}

describe('eTamil-script ASCII', { skip: available ? false : 'run npm run build' }, () => {
  test('an unmarked name is eTamil', () => {
    assert.deepEqual(painted('moqqam = 0;'), ['moqqam']);
  });

  test('a marked name is English and is left alone', () => {
    assert.deepEqual(painted('_sum = 0;'), []);
  });

  test('the mark covers the whole name, Tamil parts and all', () => {
    assert.deepEqual(painted('_மொத்த_cgst = 0;'), []);
  });

  test('an unmarked name is painted letter run by letter run', () => {
    assert.deepEqual(painted('qaravu_paqivu = 0;'), ['qaravu', 'paqivu']);
  });

  test('Tamil script is never painted: both fonts draw it', () => {
    assert.deepEqual(painted('மொத்தம் = 0;'), []);
  });

  test('digits are not painted: the font does not remap them', () => {
    assert.deepEqual(painted('kaN1 = 25;'), ['kaN']);
  });

  test('a keyword spelled in ezuqqu is eTamil', () => {
    assert.deepEqual(painted('ceyal kUttu(a, b) {'), ['ceyal', 'kUttu', 'a', 'b']);
  });

  test('a keyword spelled in English is not', () => {
    assert.deepEqual(painted('_fn _add(_a, _b) {'), []);
  });
});

describe('comments', { skip: available ? false : 'run npm run build' }, () => {
  test('an unmarked comment is eTamil', () => {
    assert.deepEqual(painted('// moqqam kaNakku'), ['moqqam', 'kaNakku']);
  });

  test('a marked comment is English', () => {
    assert.deepEqual(painted('// __the total of the ledger__'), []);
  });

  test('Tamil inside the marks stays unpainted, and so does the English', () => {
    assert.deepEqual(painted('// __the total of மொத்தம், once__'), []);
  });

  test('two marked runs on one line are two, not one', () => {
    // A greedy `__.+__` would swallow `ceyal` between them and stop it being
    // painted, which is the bug this asserts is not there.
    assert.deepEqual(painted('// __first__ ceyal __second__'), ['ceyal']);
  });

  test('the licence header is exempt and is never painted', () => {
    const header =
      '// SPDX-License-Identifier: AGPL-3.0-or-later\n' +
      '// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>';
    assert.deepEqual(painted(header), []);
  });

  test('a rule of dashes has no letters and no spans', () => {
    assert.deepEqual(painted('// ====================='), []);
  });

  test('code before a comment is still code', () => {
    assert.deepEqual(painted('moqqam = 1;  // __one__'), ['moqqam']);
  });
});

describe('an English comment across lines', { skip: available ? false : 'run npm run build' }, () => {
  // `__` opens a region rather than matching a pair on one line. Scanning each
  // line on its own left the first and last lines of a block each holding one
  // unmatched mark, so neither counted and every English word between them was
  // painted as Tamil — the unreadable direction, not a missed span.

  test('the example SCRIPT_RULES.md prints for Rule 2 is English throughout', () => {
    assert.deepEqual(
      painted(`// __Rounded to paise once, at the end. Rounding each share as it is
// computed adds money that was never there.__`),
      []
    );
  });

  test('a block of three lines, Tamil in the middle, is English throughout', () => {
    assert.deepEqual(
      painted(`// __viLimpu_nivAraNam runs the whole ladder twice — there is
// no closed form for it, so kaZakku is done twice and
// compared.__`),
      []
    );
  });

  test('the region closes, and the comment after it is eTamil again', () => {
    assert.deepEqual(
      painted(`// __English here.__
// kaZakku mudivu`),
      ['kaZakku', 'mudivu']
    );
  });

  test('an unclosed mark ends at the first line with no comment on it', () => {
    // Otherwise one stray `__` would turn the rest of the file English.
    assert.deepEqual(
      painted(`// __opened and never closed
moqqam = 5;
// kaZakku mudivu`),
      ['moqqam', 'kaZakku', 'mudivu']
    );
  });

  test('a line that opens and closes is still the ordinary single-line case', () => {
    assert.deepEqual(painted('// kaZakku __two words__ mudivu'), ['kaZakku', 'mudivu']);
  });

  test('code above a block is unaffected by what the block opens', () => {
    assert.deepEqual(
      painted(`moqqam = 5;
// __English from here
// to here.__`),
      ['moqqam']
    );
  });
});

describe('names reached through a dot', { skip: available ? false : 'run npm run build' }, () => {
  test('a field name is not painted, but the record it hangs off is', () => {
    assert.deepEqual(painted('moqqam = cAzRu.varuvAy;'), ['moqqam', 'cAzRu']);
  });

  test('the whole field name goes, mixed ones included', () => {
    assert.deepEqual(painted('r.மொத்த_cgst = 0;'), ['r']);
  });

  test('a chain leaves only the head painted', () => {
    assert.deepEqual(painted('a.b.c = 1;'), ['a']);
  });

  test('a field written in ezuqqu is still data, and still ISO', () => {
    assert.deepEqual(painted('இ.moqqam = 5;'), []);
  });

  test('an already-marked field name is no different', () => {
    assert.deepEqual(painted('r._sum = 0;'), ['r']);
  });

  test('an extension in a comment is English, whatever surrounds it', () => {
    assert.deepEqual(painted('// kOppu .qmz vakai'), ['kOppu', 'vakai']);
  });

  test('a dotted name in a comment keeps its stem and drops its extension', () => {
    assert.deepEqual(painted('// paNam.qmz'), ['paNam']);
  });

  test('a decimal has no letters to paint either way', () => {
    assert.deepEqual(painted('vIqam = 18.5;'), ['vIqam']);
  });
});

describe('strings', { skip: available ? false : 'run npm run build' }, () => {
  test('a string is data and is never painted', () => {
    assert.deepEqual(painted('அச்சு "Total due";'), []);
  });

  test('a // inside a string does not start a comment', () => {
    assert.deepEqual(painted('vilY = "https://etamil.in"; moqqam = 1;'), [
      'vilY',
      'moqqam',
    ]);
  });

  test('an escaped quote does not end the string', () => {
    assert.deepEqual(painted('col = "a \\" b"; moqqam = 1;'), ['col', 'moqqam']);
  });

  test('a string that runs across lines keeps running', () => {
    // The lexer's `"([^"\\]|\\.)*"` does not exclude a newline, so it can.
    assert.deepEqual(painted('col = "one\ntwo"; moqqam = 1;'), ['col', 'moqqam']);
  });
});

describe('offsets', { skip: available ? false : 'run npm run build' }, () => {
  test('a span names the line and the columns VS Code counts', () => {
    const spans = scanETamilScript('மொத்தம் = 0;\nceyal f() {}');
    assert.equal(spans[0].line, 1);
    assert.equal(spans[0].start, 0);
    assert.equal(spans[0].end, 5);
  });

  test('columns are counted past Tamil, which is one unit per letter here', () => {
    const source = 'மொ = ceyal;';
    const [span] = scanETamilScript(source);
    assert.equal(source.slice(span.start, span.end), 'ceyal');
  });
});

describe('the library it was written for', { skip: available ? false : 'run npm run build' }, () => {
  test('nothing in a real nUlakam module is painted, because it obeys the rules', () => {
    // Every comment in the library is marked and every English name carries
    // its underscore, so the only ASCII left to paint is the romanized
    // spellings — of which the library uses none, being written in Tamil.
    // A regression in the scanner shows up here as a span that should not
    // exist, on real code rather than on an example written to pass.
    const module = path.join(__dirname, '..', '..', 'nUlakam', 'atippatY', 'kaNiqam.qmz');
    const source = fs.readFileSync(module, 'utf8');
    const spans = painted(source);
    assert.deepEqual(spans, [], `unexpected eTamil-script spans: ${spans.join(', ')}`);
  });
});
