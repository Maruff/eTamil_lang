; Scope and binding queries for eTamil, used by editors for rename, highlighting of
; references, and "go to definition" within a file.
;
; eTamil has no block-scoped binding form: assignment is a bare `name = value`, so
; a name first written inside an `எனில்` or a loop is visible after it. Only a
; function (or lambda) opens a scope, and a whole file is the outermost one.
;
; Captures are the plain @local.scope, @local.definition and @local.reference,
; because tree-sitter's own tags loader (used by `tree-sitter test` and the CLI)
; rejects a kind suffix such as @local.definition.function that nvim-treesitter allows.

(source_file) @local.scope
(function_definition) @local.scope
(lambda) @local.scope

; A function's name belongs to the scope that encloses it, not to its own body.
(function_definition
  name: (identifier) @local.definition
  (#set! definition.scope "parent"))

(function_declaration
  name: (identifier) @local.definition
  (#set! definition.scope "parent"))

(parameter name: (identifier) @local.definition)

(assignment name: (identifier) @local.definition)
(typed_declaration name: (identifier) @local.definition)
(fixed_declaration name: (identifier) @local.definition)
(for_each_statement variable: (identifier) @local.definition)

; எடு_வினவு names the variable that receives the rows.
(database_statement target: (identifier) @local.definition)

(identifier) @local.reference
