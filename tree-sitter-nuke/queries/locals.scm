; A binding is visible below itself and inside the blocks nested there, which is
; a scope an editor can follow without evaluating anything.

(document) @local.scope
(tuple) @local.scope
(map) @local.scope

(binding name: (ident) @local.definition)

(reference (ident) @local.reference)
