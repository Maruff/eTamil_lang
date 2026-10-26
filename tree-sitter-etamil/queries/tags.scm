; Tags: the definitions and calls an editor lists in its symbol outline and uses for
; "go to symbol" and code search. Capture names follow tree-sitter's tags convention.
;
; nUlakam documents nearly every function with a `//` comment directly above it, so
; that comment is captured as its documentation.

(
  (comment)* @doc
  .
  (function_definition
    name: (identifier) @name) @definition.function
  (#strip! @doc "^//\\s*")
  (#select-adjacent! @doc @definition.function)
)

(function_declaration name: (identifier) @name) @definition.function

; A shape (வடிவம்) names a kind of record, which is the nearest thing eTamil has to a class.
(shape_definition name: (identifier) @name) @definition.class

(call function: (identifier) @name) @reference.call
