/**
 * @file eTamil grammar for tree-sitter
 * @author Mohammed Maruff (Esan Maruff) <esan@etamil.in>
 * @license MIT
 *
 * The vocabulary is generated. `keywords.js` comes from the compiler's own
 * token table, so this grammar cannot accept a spelling the compiler rejects
 * or miss one it accepts, and CI fails if the two drift.
 *
 * The *structure* is hand-written, because it is not recoverable from a token
 * list. Two shapes catch people out, and both are deliberate in the language:
 *
 *   - the condition comes **before** the keyword — `(வருவாய் > 800000) எனில் {`
 *     — which is why an editor rule looking for `எனில் (` never fires;
 *   - a name is stored exactly as written, so `{வரி: 1}` and `{vari: 1}` are
 *     different fields. Field names are data, not syntax.
 *
 * `நிலை` and `வடிவம்` are keywords only where they begin a binding or a shape,
 * exactly as in the compiler, because both are also ordinary names elsewhere —
 * `நிலை = …;` is a variable in two examples, and nUlakam has a parameter called
 * வடிவம். So they are identifiers here too, recognised by position, and the
 * highlight queries pick them out by their text.
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

const KEYWORDS = require('./keywords.js');

/** Every spelling of one keyword, as an anonymous choice. */
function kw(name) {
  const forms = KEYWORDS[name];
  if (!forms || forms.length === 0) {
    throw new Error(`no spellings generated for token ${name}`);
  }
  return forms.length === 1 ? forms[0] : choice(...forms);
}

/** A comma-separated list of at least one `rule`. */
function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)));
}

function commaSep(rule) {
  return optional(commaSep1(rule));
}

// Tamil (U+0B80–U+0BFF) alongside ASCII, mirroring the lexer's own identifier
// rule. Anything narrower makes every Tamil name invisible to the parser.
const IDENT = /[a-zA-Z_\u0B80-\u0BFF][a-zA-Z0-9_\u0B80-\u0BFF]*/;

module.exports = grammar({
  name: 'etamil',

  extras: ($) => [/\s/, $.comment],

  word: ($) => $.identifier,

  // A statement beginning with an identifier is ambiguous until the parser
  // has seen what follows the target: `வரிசை[0]` starts an index assignment
  // if a `=` comes next and is an index expression otherwise, and one token of
  // lookahead cannot tell. The compiler's recursive-descent parser decides by
  // peeking; tree-sitter is GLR, so the honest answer is to declare the
  // ambiguity and let it carry both interpretations until the `=` settles it.
  // `assignment` is not listed: a plain `name = value;` is decided by the `=`
  // itself, which is one token of lookahead, so declaring it would be an
  // ambiguity that does not exist.
  //
  // The same is true of the forms that begin with two names — `கடன் க = …;`,
  // `நிலை எண் x = …;`, `வடிவம் கடன் { … }` — and of a name followed by `{`,
  // which is a shaped record `கடன்{…}` in an expression and the start of a
  // block after the collection of a ஒவ்வொரு. The compiler settles that last one
  // by not allowing the literal there, as Rust does; here GLR keeps both and
  // the one that cannot finish is dropped.
  conflicts: ($) => [
    [$._primary, $.index_assignment],
    [$._primary, $.field_assignment],
    [$._primary, $.shape_literal],
    // Inside a shape, `செயல் பெயர்(` is a method and `செயல் பெயர்,` a field
    // that holds a function; the token after the name decides.
    [$.function_definition, $.function_type],
  ],

  // Loosest first, matching the precedence the parser documents:
  // or < and < not < comparison < additive < term < unary < postfix
  precedences: () => [
    [
      'postfix',
      'unary',
      'term',
      'additive',
      'comparison',
      'not',
      'and',
      'or',
    ],
  ],

  rules: {
    source_file: ($) => repeat($._statement),

    // eTamil has line comments only. There is no block comment, which is why
    // a /* */ rule in an editor greys out code that then fails to lex.
    comment: () => token(seq('//', /[^\n]*/)),

    _statement: ($) =>
      choice(
        $.import_statement,
        $.function_definition,
        $.return_statement,
        $.if_statement,
        $.while_statement,
        $.for_each_statement,
        $.print_statement,
        $.input_statement,
        $.route_statement,
        $.response_statement,
        $.json_response_statement,
        $.every_statement,
        $.server_statement,
        $.file_statement,
        $.csv_statement,
        $.database_statement,
        $.shape_definition,
        $.declaration,
        $.typed_declaration,
        $.fixed_declaration,
        $.function_declaration,
        $.assignment,
        $.index_assignment,
        $.field_assignment,
        $.expression_statement,
      ),

    // இறக்கு "nUlakam/paNam/paNam.qmz";
    import_statement: ($) => seq(kw('Import'), field('path', $.string), ';'),

    // செயல் பெயர்(அளவுரு) { … }, with optional declared parameter and
    // return types: செயல் வரி(எண் தொகை) எண் { … }
    function_definition: ($) =>
      seq(
        kw('Function'),
        field('name', $.identifier),
        field('parameters', $.parameter_list),
        field('return_type', optional($._return_type)),
        field('body', $.block),
      ),

    // A type keyword, `செயல்` for a function, or a shape's name.
    _return_type: ($) => choice($.type, $.function_type, $.identifier),

    function_type: () => kw('Function'),

    parameter_list: ($) => seq('(', commaSep($.parameter), ')'),

    // `x`, `எண் x`, `செயல் மாற்று`, `கடன் க` (a shape), `நிலை x`, and
    // `நிலை எண் x`. Two names in a row are a shape and a name, or நிலை and a
    // name; which one is the compiler's business and the reader's, not the
    // parser's.
    parameter: ($) =>
      choice(
        field('name', $.identifier),
        seq(field('type', choice($.type, $.function_type)), field('name', $.identifier)),
        seq(field('type', $.identifier), field('name', $.identifier)),
        seq(
          field('modifier', $.identifier),
          field('type', choice($.type, $.function_type, $.identifier)),
          field('name', $.identifier),
        ),
      ),

    // வடிவம் கடன் { எண் அசல், எண் வீதம், செயல் வட்டி(இது) எண் { … } }
    shape_definition: ($) =>
      seq(
        field('keyword', $.identifier),
        field('name', $.identifier),
        '{',
        repeat(choice(seq($.shape_field, optional(',')), $.function_definition)),
        '}',
      ),

    // Two names are always a type and a name: fields are separated by commas,
    // so `எண் அசல்` cannot be two fields.
    shape_field: ($) =>
      prec.right(
        seq(
          field('type', optional(choice($.type, $.function_type, $.identifier))),
          field('name', $.identifier),
        ),
      ),

    return_statement: ($) =>
      seq(kw('Return'), field('value', optional($._expression)), ';'),

    // The condition precedes the keyword. This is the shape most editor
    // integrations get backwards.
    if_statement: ($) =>
      seq(
        '(',
        field('condition', $._expression),
        ')',
        kw('If'),
        field('consequence', $.block),
        optional(seq(kw('Else'), field('alternative', $.block))),
      ),

    while_statement: ($) =>
      seq(
        '(',
        field('condition', $._expression),
        ')',
        kw('Loop'),
        field('body', $.block),
      ),

    // ஒவ்வொரு உறுப்பு இல் வரிசை { … }
    for_each_statement: ($) =>
      seq(
        kw('ForEach'),
        field('variable', $.identifier),
        kw('In'),
        field('collection', $._expression),
        field('body', $.block),
      ),

    print_statement: ($) => seq(kw('Print'), field('value', $._expression), ';'),

    input_statement: ($) => seq(kw('Input'), field('target', $._expression), ';'),

    // வழி பெறு, "/kaNakku/:id" { … }
    route_statement: ($) =>
      seq(
        kw('Route'),
        field('method', $.http_method),
        ',',
        field('path', $.string),
        field('handler', $.block),
      ),

    http_method: () =>
      choice(
        kw('HttpGet'),
        kw('HttpPost'),
        kw('HttpPut'),
        kw('HttpDelete'),
        kw('HttpPatch'),
        kw('HttpOptions'),
        kw('HttpHead'),
      ),

    // பதில் 200, உடல், {"Content-Type": "text/html"};
    response_statement: ($) =>
      seq(
        kw('Response'),
        field('status', $._expression),
        ',',
        field('body', $._expression),
        optional(seq(',', field('headers', $._expression))),
        ';',
      ),

    // ஜேசான்_உரை "{...}", 200;
    json_response_statement: ($) =>
      seq(
        kw('JSONBody'),
        field('body', $._expression),
        optional(seq(',', field('status', $._expression))),
        ';',
      ),

    // இடைவெளி 600 { … } — run this block every N seconds. A background task
    // rather than a loop: it takes a block and no condition.
    every_statement: ($) =>
      seq(
        kw('Every'),
        field('interval', $._expression),
        field('body', $.block),
      ),

    server_statement: ($) =>
      choice(
        seq(
          kw('StartServer'),
          field('host', $._expression),
          ',',
          field('port', $._expression),
          ';',
        ),
        seq(kw('StopServer'), ';'),
      ),

    // கோப்பு_திற "f.txt", "write";  — the mode must be a literal.
    file_statement: ($) =>
      choice(
        seq(
          kw('FileOpen'),
          field('path', $._expression),
          optional(seq(',', field('mode', $._expression))),
          ';',
        ),
        seq(kw('FileClose'), field('path', $._expression), ';'),
        seq(
          kw('FileRead'),
          field('path', $._expression),
          ',',
          field('target', $.identifier),
          ';',
        ),
        seq(
          kw('FileWrite'),
          field('path', $._expression),
          ',',
          field('data', $._expression),
          ';',
        ),
      ),

    csv_statement: ($) =>
      choice(
        seq(
          kw('ReadCSV'),
          field('path', $._expression),
          ',',
          field('target', $.identifier),
          ';',
        ),
        seq(
          kw('WriteCSV'),
          field('path', $._expression),
          ',',
          field('row', $._expression),
          ';',
        ),
      ),

    // The database backend is the host's name for a driver, not a name the
    // author invented, so it keeps its canonical spelling.
    database_backend: () =>
      choice(
        kw('SQLite'),
        kw('PostgreSQL'),
        kw('MySQL'),
        kw('MongoDB'),
        kw('Redis'),
        kw('JSONdb'),
        kw('SQL'),
        kw('NoSQL'),
      ),

    // Queries are always parameterised: there is deliberately no way to
    // splice a value into SQL text from eTamil.
    database_statement: ($) =>
      choice(
        seq(
          kw('DBConnect'),
          field('backend', $.database_backend),
          ',',
          field('connection', $._expression),
          ';',
        ),
        seq(kw('DBDisconnect'), field('backend', $.database_backend), ';'),
        seq(
          kw('DBQuery'),
          field('sql', $._expression),
          ',',
          field('parameters', $._expression),
          ',',
          field('target', $.identifier),
          ';',
        ),
        seq(
          kw('DBExecute'),
          field('sql', $._expression),
          ',',
          field('parameters', $._expression),
          ';',
        ),
      ),

    // எண் வருவாய் = 100000;  — and a bare `எண் வருவாய்;`
    declaration: ($) =>
      seq(
        field('type', $.type),
        field('name', $.identifier),
        optional(seq('=', field('value', $._expression))),
        ';',
      ),

    // கடன் க = கடன்{…};  — a shape's name before a name — and நிலை x = 5;,
    // which has the same form.
    typed_declaration: ($) =>
      seq(
        field('type', $.identifier),
        field('name', $.identifier),
        optional(seq('=', field('value', $._expression))),
        ';',
      ),

    // நிலை எண் வரம்பு_ = 250000;  நிலை கடன் க = …;  நிலை செயல் f = …;
    fixed_declaration: ($) =>
      seq(
        field('keyword', $.identifier),
        field('type', choice($.type, $.function_type, $.identifier)),
        field('name', $.identifier),
        '=',
        field('value', $._expression),
        ';',
      ),

    // செயல் மாற்று = இரட்டி;  — a variable declared to hold a function.
    function_declaration: ($) =>
      seq(kw('Function'), field('name', $.identifier), '=', field('value', $._expression), ';'),

    assignment: ($) =>
      seq(field('name', $.identifier), '=', field('value', $._expression), ';'),

    index_assignment: ($) =>
      seq(
        field('name', $.identifier),
        '[',
        field('index', $._expression),
        ']',
        '=',
        field('value', $._expression),
        ';',
      ),

    field_assignment: ($) =>
      seq(
        field('name', $.identifier),
        '.',
        field('field', $.identifier),
        '=',
        field('value', $._expression),
        ';',
      ),

    expression_statement: ($) => seq($._expression, ';'),

    block: ($) => seq('{', repeat($._statement), '}'),

    type: () =>
      choice(
        kw('IntegerType'),
        kw('FloatType'),
        kw('StringType'),
        kw('BoolType'),
        kw('TextType'),
        kw('ArrayType'),
        kw('DataType'),
        kw('ObjectType'),
        kw('DateType'),
      ),

    // --- Expressions ---------------------------------------------------

    _expression: ($) =>
      choice(
        $.logical_expression,
        $.unary_not,
        $.comparison_expression,
        $.binary_expression,
        $.unary_minus,
        $._postfix,
      ),

    logical_expression: ($) =>
      choice(
        prec.left('or', seq($._expression, kw('Or'), $._expression)),
        prec.left('and', seq($._expression, kw('And'), $._expression)),
      ),

    unary_not: ($) => prec.right('not', seq(kw('Not'), $._expression)),

    comparison_expression: ($) =>
      prec.left(
        'comparison',
        seq(
          field('left', $._expression),
          field('operator', choice('==', '!=', '<=', '>=', '<', '>')),
          field('right', $._expression),
        ),
      ),

    binary_expression: ($) =>
      choice(
        prec.left(
          'additive',
          seq(
            field('left', $._expression),
            // `&` concatenates; it is additive-level in this language.
            field('operator', choice('+', '-', '&')),
            field('right', $._expression),
          ),
        ),
        prec.left(
          'term',
          seq(
            field('left', $._expression),
            field('operator', choice('*', '/')),
            field('right', $._expression),
          ),
        ),
      ),

    unary_minus: ($) => prec.right('unary', seq('-', $._expression)),

    _postfix: ($) =>
      choice(
        $.call,
        $.index,
        $.field_access,
        $.try_expression,
        $._primary,
      ),

    // `f(x)`, and a call of anything that gives a function: `f(1)(2)`,
    // `விதிகள்[0](x)`, and `க.வட்டி(2)` — a method, or a function a record
    // holds in that field.
    call: ($) =>
      prec(
        'postfix',
        seq(field('function', $._postfix), field('arguments', $.argument_list)),
      ),

    argument_list: ($) => seq('(', commaSep($._expression), ')'),

    index: ($) =>
      prec(
        'postfix',
        seq(field('base', $._postfix), '[', field('index', $._expression), ']'),
      ),

    field_access: ($) =>
      prec('postfix', seq(field('base', $._postfix), '.', field('field', $.identifier))),

    // `?` — unwrap a சரி, or hand the தவறு back to the caller.
    try_expression: ($) => prec('postfix', seq($._postfix, '?')),

    _primary: ($) =>
      choice(
        $.parenthesized_expression,
        $.lambda,
        $.shape_literal,
        $.array,
        $.record,
        $.string,
        $.percentage,
        $.number,
        $.boolean,
        $.null,
        $.identifier,
      ),

    parenthesized_expression: ($) => seq('(', $._expression, ')'),

    array: ($) => seq('[', commaSep($._expression), ']'),

    record: ($) => seq('{', commaSep($.pair), '}'),

    // செயல்(x) { திரும்பு x * 2; } — a function written where a value goes.
    lambda: ($) =>
      seq(
        kw('Function'),
        field('parameters', $.parameter_list),
        field('return_type', optional($._return_type)),
        field('body', $.block),
      ),

    // கடன்{அசல்: 100000, ..பழையது} — a record made as a declared shape.
    shape_literal: ($) =>
      seq(
        field('shape', $.identifier),
        '{',
        optional(
          choice(
            seq(commaSep1($.pair), optional(seq(',', $.spread))),
            $.spread,
          ),
        ),
        optional(','),
        '}',
      ),

    spread: ($) => seq('..', field('base', $._expression)),

    // A field name is data — what the author typed — so a quoted key and a
    // bare one mean the same thing and neither is a keyword.
    pair: ($) =>
      seq(field('key', choice($.identifier, $.string)), ':', field('value', $._expression)),

    // Every number is a fixed-point decimal, and a percentage literal is
    // exact: 18% is 0.18, never a rounded double.
    percentage: () => token(seq(/[0-9]+/, optional(seq('.', /[0-9]+/)), '%')),

    number: () => token(seq(/[0-9]+/, optional(seq('.', /[0-9]+/)))),

    // `token.immediate` on the contents is load-bearing. `extras` includes
    // whitespace, and tree-sitter inserts extras *between* the elements of a
    // seq — so with an ordinary regex here the newlines inside a multi-line
    // string were being skipped as whitespace rather than kept as text, and
    // every multi-line SQL literal in nUlakam failed to parse.
    string: ($) =>
      seq(
        '"',
        repeat(
          choice($.escape_sequence, $.invalid_escape, token.immediate(/[^"\\]+/)),
        ),
        token.immediate('"'),
      ),

    // The five the compiler decodes.
    escape_sequence: () => token.immediate(/\\[ntr"\\]/),

    // Anything else keeps both characters rather than being an escape, which
    // is worth showing as different from the ones that are.
    invalid_escape: () => token.immediate(/\\[^ntr"\\]/),

    boolean: () => choice(kw('True'), kw('False')),

    null: () => kw('Null'),

    identifier: () => token(IDENT),
  },
});
