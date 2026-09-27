// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//
// The numbers on the Marketplace page are the numbers in the compiler.
//
// README.md is the extension's listing. It claims how many keywords, spellings,
// builtins and `செயல்` functions the extension knows about, and every one of
// those claims is typed by hand while the real figure moves on its own — one
// commit to nUlakam and the page is wrong, with nothing anywhere to say so.
//
// It has gone wrong twice. The page offered "23 host builtins and all 122
// `செயல்` functions" long after they were 62 and 681; the fix said 681 on a day
// when the answer had already become 691. Both were found by a person reading
// the page, which is not a mechanism.
//
// So the page is held to `src/generated/language-data.ts`, which is generated
// from lexer.rs, parser.rs, interpreter.rs and nUlakam itself. A number that
// drifts now fails a test rather than shipping.
//
//   node --test test/counts.test.js
//
// Requires `npm run build`.

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { test, describe } = require('node:test');

const BUILT = path.join(__dirname, '..', 'out', 'generated', 'language-data.js');
const available = fs.existsSync(BUILT);
const data = available ? require(BUILT) : null;

const README = path.join(__dirname, '..', 'README.md');

/** The live figures, from the data the extension actually answers with. */
function live() {
  return {
    keywords: data.KEYWORDS.length,
    spellings: data.KEYWORDS.reduce((total, k) => total + k.forms.length, 0),
    builtins: data.FUNCTIONS.filter((f) => f.kind === 'builtin').length,
    stdlib: data.FUNCTIONS.filter((f) => f.kind === 'stdlib').length,
  };
}

/**
 * Each claim, as a pattern with one capture group.
 *
 * Anchored on the words around the number rather than on the number, so the
 * test fails when the figure is wrong and fails differently — "no claim
 * matched" — when somebody rewrites the sentence and takes the claim with it.
 * A test that quietly stops checking is worse than no test.
 */
const CLAIMS = [
  ['keywords', /All (\d+) keywords across \d+ spellings/],
  ['spellings', /All \d+ keywords across (\d+) spellings/],
  ['builtins', /(\d+) host builtins/],
  ['stdlib', /all (\d+) `செயல்` functions/],
];

describe('the README', { skip: available ? false : 'run npm run build' }, () => {
  const readme = fs.readFileSync(README, 'utf8');
  const actual = live();

  for (const [what, pattern] of CLAIMS) {
    test(`claims the right number of ${what}`, () => {
      const found = readme.match(pattern);
      assert.ok(
        found,
        `no claim about ${what} matched ${pattern} — was the sentence rewritten?`
      );
      assert.equal(
        Number(found[1]),
        actual[what],
        `README says ${found[1]} ${what}; the generated data says ${actual[what]}`
      );
    });
  }

  test('the example count is right, spelled out', () => {
    // Written as a word on the page, so it needs the small table rather than a
    // regex on digits. Worth checking: it is claimed twice.
    const WORDS = {
      'twenty-seven': 27,
      'twenty-eight': 28,
      'twenty-nine': 29,
      thirty: 30,
      'thirty-one': 31,
      'thirty-two': 32,
      'thirty-three': 33,
      'thirty-four': 34,
      'thirty-five': 35,
      'thirty-six': 36,
      'thirty-seven': 37,
      'thirty-eight': 38,
      'thirty-nine': 39,
      forty: 40,
    };
    const examples = countExamples();
    // Whole words: "thirty-two" contains "thirty", and a substring match
    // read that as a second, wrong, claim.
    const claimed = Object.keys(WORDS).filter((word) =>
      new RegExp(`(?<![A-Za-z0-9_-])${word}(?![A-Za-z0-9_-])`).test(readme)
    );
    assert.ok(
      claimed.length > 0,
      `the README claims no example count near ${examples} — ` +
        'add the word to WORDS here if the number outgrew the table'
    );
    for (const word of claimed) {
      assert.equal(
        WORDS[word],
        examples,
        `README says "${word}" example programs; there are ${examples}`
      );
    }
  });
});

/** How many `.qmz` files the packaging script would carry. */
function countExamples() {
  const root = path.join(__dirname, '..', '..', 'examples');
  let total = 0;
  const walk = (directory) => {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const full = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        walk(full);
      } else if (entry.name.endsWith('.qmz')) {
        total += 1;
      }
    }
  };
  walk(root);
  return total;
}
