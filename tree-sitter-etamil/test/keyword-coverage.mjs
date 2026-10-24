// Which keywords does the corpus never exercise?
//
//   node test/keyword-coverage.mjs           # report
//   node test/keyword-coverage.mjs --check   # exit 1 if any keyword the grammar uses is untested
//
// keywords.js lists every spelling the compiler accepts, and grammar.js turns each
// keyword it references into a rule. A keyword with a rule but no corpus test is a
// rule nobody has pinned: it can break, or be silently changed, without a failure.
// This reads the hand-written corpus in test/corpus/ (the shapes of individual
// constructs, as opposed to test/parse-corpus.mjs, which parses real programs) and
// reports every referenced keyword with no spelling in it.
//
// A spelling counts only as a whole word: `if` is not found inside `diff`, and a
// Tamil spelling is not found inside a longer Tamil word that contains it.

import { createRequire } from 'node:module';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const GRAMMAR_DIR = join(HERE, '..');
const require = createRequire(import.meta.url);

const keywords = require(join(GRAMMAR_DIR, 'keywords.js'));
const grammar = readFileSync(join(GRAMMAR_DIR, 'grammar.js'), 'utf8');
const corpus = readdirSync(join(HERE, 'corpus'))
  .map((file) => readFileSync(join(HERE, 'corpus', file), 'utf8'))
  .join('\n');

const used = [...new Set([...grammar.matchAll(/kw\('([A-Za-z0-9_]+)'\)/g)].map((m) => m[1]))].sort();

const escape = (text) => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const WORD = '[\\p{L}\\p{M}\\p{N}_]';
const appearsIn = (text, spelling) =>
  new RegExp(`(^|[^\\p{L}\\p{M}\\p{N}_])${escape(spelling)}($|[^\\p{L}\\p{M}\\p{N}_])`, 'u').test(text);

const unknown = used.filter((name) => !keywords[name]);
const untested = used.filter(
  (name) => keywords[name] && !keywords[name].some((spelling) => appearsIn(corpus, spelling))
);

console.log(
  `${Object.keys(keywords).length} keywords in keywords.js, ${used.length} used by grammar.js, ` +
    `${used.length - untested.length - unknown.length} exercised by test/corpus, ${untested.length} not`
);
if (unknown.length) {
  console.log(`\ngrammar.js names keywords keywords.js does not define: ${unknown.join(', ')}`);
}
if (untested.length) {
  console.log('\nNot exercised by any corpus test:');
  for (const name of untested) {
    console.log(`  ${name.padEnd(18)} ${keywords[name].join('  ')}`);
  }
}

if (process.argv.includes('--check') && (untested.length || unknown.length)) {
  process.exit(1);
}
