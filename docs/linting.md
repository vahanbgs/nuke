# Linting

`nuke_lint::lint` reads a document and reports what is wrong with it that is not wrong with
its grammar, and `nuke lint FILE` is that function on a path or a pipe. It rewrites nothing.

## A lint is what the formatter may not fix

`docs/formatting.md` draws its half of `docs/embedding.md`'s line: reduction decides what a
document *means*, formatting decides only how it *reads*, and the two must not meet. Every
leaf the printer emits is copied from its own span for that reason, so the formatter cannot
respell anything at all.

A name is not whitespace. A field name survives into JSON, TOML and the seven others; an atom
**is** a value. Respelling one is reduction's side of that line, and the formatter is
forbidden to cross it — which leaves the fault standing with nothing able to fix it. That
gap is what a linter is for, and it is the whole definition: **a lint is exactly what the
formatter may not fix**. So it reports rather than rewrites, so there is no `--fix`, and so
there is no configuration either, for the reason `docs/formatting.md` gives against knobs.

The converse bounds it. A lint never affects whether a document parses, and a document that
did not parse has one fault and it is the parser's — so `lint` returns `Err` for a syntax
error rather than folding it in among the findings. An editor wants those two apart.

## Three rules, and each is only the tail of its prose

`docs/canonical-form.md` says atoms are `UpperCamelCase` and identifiers are `snake_case`
with no leading, trailing or doubled `_`. `docs/imports.md` says asking for the `.nuke`
extension is the linter's. The lexer already holds most of that, and what is left is small:

- **`atom-case`** fires on two capitals standing together — `TRUE`, `HTTPServer`. An atom already
  starts `A-Z` and continues alphanumeric, so `Foo_Bar` is a token fault and never arrives.
- **`ident-case`** fires on a trailing `_` or a doubled `__`. An identifier already starts `a-z`,
  so a leading `_` and every capital are token faults; the rule's third clause is paid already.
- **`import-extension`** fires on an `@import` whose path does not end `.nuke`. Nothing precedes
  this one: the whole of the rule is the linter's.

That is the shape to keep. **The linter's share of a rule is precisely the part the grammar
admits**, never a second enforcement of what the lexer already refuses — two checks on one
spelling is two places to disagree about it. `ident-case` therefore runs at every name a
document has: a binding, a field, a projected field, a builtin's name, and a reference.

## It takes no filesystem

`nuke-lint` depends on `nuke-syntax` and never on `nuke-eval`, and that is the crate's
defining constraint rather than an accident of the layering. `docs/embedding.md` said reading
the dependency graph *without* a filesystem is a walk over `ExprKind::Call`, whose operand is
a literal precisely so that walk is possible, and that it waits for the tool that wants it.
This is that tool.

So `nuke lint` reports the file it was given and follows no import. A linter that opened what
a document imports would report a fault in a file its author may not own and cannot fix from
here, and `nuke deps` already answers the question about the graph. It is also what makes the
linter safe to run on every keystroke, which is what the LSP server will do with it.

## What holds it

Every fixture that parses reports nothing — `fixtures/valid` and
`fixtures/surface/{valid,reduced,refused,modules}`, the set `docs/formatting.md` is held to,
filtered the same way because `modules/is-not-a-document.nuke` exists to be unparseable. A
fixture in the wrong style fails the suite rather than setting a second precedent for what
Nuke looks like. It passed unchanged the day it was written, which is the corpus saying the
rules were already the convention rather than a new opinion about it.

Each rule pins its **span** as well as its finding, because a diagnostic on the right
document at the wrong place is one an editor underlines somewhere else, and findings arrive
in source order so that stays a promise rather than an implementation detail.

## What it does not report yet

An **unused binding** is the lint a user would meet first, and the only one that needs a
scope pass rather than a walk — sequential scope with shadowing, which `locals.scm` already
models for the editor. It is the next rule, not a declined one.

`docs/toml.md` asks that a document wanting headers throughout put its tables last, and hands
that to "the formatter — not the transpiler". The formatter never reorders, so it is really a
lint; but it is a lint about a **target**, and the linter reads no target. Whoever writes it
has to say first whether a rule may know where a document is going.
