; Nuke, for Helix. `True`, `False` and `Null` are ordinary atoms with no special
; treatment in the language, so they get none here either.

(comment) @comment

(atom) @constant
(number) @constant.numeric
(malformed_number) @error

(string) @string
(interpolation) @string
(escape_sequence) @constant.character.escape
(doubled_brace) @constant.character.escape
(format_spec) @string.special

(binding name: (ident) @variable)
(reference (ident) @variable)
(field name: (ident) @variable.other.member)
(access key: (ident) @variable.other.member)
(call builtin: (ident) @function.builtin)

[
  ":="
  "="
  "=>"
  "."
  ":"
] @operator

"@" @punctuation.special

(interpolation "$\"" @punctuation.special)
(hole ["{" "}"] @punctuation.special)

(tuple ["{" "}"] @punctuation.bracket)
(map ["{" "}"] @punctuation.bracket)
(list ["[" "]"] @punctuation.bracket)
(computed ["(" ")"] @punctuation.bracket)
