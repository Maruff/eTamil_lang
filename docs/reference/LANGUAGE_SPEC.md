# The eTamil language: lexical structure and grammar

This is the published specification of what an eTamil program looks like: the
characters it is made of, its words, and the shapes they combine into. It does not
say what a program *does*; see [What this does not cover](#what-this-does-not-cover).

The machine-readable grammar is [`etamil.ebnf`](etamil.ebnf). The keyword list, with
every spelling of every keyword, is [`KEYWORDS.md`](KEYWORDS.md). Both are generated,
so they cannot disagree with this page or with the compiler's own vocabulary.

## How this was derived, and how far to trust it

The grammar below is not a second description kept beside the compiler. It is the
tree-sitter grammar (`tree-sitter-etamil/grammar.js`) printed as EBNF by
`scripts/generate_ebnf.mjs`, and that grammar is held to the language two ways:

- its keywords are generated from the compiler's lexer (`etamil_compiler/src/lexer.rs`),
  so it accepts exactly the spellings the compiler does, and CI fails if the two drift;
- `npm run parse:corpus` parses every program in `nUlakam/` and `examples/` with it.
  At the time of writing that is **1,133 files, all parsed with no errors**.

What that does **not** show: that the grammar *rejects* everything the compiler
rejects. A program can be accepted here and still be refused by the compiler, which
also checks types, names and statement placement. Treat this grammar as a description
of what can be written, and `etamil --check` as the judge of what is valid.

## Lexical structure

**Source text** is Unicode, written as UTF-8. Whitespace separates tokens and is
otherwise ignored.

**Comments** are line comments only: `//` to the end of the line. There is no block
comment, so `/* ... */` is not a comment and will not lex.

**Names** (identifiers) start with an ASCII letter, `_`, or a character in the Tamil
block (U+0B80 to U+0BFF), and continue with those or ASCII digits. A name is stored
**exactly as written**: `வரி` and `vari` are different names, and so are the fields
`{வரி: 1}` and `{vari: 1}`. Pick one spelling per program.

**Keywords** each have a Tamil spelling and a Latin-letter spelling (and sometimes
more), all interchangeable; the full list is [`KEYWORDS.md`](KEYWORDS.md). The type
keywords and the SQL clause keywords are reserved. Two words, `நிலை` and `வடிவம்`, are
keywords only where they begin a binding or a shape and are ordinary names everywhere
else, so `நிலை = 5;` is an assignment to a variable called `நிலை`.

**Numbers** are decimal: digits, optionally `.` and more digits. Every number is a
fixed-point decimal, not a floating-point value.

**Percentages** are a number followed directly by `%`, and are exact: `18%` is
`0.18`, never a rounded value.

**Strings** are written between double quotes and may span several lines. Five
escapes are decoded: `\n`, `\t`, `\r`, `\"` and `\\`. A backslash followed by anything
else is not an escape; both characters are kept.

**Punctuation and operators** are the quoted terminals in the grammar below: the
brackets `( ) [ ] { }`, the separators `, ; : .`, the spread `..`, the postfix `?`,
the assignment `=`, and the operators in the precedence table.

## Grammar

Notation: `::=` defines; `|` chooses; `( )` groups; `?` optional; `*` zero or more;
`+` one or more; `"x"` is the literal text x; `/re/` is a regular expression; `<Name>`
is a keyword token, named as in the Token column of [`KEYWORDS.md`](KEYWORDS.md).

<!-- BEGIN GENERATED: grammar -->
```ebnf
(* Syntax. A rule that reads a token and then another is separated by any amount of
   whitespace and comments. <Name> is a keyword; see KEYWORDS.md for its spellings. *)

source_file ::= statement*
statement ::= import_statement
            | function_definition
            | return_statement
            | if_statement
            | while_statement
            | for_each_statement
            | print_statement
            | input_statement
            | route_statement
            | response_statement
            | json_response_statement
            | every_statement
            | server_statement
            | file_statement
            | csv_statement
            | database_statement
            | shape_definition
            | declaration
            | typed_declaration
            | fixed_declaration
            | function_declaration
            | assignment
            | index_assignment
            | field_assignment
            | expression_statement
import_statement ::= <Import> string ";"
function_definition ::= <Function> identifier parameter_list return_type? block
return_type ::= type | function_type | identifier
function_type ::= <Function>
parameter_list ::= "(" (parameter ("," parameter)*)? ")"
parameter ::= identifier
            | (type | function_type) identifier
            | identifier identifier
            | identifier (type | function_type | identifier) identifier
shape_definition ::= identifier identifier "{" (shape_field ","? | function_definition)* "}"
shape_field ::= (type | function_type | identifier)? identifier
return_statement ::= <Return> expression? ";"
if_statement ::= "(" expression ")" <If> block (<Else> block)?
while_statement ::= "(" expression ")" <Loop> block
for_each_statement ::= <ForEach> identifier <In> expression block
print_statement ::= <Print> expression ";"
input_statement ::= <Input> expression ";"
route_statement ::= <Route> http_method "," string block
http_method ::= <HttpGet>
              | <HttpPost>
              | <HttpPut>
              | <HttpDelete>
              | <HttpPatch>
              | <HttpOptions>
              | <HttpHead>
response_statement ::= <Response> expression "," expression ("," expression)? ";"
json_response_statement ::= <JSONBody> expression ("," expression)? ";"
every_statement ::= <Every> expression block
server_statement ::= <StartServer> expression "," expression ";" | <StopServer> ";"
file_statement ::= <FileOpen> expression ("," expression)? ";"
                 | <FileClose> expression ";"
                 | <FileRead> expression "," identifier ";"
                 | <FileWrite> expression "," expression ";"
csv_statement ::= <ReadCSV> expression "," identifier ";"
                | <WriteCSV> expression "," expression ";"
database_backend ::= <SQLite>
                   | <PostgreSQL>
                   | <MySQL>
                   | <MongoDB>
                   | <Redis>
                   | <JSONdb>
                   | <SQL>
                   | <NoSQL>
database_statement ::= <DBConnect> database_backend "," expression ";"
                     | <DBDisconnect> database_backend ";"
                     | <DBQuery> expression "," expression "," identifier ";"
                     | <DBExecute> expression "," expression ";"
declaration ::= type identifier ("=" expression)? ";"
typed_declaration ::= identifier identifier ("=" expression)? ";"
fixed_declaration ::= identifier (type | function_type | identifier) identifier "=" expression
                        ";"
function_declaration ::= <Function> identifier "=" expression ";"
assignment ::= identifier "=" expression ";"
index_assignment ::= identifier "[" expression "]" "=" expression ";"
field_assignment ::= identifier "." identifier "=" expression ";"
expression_statement ::= expression ";"
block ::= "{" statement* "}"
type ::= <IntegerType>
       | <FloatType>
       | <StringType>
       | <BoolType>
       | <TextType>
       | <ArrayType>
       | <DataType>
       | <ObjectType>
       | <DateType>
expression ::= logical_expression
             | unary_not
             | comparison_expression
             | binary_expression
             | unary_minus
             | postfix
logical_expression ::= expression <Or> expression | expression <And> expression
unary_not ::= <Not> expression
comparison_expression ::= expression ("==" | "!=" | "<=" | ">=" | "<" | ">") expression
binary_expression ::= expression ("+" | "-" | "&") expression
                    | expression ("*" | "/") expression
unary_minus ::= "-" expression
postfix ::= call | index | field_access | try_expression | primary
call ::= postfix argument_list
argument_list ::= "(" (expression ("," expression)*)? ")"
index ::= postfix "[" expression "]"
field_access ::= postfix "." identifier
try_expression ::= postfix "?"
primary ::= parenthesized_expression
          | lambda
          | shape_literal
          | array
          | record
          | string
          | percentage
          | number
          | boolean
          | null
          | identifier
parenthesized_expression ::= "(" expression ")"
array ::= "[" (expression ("," expression)*)? "]"
record ::= "{" (pair ("," pair)*)? "}"
lambda ::= <Function> parameter_list return_type? block
shape_literal ::= identifier "{" (pair ("," pair)* ("," spread)? | spread)? ","? "}"
spread ::= ".." expression
pair ::= (identifier | string) ":" expression
boolean ::= <True> | <False>
null ::= <Null>

(* Lexical. Inside these there is no whitespace unless written. A name is stored exactly
   as typed, so a spelling in Tamil and one in Latin letters are different names. *)

comment ::= "//" /[^\n]*/
percentage ::= /[0-9]+/ ("." /[0-9]+/)? "%"
number ::= /[0-9]+/ ("." /[0-9]+/)?
string ::= '"' (escape_sequence | invalid_escape | /[^"\\]+/)* '"'
escape_sequence ::= /\\[ntr"\\]/
invalid_escape ::= /\\[^ntr"\\]/
identifier ::= /[a-zA-Z_\u0B80-\u0BFF][a-zA-Z0-9_\u0B80-\u0BFF]*/
```
<!-- END GENERATED: grammar -->

## Operator precedence

An expression is built from the rules above, and the table says how they nest when
no brackets are written. Higher in the table binds tighter. Operators of the same
level and associativity group as shown: `a - b - c` is `(a - b) - c`.

<!-- BEGIN GENERATED: precedence -->
| Level (tightest first) | Associativity | Rule | Operators |
|---|---|---|---|
| postfix | - | `call` | (a call of an expression) |
| postfix | - | `index` | `[` `]` |
| postfix | - | `field_access` | `.` |
| postfix | - | `try_expression` | `?` |
| unary | right | `unary_minus` | `-` |
| term | left | `binary_expression` | `*` `/` |
| additive | left | `binary_expression` | `+` `-` `&` |
| comparison | left | `comparison_expression` | `==` `!=` `<=` `>=` `<` `>` |
| not | right | `unary_not` | `<Not>` |
| and | left | `logical_expression` | `<And>` |
| or | left | `logical_expression` | `<Or>` |
<!-- END GENERATED: precedence -->

Note that `&` joins strings and is at the same level as `+` and `-`.

## Shapes of the language that surprise people

- **The condition comes before the keyword.** An `if` is `(condition) எனில் { ... }`,
  and a `while` is `(condition) சுற்று { ... }`: the parenthesised condition is
  written first in both. An editor rule that looks for the keyword followed by `(` will never
  match. (In the grammar these are `if_statement` and `while_statement`.)
- **A statement can begin with two names.** `கடன் க = ...;` (a value of a declared
  shape) and `நிலை எண் x = ...;` both start with a name followed by a name, and a
  shape definition `வடிவம் கடன் { ... }` does too. Which reading applies is decided by
  what follows, not by a keyword.
- **A name followed by `{` is a record of a declared shape**, `கடன்{அசல்: 100000}`,
  except after the collection of a for-each, where the `{` begins the loop body.
- **A field name is data.** In `{வரி: 1}` and `{"வரி": 1}` the key means the same
  thing, and neither is a keyword.

The parser has to look ahead to settle these. The grammar declares them as ambiguities
it keeps open until the next token decides:

<!-- BEGIN GENERATED: ambiguities -->
- `primary / index_assignment`
- `primary / field_assignment`
- `primary / shape_literal`
- `function_definition / function_type`
<!-- END GENERATED: ambiguities -->

## What this does not cover

- **Meaning.** What a statement does when it runs, how decimals round, what a
  `சரி`/`தவறு` result is: see the language reference and the compiler's behaviour.
- **Types and names.** Whether a name is defined, or an argument has the right type,
  is checked by the compiler, not described by this grammar.
- **The standard library** (`nUlakam/`), the HTTP server and database statements'
  runtime behaviour, and the VM.
- **Grammar for a future version.** This describes the language as the compiler
  accepts it today.

## Keeping this current

`node scripts/generate_ebnf.mjs` rewrites `etamil.ebnf` and the generated blocks of
this page from `tree-sitter-etamil/grammar.js`; `node scripts/generate_ebnf.mjs --check`
fails if they are out of date. Change the grammar in `grammar.js` (and the lexer for a
keyword), never the generated parts.
