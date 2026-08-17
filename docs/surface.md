# The surface language

A Nuke file is an expression, and evaluating it reduces it to the canonical form. `nuke-syntax`
reads one into a `Document` with `surface::parse`, `nuke-eval` reduces that to a `Value` with
`reduce`, and `eval` does both. `grammar/tokens.abnf` then `grammar/surface.abnf` are the
normative grammar; this records what they cannot. The surface language adds two things to the
canonical form: **a name stands for a value**, and **a field projects out of a tuple.**

## Bindings

`name := expr` binds a name and contributes nothing to the result, so
`accent := "#FE8019" {editor = {theme = accent} terminal = {cursor = accent}}` is a tuple of two.

`:=` is a third binder beside `=` and `=>`, not a keyword: the canonical form has no reserved
words, and a brace block already dispatches on its pair operator, so `let name = expr` would make
`=` mean two things told apart by a word. A bare `:` is still no token, `{x: 1}` still fails, and
`MixedPairOperators` still means `=` beside `=>` and never a binding beside either.

A binding stands at the **head** of the document or of a brace block, before any pair. `{a = 1
n := 2}` is a syntax error rather than an order trap, and a list holds no binding, because an
element's identity in a list is its position and a binding takes text without taking one.

`{n := 1}` is `{}`, which `docs/canonical-form.md` says satisfies both readings. So a block of
bindings is empty data and bindings are no namespace anyone can be handed — which is why a
module's surface will have to be its fields when imports arrive.

## Scope is sequential, so a cycle cannot be written

A binding is visible below itself in its own block and inside the blocks nested there, and nowhere
else. **A binding's value is reduced before its own name enters scope**, so `n := n` names the one
above, exactly as Rust's `let x = x;` does.

That single ordering is the totality argument. A name can see neither itself nor anything below
it, so a reference cycle has no spelling — no cycle detector, no fixpoint, no thunk, nothing to
detect. It gives shadowing for free too: the scope is a stack read from the top, so an inner block
may rebind a name an outer one holds, while rebinding within *one* block is an error for the
reason a repeated field and a repeated key are. The claim holds inside a file; imports will need a
rule of their own. Reduction is **eager**, so a fault in a binding nothing reads is still a fault:
what a document denotes should not depend on which of its names it happens to use.

## A field is not a binding

`{a = 1 b = a}` fails on an unbound `a` — only `:=` introduces a name. Were a field also a binding,
a block would be read as a set for scoping and a sequence for output, two readings of one construct
and what the `=`/`=>` split refuses; and it would need a marker for a field that is not output,
which `docs/xml.md` has spent. So `{port := 8080 port = port}` is legal and means what it looks
like, and a bound name can never *become* a field name: a block dispatches on `ident =` first.

## Field access

`expr.name` projects a field out of a tuple. It is postfix, reads left to right, and what it
yields is projectable again. What stands to its left is a **value**, not only a name — which is
what will let an import be consumed the day imports arrive.

```nuke
palette := {accent = "#FE8019"} {editor = {theme = palette.accent} status = {fg = palette.accent}}
```

A tuple has fields; a map has entries keyed by values, and a list has positions. So `{"a" => 1}.a`
and `[1 2].a` are refused at reduction, not at parse time: no grammar can say "an expression
denoting a tuple", so a narrower operand removes no fault, only the spellings where the mistake
shows. Reading `.a` as the string key `"a"` is definable, and that is why it is declined: indexing
must exist anyway for the keys `.a` cannot reach, so `.` on a map would be a second spelling of one
operation. `{a.b = 1}` is no nested field either — a field *name* is an identifier, never an
expression, the mirror of a field not being a binding.

`ows` surrounds the dot as it surrounds every other operator, so a list element does not end at a
newline and `[a .b]` is one element; the alternative would make `.` the only place a space changes
a reading. The dot is also where greedy tokens finally bite: a number takes the whole run that
could belong to one, so `1.b` is the malformed number `1.` beside a name while `1 . b` is a
projection reduction turns down — both refused either way, which is all the collision costs.
Narrowing `frac` would only move that refusal from the lexer to the evaluator, and edit the
canonical form to buy a surface feature. Whitespace, `docs/canonical-form.md` says, is needed only
between two *values* that run together into one token; `.` is the first thing that is not a value
and can.

## What is checked when

Parse time is what is about names; reduction is what is about values. The parser keeps
`DuplicateField` and gains `DuplicateBinding`, and does no key check at all: a map key is an
expression now, so `{n := 1 n => "a" 1 => "b"}` is a collision no parser can see, and
`DuplicateKey` moves whole to the evaluator.

Termination is not feasibility. `a := [1 1]` then `b := [a a]` doubles per line while the text
stays two levels deep, so `nuke_syntax::MAX_DEPTH` bounds none of it — it guards nesting, and
sharing explodes breadth. So the evaluator carries `MAX_VALUES` beside it and checks depth as the
use site's plus the bound value's, and a reduction can never build a value the canonical parser
would refuse to read back. `MAX_DEPTH` bounds the *expression* now, since a chain of projections
nests no data but does nest the tree the evaluator walks.

## Errors carry a span, not a path

Where a transpile error names a place in the value, an evaluation error names a place in the
source, because `Expr` carries positions and `Value` deliberately does not — a position inside
`Value` would make two identical map keys different. There are seven: a syntax error the parser
raised, wrapped rather than flattened; a name nothing binds; a projection out of what is not a
tuple, and one of a field a tuple has not got; a repeated key; a value past `MAX_DEPTH`; and a
document past `MAX_VALUES`. Everything else reduces.
