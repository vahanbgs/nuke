# Linting

`nuke_lint::lint` reads a document and reports what is wrong with it that is not wrong with
its grammar, and `nuke lint FILE` is that function on a path or a pipe. It rewrites nothing.

## A lint is what the formatter may not fix

`docs/formatting.md` draws its half of `docs/embedding.md`'s line: reduction decides what a
document *means*, formatting decides only how it *reads*, and the two must not meet. Every
leaf the printer emits is copied from its own span for that reason, so the formatter cannot
respell anything at all.

A name is not whitespace. A field name survives into JSON, TOML and the eight others; an atom
**is** a value. Respelling one is reduction's side of that line, and the formatter is
forbidden to cross it — which leaves the fault standing with nothing able to fix it. That
gap is what a linter is for, and it is the whole definition: **a lint is exactly what the
formatter may not fix**. So it reports rather than rewrites, so there is no `--fix`, and so
there is no configuration either, for the reason `docs/formatting.md` gives against knobs.

The converse bounds it. A lint never affects whether a document parses, and a document that
did not parse has one fault and it is the parser's — so `lint` returns `Err` for a syntax
error rather than folding it in among the findings. An editor wants those two apart.

## Four rules, and three are only the tail of their prose

`docs/canonical-form.md` says atoms are `UpperCamelCase` and identifiers are `snake_case`
with no leading, trailing or doubled `_`. `docs/imports.md` says asking for the `.nuke`
extension is the linter's. The lexer already holds most of that, and what is left is small:

- **`atom-case`** fires on two capitals standing together — `TRUE`, `HTTPServer`. An atom already
  starts `A-Z` and continues alphanumeric, so `Foo_Bar` is a token fault and never arrives.
- **`ident-case`** fires on a trailing `_` or a doubled `__`. An identifier already starts `a-z`,
  so a leading `_` and every capital are token faults; the rule's third clause is paid already.
- **`import-extension`** fires on an `@import` whose path does not end `.nuke`. Nothing precedes
  this one: the whole of the rule is the linter's.
- **`unused-binding`** fires on a name nothing below it reads. Nothing precedes this one either:
  the reducer evaluates every binding and keeps none of them, so it has no complaint to make.

That is the shape to keep. **The linter's share of a rule is precisely the part the grammar
admits**, never a second enforcement of what the lexer already refuses — two checks on one
spelling is two places to disagree about it. `ident-case` therefore runs at every name a
document has: a binding, a field, a projected field, a builtin's name, and a reference.

## `unused-binding` resolves rather than walks

It is the one rule that needs a scope, and the scope is `nuke-eval`'s: bindings stand at the
head of a block and reach to the end of it, a binding's value is read before its own name
exists, and a reference names the last binding of that name. So the pass pushes a binding
*after* walking its value — which is what leaves `n := n` naming the one above — and reports
whatever is unread when the block closes, at the **name**. Rebinding within one block is a
parse error, so the only shadow is a nested one, and a name a nested block covers before
anyone reads it is unread. A reference that resolves to nothing marks nothing: an unbound
name is the reducer's fault and the linter does not say it twice.

That pass is `crates/nuke-resolve` and no longer this crate's, because the server resolves the
same names for go-to-definition and two implementations of one scope would be free to disagree
about which binding a word means. The linter pays a second walk over a tree already in memory
for that, and asks `Resolution` for the names nothing read.

There is no `_name` to opt out of it, because a leading `_` is a token fault and there is no
configuration either. That costs nothing while every binding is written by hand and can simply
be deleted. It starts costing something when a lambda's parameter is forced on you by a
signature — which is what Rust's `_x` is for — and that is when to spell it.

## It takes no filesystem

`nuke-lint` depends on `nuke-syntax` and never on `nuke-eval`, and that is the crate's
defining constraint rather than an accident of the layering. `docs/embedding.md` said reading
the dependency graph *without* a filesystem is a walk over `ExprKind::Call`, whose operand is
a literal precisely so that walk is possible, and that it waits for the tool that wants it.
This is that tool.

So `nuke lint` reports the file it was given and follows no import. A linter that opened what
a document imports would report a fault in a file its author may not own and cannot fix from
here, and `nuke deps` already answers the question about the graph. It is also what makes the
linter safe to run on every keystroke, which is what the server does with it. `docs/lsp.md`
publishes these findings beside the reducer's, having a filesystem of its own, and reports an
imported file's fault at the `@import` that asked for it.

## What holds it

Every fixture that parses reports nothing — `fixtures/valid` and
`fixtures/surface/{valid,reduced,refused,modules}`, the set `docs/formatting.md` is held to,
filtered the same way because `modules/is-not-a-document.nuke` exists to be unparseable. A
fixture in the wrong style fails the suite rather than setting a second precedent for what
Nuke looks like. Three rules passed the corpus unchanged the day they were written; the
fourth cost two `refused/` fixtures a reference each, both of which bound a name only the
refusal itself was interested in. That is the corpus agreeing rather than being made to.

Each rule pins its **span** as well as its finding, because a diagnostic on the right
document at the wrong place is one an editor underlines somewhere else, and findings arrive
in source order so that stays a promise rather than an implementation detail.

## What it does not report yet

`docs/toml.md` asks that a document wanting headers throughout put its tables last, and hands
that to "the formatter — not the transpiler". The formatter never reorders, so it is really a
lint; but it is a lint about a **target**, and the linter reads no target. Whoever writes it
has to say first whether a rule may know where a document is going.
