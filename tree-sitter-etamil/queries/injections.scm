; Injections: other languages inside eTamil.

; The SQL given to a database statement. A multi-line string is a single `string`
; node; the offset drops the opening and closing quote from the injected range.
((database_statement
   sql: (string) @injection.content)
 (#set! injection.language "sql")
 (#offset! @injection.content 0 1 0 -1))

; Comments often carry TODO and similar markers.
((comment) @injection.content
 (#set! injection.language "comment"))
