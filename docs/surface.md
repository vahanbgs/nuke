# The surface language

A Nuke file is an expression, and evaluating it reduces it to the canonical form. `nuke-syntax`
reads one into a `Document` with `surface::parse`, and `nuke-eval` reduces that to a `Value` with
`reduce`, or does both with `eval`. `grammar/tokens.abnf` then `grammar/surface.abnf` are the
normative grammar; this document records what the grammar cannot say. So far the surface language
adds one thing to the canonical form: **a name can stand for a value.**

## Bindings

`name := expr` binds a name and contributes nothing to the result.

```nuke
accent := "#FE8019"

{
  tab := 2

  editor = {theme = accent tab_width = tab}
  terminal = {cursor = accent}
}
```

`:=` is a third binder beside `=` and `=>`, not a keyword. The canonical form has no reserved
words, and a brace block already dispatches on its pair operator — which is why `let name = expr`
would be wrong here rather than merely unfamiliar: it would make `=` mean two things told apart by
a word. A bare `:` remains no token at all, so `{x: 1}` fails where it always did.

A binding stands at the **head** of the document or of a brace block, before any pair. `{a = 1
n := 2}` is a syntax error rather than an order trap, and a list holds no binding, because an
element's identity in a list is its position and a binding takes text without taking one.

`{n := 1}` is `{}`, which `docs/canonical-form.md` already says satisfies both readings. The
corollary matters later: a block of bindings is empty data, so bindings are not a namespace
anyone can be handed, and a module's surface will have to be its fields when imports arrive.

`MixedPairOperators` keeps its meaning exactly — `=` beside `=>`, never a binding beside either.

## Scope is sequential, so a cycle cannot be written

A binding is visible below itself in its own block and inside the blocks nested there, and
nowhere else. **A binding's value is reduced before its own name enters scope**, so `n := n` names
the one above, exactly as Rust's `let x = x;` does.

That single ordering is the totality argument. A name can see neither itself nor anything below
it, so a reference cycle has no spelling — there is no cycle detector, no fixpoint and no thunk,
because there is nothing to detect. It also gives shadowing for free: the scope is a stack read
from the top, so an inner block may rebind a name an outer one holds. Rebinding within *one*
block is an error, for the reason a repeated field and a repeated key are.

The claim holds inside a file. Imports will need a rule of their own, and this one does not
supply it.

Reduction is **eager**: a fault in a binding nothing reads is still a fault, because what a
document denotes should not depend on which of its names it happens to use.

## A field is not a binding

`{a = 1 b = a}` fails on an unbound `a`. Only `:=` introduces a name.

This keeps `docs/canonical-form.md`'s "a tuple is a sequence of fields rather than a map from
names to values" true of the surface language too. Were a field also a binding, a block would be
read as a set for scoping and as a sequence for output — two readings of one construct, which is
what the `=`/`=>` split exists to refuse. It would also need a marker for a field that is not
output, and there is none available: `docs/xml.md` depends on an identifier never beginning with
`_`.

So `{port := 8080 port = port}` is legal and means what it looks like. And a bound name can never
*become* a field name, because a block dispatches on `ident =` before any expression is read.

## What is checked when

Parse time is what is about names; reduction is what is about values.

The parser keeps `DuplicateField` and gains `DuplicateBinding`, and it does no key check at all.
A map key is an expression now, so `{n := 1 n => "a" 1 => "b"}` is a collision no parser can see,
and `DuplicateKey` moves whole to the evaluator.

Termination is not feasibility. `a := [1 1]` then `b := [a a]` doubles per line while the text
stays two levels deep, so `nuke_syntax::MAX_DEPTH` bounds none of it — it guards nesting, and
sharing explodes breadth. Bindings are where sharing enters the language, so the evaluator carries
`MAX_VALUES` beside it, and checks depth as the use site's plus the bound value's, so a reduction
can never build a value the canonical parser would refuse to read back.

## Errors carry a span, not a path

Where a transpile error names a place in the value, an evaluation error names a place in the
source, because `Expr` carries positions and `Value` deliberately does not — a position inside
`Value` would make two identical map keys different. There are five — a syntax error the parser
raised, wrapped rather than flattened; a name nothing binds; a key already in its map; a value
nested past `MAX_DEPTH`; and a document that expands past `MAX_VALUES`. Everything else reduces.
