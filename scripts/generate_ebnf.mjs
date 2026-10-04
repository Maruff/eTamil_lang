#!/usr/bin/env node
// Write the language's grammar as EBNF, from the grammar that parses it.
//
//   node scripts/generate_ebnf.mjs            # rewrite the generated files
//   node scripts/generate_ebnf.mjs --check    # fail if they are out of date (CI)
//
// The source is tree-sitter-etamil/grammar.js. That grammar is already held to the
// language by two gates: its keyword vocabulary is generated from the compiler's
// lexer, and `npm run parse:corpus` parses every real program in the repository
// with it. Turning it into EBNF here, rather than writing EBNF by hand beside it,
// means the published grammar cannot drift from the one that is tested.
//
// How: grammar.js is a JavaScript DSL (seq, choice, repeat, ...). It is run here
// with those functions replaced by ones that build a tree instead of a parser, and
// the tree is printed. Nothing is reimplemented, so a construct the DSL gains
// that this does not know fails loudly rather than being skipped.
//
// Outputs:
//   docs/reference/etamil.ebnf        the productions, for tools and readers
//   docs/reference/LANGUAGE_SPEC.md   the blocks between GENERATED markers: the
//                                     same productions, and the precedence table
//
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>

import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const GRAMMAR = join(ROOT, 'tree-sitter-etamil', 'grammar.js');
const EBNF = join(ROOT, 'docs', 'reference', 'etamil.ebnf');
const SPEC = join(ROOT, 'docs', 'reference', 'LANGUAGE_SPEC.md');
const WIDTH = 96;

// ---- Run grammar.js with a DSL that builds a tree --------------------------------

function loadGrammar() {
  const norm = (x) => {
    if (typeof x === 'string') return { t: 'str', v: x };
    if (x instanceof RegExp) return { t: 're', v: x.source };
    if (x && typeof x === 'object' && x.t) return x;
    throw new Error(`unsupported DSL value: ${String(x)}`);
  };
  const list = (args) => args.map(norm);

  const seq = (...a) => ({ t: 'seq', a: list(a) });
  const choice = (...a) => ({ t: 'choice', a: list(a) });
  const repeat = (a) => ({ t: 'rep', a: norm(a) });
  const repeat1 = (a) => ({ t: 'rep1', a: norm(a) });
  const optional = (a) => ({ t: 'opt', a: norm(a) });
  const blank = () => ({ t: 'seq', a: [] });
  const field = (name, a) => ({ t: 'field', name, a: norm(a) });
  const alias = (a) => norm(a);

  // prec('level', rule), prec.left(rule), prec.right(1, rule) ...
  const makePrec = (assoc) => (...args) => {
    const rule = args[args.length - 1];
    const level = args.length > 1 ? args[0] : null;
    return { t: 'prec', assoc, level, a: norm(rule) };
  };
  const prec = makePrec(null);
  prec.left = makePrec('left');
  prec.right = makePrec('right');
  prec.dynamic = makePrec('dynamic');

  const token = (a) => ({ t: 'tok', a: norm(a) });
  token.immediate = (a) => ({ t: 'tok', imm: true, a: norm(a) });

  let definition = null;
  const grammar = (d) => (definition = d);

  // The keyword spellings are the generated part, and are listed in KEYWORDS.md, so
  // here each keyword is one marker. kw('If') then gives <If>, not 'எனில்' | 'eZil'.
  const keywords = new Proxy({}, { get: (_, name) => (typeof name === 'string' ? [`@kw:${name}`] : undefined) });
  const fakeRequire = (id) => {
    if (id === './keywords.js') return keywords;
    throw new Error(`grammar.js requires ${id}, which this generator does not provide`);
  };

  const names = ['require', 'module', 'exports', 'grammar', 'seq', 'choice', 'repeat', 'repeat1', 'optional', 'token', 'prec', 'field', 'alias', 'blank'];
  const code = readFileSync(GRAMMAR, 'utf8');
  const run = new Function(...names, code);
  run(fakeRequire, { exports: {} }, {}, grammar, seq, choice, repeat, repeat1, optional, token, prec, field, alias, blank);
  if (!definition) throw new Error('grammar.js did not call grammar()');

  const $ = new Proxy({}, { get: (_, name) => ({ t: 'sym', name }) });
  const rules = Object.entries(definition.rules).map(([name, fn]) => [name, norm(fn($))]);
  return {
    name: definition.name,
    rules,
    precedences: definition.precedences ? definition.precedences($) : [],
    conflicts: definition.conflicts ? definition.conflicts($).map((pair) => pair.map((s) => s.name)) : [],
  };
}

// ---- Simplify and print ----------------------------------------------------------

const shown = (name) => name.replace(/^_/, '');

function simplify(n) {
  switch (n.t) {
    case 'field':
    case 'prec':
      return simplify(n.a);
    case 'seq': {
      const parts = n.a.flatMap((c) => {
        const s = simplify(c);
        return s.t === 'seq' ? s.a : [s];
      });
      return parts.length === 1 ? parts[0] : { t: 'seq', a: parts };
    }
    case 'choice': {
      const parts = n.a.flatMap((c) => {
        const s = simplify(c);
        return s.t === 'choice' ? s.a : [s];
      });
      const seen = new Set();
      const unique = parts.filter((p) => {
        const key = print(p, 0);
        return seen.has(key) ? false : seen.add(key);
      });
      return unique.length === 1 ? unique[0] : { t: 'choice', a: unique };
    }
    case 'rep':
    case 'rep1':
    case 'opt':
    case 'tok':
      return { ...n, a: simplify(n.a) };
    default:
      return n;
  }
}

const quote = (v) => (v.includes('"') ? `'${v}'` : `"${v.replace(/\\/g, '\\\\').replace(/\n/g, '\\n')}"`);

// ctx: 0 = alternatives, 1 = a sequence item context, 2 = operand of * + ?
function print(n, ctx) {
  const wrap = (s, needs) => (needs ? `(${s})` : s);
  switch (n.t) {
    case 'sym':
      return shown(n.name);
    case 'str':
      return n.v.startsWith('@kw:') ? `<${n.v.slice(4)}>` : quote(n.v);
    case 're':
      return `/${n.v}/`;
    case 'seq':
      return wrap(n.a.map((c) => print(c, 1)).join(' '), ctx >= 2);
    case 'choice':
      return wrap(n.a.map((c) => print(c, 0)).join(' | '), ctx >= 1);
    case 'opt':
      return `${print(n.a, 2)}?`;
    case 'rep':
      return `${print(n.a, 2)}*`;
    case 'rep1':
      return `${print(n.a, 2)}+`;
    case 'tok':
      return print(n.a, ctx);
    default:
      throw new Error(`cannot print ${n.t}`);
  }
}

/** Greedy line wrapping at spaces that are not inside brackets, quotes or slashes. */
function wrapText(prefix, text, indent) {
  if (prefix.length + text.length <= WIDTH) return prefix + text;
  const words = [];
  let depth = 0;
  let inQuote = null;
  let inRegex = false;
  let current = '';
  for (const ch of text) {
    if (inQuote) {
      if (ch === inQuote) inQuote = null;
    } else if (inRegex) {
      if (ch === '/' && !current.endsWith('\\')) inRegex = false;
    } else if (ch === '"' || ch === "'") inQuote = ch;
    else if (ch === '/' && (current === '' || /[\s(|]$/.test(current))) inRegex = true;
    else if (ch === '(') depth++;
    else if (ch === ')') depth--;
    if (ch === ' ' && depth === 0 && !inQuote && !inRegex) {
      words.push(current);
      current = '';
    } else current += ch;
  }
  words.push(current);
  const pad = ' '.repeat(indent);
  const lines = [];
  let line = prefix;
  let empty = true;
  for (const word of words) {
    if (!empty && line.length + 1 + word.length > WIDTH) {
      lines.push(line);
      line = pad + word;
    } else {
      line += (empty ? '' : ' ') + word;
    }
    empty = false;
  }
  lines.push(line);
  return lines.join('\n');
}

function production(name, body) {
  const head = `${shown(name)} ::= `;
  const pad = ' '.repeat(head.length - 2);
  if (body.t !== 'choice') return wrapText(head, print(body, 0), head.length + 2);
  const alternatives = body.a.map((alt) => print(alt, 1));
  const oneLine = head + alternatives.join(' | ');
  if (oneLine.length <= WIDTH) return oneLine;
  return alternatives
    .map((alt, i) => wrapText(i === 0 ? head : `${pad}| `, alt, head.length + 2))
    .join('\n');
}

const contains = (n, type) => n.t === type || (n.a !== undefined && (Array.isArray(n.a) ? n.a.some((c) => contains(c, type)) : contains(n.a, type)));

// ---- Precedence, from the prec() calls and the precedences list ------------------

function operatorsIn(n) {
  if (n.t === 'field' && n.name === 'operator') return strings(n.a);
  if (n.a === undefined) return [];
  return (Array.isArray(n.a) ? n.a : [n.a]).flatMap(operatorsIn);
}

function strings(n) {
  if (n.t === 'str') return [n.v];
  if (n.a === undefined) return [];
  return (Array.isArray(n.a) ? n.a : [n.a]).flatMap(strings);
}

function collectPrec(n, rule, out) {
  if (n.t === 'prec' && typeof n.level === 'string') {
    const named = operatorsIn(n.a);
    // A form with no operator field (a call, an index, unary minus) is shown by its terminals.
    out.push({ level: n.level, assoc: n.assoc, rule: shown(rule), operators: named.length ? named : strings(n.a) });
  }
  if (n.a !== undefined) (Array.isArray(n.a) ? n.a : [n.a]).forEach((c) => collectPrec(c, rule, out));
}

function precedenceTable(g) {
  const uses = [];
  for (const [name, node] of g.rules) collectPrec(node, name, uses);
  // In tree-sitter a name listed earlier binds tighter.
  const order = g.precedences.flat();
  const levels = [...order, ...uses.map((u) => u.level).filter((l) => !order.includes(l))];
  const rows = ['| Level (tightest first) | Associativity | Rule | Operators |', '|---|---|---|---|'];
  for (const level of levels) {
    for (const use of uses.filter((u) => u.level === level)) {
      const label = (o) => (o.startsWith('@kw:') ? `<${o.slice(4)}>` : o);
      const ops = use.operators.length ? use.operators.map((o) => `\`${label(o)}\``).join(' ') : '(a call of an expression)';
      rows.push(`| ${level} | ${use.assoc ?? '-'} | \`${use.rule}\` | ${ops} |`);
    }
  }
  return rows.join('\n');
}

// ---- Assemble --------------------------------------------------------------------

function generate() {
  const g = loadGrammar();
  const names = new Map();
  for (const [name] of g.rules) {
    const key = shown(name);
    if (names.has(key)) throw new Error(`rules ${names.get(key)} and ${name} would both print as ${key}`);
    names.set(key, name);
  }

  const syntax = [];
  const lexical = [];
  for (const [name, node] of g.rules) {
    const simple = simplify(node);
    (contains(simple, 'tok') ? lexical : syntax).push(production(name, simple));
  }

  const productions = [
    '(* Syntax. A rule that reads a token and then another is separated by any amount of',
    '   whitespace and comments. <Name> is a keyword; see KEYWORDS.md for its spellings. *)',
    '',
    ...syntax,
    '',
    '(* Lexical. Inside these there is no whitespace unless written. A name is stored exactly',
    '   as typed, so a spelling in Tamil and one in Latin letters are different names. *)',
    '',
    ...lexical,
  ].join('\n');

  const conflicts = g.conflicts.map((pair) => `${pair.map(shown).join(' / ')}`);

  const header = [
    '(* The eTamil grammar. Generated by scripts/generate_ebnf.mjs from',
    '   tree-sitter-etamil/grammar.js; do not edit by hand.',
    '',
    '   Notation: ::= defines; | chooses; ( ) groups; ? optional; * zero or more; + one or more;',
    '   "x" is the literal text x; /re/ is a regular expression; <Name> is a keyword token. *)',
    '',
  ].join('\n');

  return {
    ebnf: `${header}\n${productions}\n`,
    productions,
    precedence: precedenceTable(g),
    conflicts,
    ruleCount: g.rules.length,
  };
}

function block(text, name, body) {
  const begin = `<!-- BEGIN GENERATED: ${name} -->`;
  const end = `<!-- END GENERATED: ${name} -->`;
  const i = text.indexOf(begin);
  const j = text.indexOf(end);
  if (i < 0 || j < i) throw new Error(`${SPEC} has no ${begin} ... ${end} block`);
  return text.slice(0, i + begin.length) + '\n' + body + '\n' + text.slice(j);
}

const out = generate();
const nl = (s) => s.replace(/\r\n/g, '\n');
const files = [[EBNF, out.ebnf]];

if (existsSync(SPEC)) {
  let spec = nl(readFileSync(SPEC, 'utf8'));
  spec = block(spec, 'grammar', '```ebnf\n' + out.productions + '\n```');
  spec = block(spec, 'precedence', out.precedence);
  spec = block(spec, 'ambiguities', out.conflicts.map((c) => `- \`${c}\``).join('\n'));
  files.push([SPEC, spec]);
}

if (process.argv.includes('--check')) {
  const stale = files.filter(([path, content]) => !existsSync(path) || nl(readFileSync(path, 'utf8')) !== content);
  if (stale.length) {
    console.error('out of date: ' + stale.map(([p]) => p).join(', ') + '\nrun: node scripts/generate_ebnf.mjs');
    process.exit(1);
  }
  console.log(`up to date: ${out.ruleCount} rules`);
} else {
  for (const [path, content] of files) writeFileSync(path, content, 'utf8');
  console.log(`wrote ${files.length} file(s), ${out.ruleCount} rules`);
}
