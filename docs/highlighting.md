# Highlighting

`tree-sitter-nuke/` is the third grammar of Nuke here, and the first written for text that is
not finished. `grammar/` is normative and `crates/nuke-grammar` runs it; `crates/nuke-syntax` is
the implementation both are held to. Neither helps an editor, which needs a tree for every
keystroke of a document halfway through being typed — and Helix takes highlighting from
tree-sitter and from nowhere else, so no LSP server we write later can supply it.

It reads the **surface** language, which contains the canonical form, so a `.nuke` file is one
kind of file and an editor is never told which one it is holding.

## What a third grammar may disagree about

`tree-sitter-nuke/test/verdicts` names every fixture this grammar does not simply accept, and
the list is exhaustive: a fixture missing from it must parse clean, so a divergence that appears
fails the suite rather than settling in. That is the arrangement
`crates/nuke-syntax/tests/conformance.rs` already uses against the ABNF, moved to the tool that
can run it.

Seven fixtures the parser refuses are absent on purpose. Five are the boundary the surface
language already crossed — `bare-ident`, `braceless-fields`, `hex-literal`, `ident-as-map-key` and
`interpolated-string` are refused by the canonical form and admitted by this one. One is
`a-specifier-that-is-not-one`, `{1:007}`, because a specifier is a raw tail to the closing brace
here and `Spec::parse` is what reads it. The last is `byte-order-mark`, and it is the only one
that is a limit of the tool: `tree-sitter parse` strips a BOM before the grammar is given the
text, so there is no byte here to refuse.

Everything else a semantic pass owns — a duplicate field, a lone surrogate, a cycle between
files, a hole holding a float — is a `fixtures/surface/refused` document, and those all parse.
That is the point rather than a gap: the linter and the LSP server report them, and they will
read this tree to do it.

## The refinement falls out of longest match

`grammar/tokens.abnf` splits a number in two, a permissive token and a rule refining it, so that
a misspelling is one bad token that names itself rather than two good ones — `0xff` must not
read as `0` beside `xff`. ABNF cannot say that in one rule, and `Grammar::parse` checks it in a
second pass over the tree.

A tree-sitter lexer needs no second pass. Both spellings are tokens — `number` matching
`surface-number` exactly, `malformed_number` matching the permissive run — and longest match
picks between them, with rule order settling a tie in favour of the good one. `0xFF` and `0e5`
tie and are numbers; `0xff`, `01`, `1E5`, `1e05`, `1.` and `0xFF.` are longer as the permissive
token and are named. `0d5` stays a number beside a name, because there is no decimal marker.

So the fault has a node rather than an `ERROR`, which is what lets `highlights.scm` paint it and
what keeps the tree around it intact. It is also the one thing here that reads the language more
precisely than the ABNF alone does.

## `{}` is a tuple

`tuple` is `{ binding* field* }` and `map` is `{ binding* entry+ }`, so the empty block belongs
to the first. The ABNF gives it to both and says nothing turns on which a parser records;
`surface.rs::block` had already made this call, and a grammar that left it open would need a
conflict declared for a distinction no one can observe.

## The mode stack is the parse stack

`lexer.rs` carries a stack of `Mode::{Text, Hole}` because one lexer runs the length of a file
and has to remember where it is. A tree-sitter parse state already is that stack, so there is no
external scanner: `$"` opens, and every piece inside the quotes — the text run, an escape, `{{`,
the closing `"` — is a `token.immediate`, which is the whole of what `Mode::Text` said. A hole's
interior is ordinary and takes whitespace and comments for free, which is what `ows` means
there. Longest match then gives the two rules the language already fixed: `{{` beats a hole, and
a lone `}` matches nothing valid and is a fault.

## One token precedence, and where it is spent

Whitespace and comments are *extras*, which means tree-sitter will match them anywhere,
including inside a string — and `#` starts a comment, so `$"#{accent}"` lexed as `$"` beside a
comment running to the end of the line. A colour is the first thing anyone writes in a dot file,
so this is not a corner.

Token precedence beats longest match, and the three runs of raw text — a string's content, an
interpolation's, and a specifier — carry it. Nothing else does, and in particular `number` does
not: a precedence there would beat the longest match the refinement above depends on, and `[01]`
would quietly split into `0` and `1`, which is the reading `grammar/tokens.abnf` was written to
prevent.

## The parser is committed, and cannot drift

Helix and nvim compile `src/parser.c`; neither runs `tree-sitter generate`. A grammar left
ungenerated is one no editor can load, so `src/` is checked in and marked `linguist-generated`.
What holds it to `grammar.js` is the last step of `conformance.sh`, which generates and then
refuses a `src/` that is not what the repository already has.

## The queries

`highlights.scm`, `locals.scm`, `indents.scm` and `textobjects.scm`, in Helix's capture names.
`True`, `False` and `Null` get no highlight of their own, because they are ordinary atoms and a
colour saying otherwise would be this file deciding what an atom means.

`locals.scm` is the one an editor could not have had without a grammar: a binding is visible
below itself and inside the blocks nested there, so a scope is a `document`, a `tuple` or a
`map`, and a name can be followed to where it was bound without evaluating anything. There is no
`injections.scm` — nothing inside a Nuke file is another language.
