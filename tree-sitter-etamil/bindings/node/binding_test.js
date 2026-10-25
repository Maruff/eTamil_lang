const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");
const { test } = require("node:test");

const Parser = require("tree-sitter");
const { Query } = Parser;

const root = path.join(__dirname, "..", "..");
const language = require(".");

function parse(source) {
  const parser = new Parser();
  parser.setLanguage(language);
  return parser.parse(source);
}

test("can load grammar", () => {
  const parser = new Parser();
  assert.doesNotThrow(() => parser.setLanguage(language));
});

test("parses a program without errors", () => {
  const tree = parse("ceyal add(a, b) { qirumpu a + b; }\ntotal = add(1, 2);\n");
  assert.strictEqual(tree.rootNode.hasError, false);
});

// Regression: built without /utf-8, MSVC read parser.c in the ANSI code page and
// turned every Tamil node name into '?', which made any query naming a keyword
// node fail to compile. Windows is where this showed; the test is for every OS.
test("Tamil keyword nodes keep their names", () => {
  const tree = parse('ஆம் = மெய்;\n(அ > 1) எனில் { அச்சு 1; }\n');
  assert.strictEqual(tree.rootNode.hasError, false);

  const names = new Set();
  (function walk(node) {
    if (!node.isNamed) names.add(node.type);
    node.children.forEach(walk);
  })(tree.rootNode);

  for (const keyword of ["மெய்", "எனில்", "அச்சு"]) {
    assert.ok(names.has(keyword), `${keyword} is a node type, got: ${[...names].join(" ")}`);
  }
  assert.ok(![...names].some((name) => name.includes("?")), "no node name was turned into '?'");
});

// node-tree-sitter rejects the editor-level predicates #offset!, #strip! and
// #select-adjacent!, so injections.scm and tags.scm are checked by the CLI and the
// Rust binding instead. highlights and locals use only predicates it knows.
for (const name of ["highlights", "locals"]) {
  test(`${name}.scm compiles against the grammar`, () => {
    const source = fs.readFileSync(path.join(root, "queries", `${name}.scm`), "utf8");
    assert.doesNotThrow(() => new Query(language, source));
  });
}
