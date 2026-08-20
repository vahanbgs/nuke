# The surface language

A Nuke file is a document, and evaluating it reduces it to the canonical form —
`nuke_eval::eval_at` does both halves. `grammar/tokens.abnf` then `grammar/surface.abnf` are the
normative grammar; this records what they cannot. The surface language adds six things: **a name
stands for a value**, **a file stands in for its own braces**, **a projection reads out of a
collection**, **`@` calls a builtin**, **`$"…"` interpolates** — a value becoming text — and **a
number is spelled in a base other than ten**. `docs/interpolation.md` argues the fifth and
`docs/dyadic.md` the sixth, as `docs/imports.md` argues the rules about files.

## Bindings

`name := expr` binds a name and contributes nothing to the result, and a bare `:` is a token only
inside a hole. A binding stands at the **head** of the document or of a brace block, before any
pair: `{a = 1 n := 2}` is a syntax error rather than an order trap, and a list holds none, an
element's identity there being its position. So `{n := 1}` is `{}`.

## A file stands in for its own braces

A file whose value is a tuple may leave the braces to the file: what follows the bindings is either
one value or a **run of fields**, and the boundary of the text is what `{` and `}` would have been.
`accent := "#FE8019"` then `editor = {theme = accent}` is the tuple of one that needed a brace
before. That is Rust's arrangement — `mod foo { … }` and `foo.rs` are one thing — and it makes
`docs/imports.md`'s claim that a file's bindings are private and its fields are its surface true of
the *text* rather than of a tuple the author had to write. Four backends refuse a root that is no
table anyway, so there the outer brace was never a choice.

One field or more, so `{}` keeps its one spelling and a file that only binds names is still no
document. A map has none of it: a field run is settled after one identifier, where an entry could
not be told from a value until a whole one had been read and an `=>` found past it, so a map keeps
its braces. The canonical form gains nothing either, its document being still one value, so both
spellings stay legal and no tool prefers either.

## Scope is sequential, so a cycle cannot be written

A binding is visible below itself in its own block and inside the blocks nested there, and nowhere
else. **A binding's value is reduced before its own name enters scope**, so `n := n` names the one
above, as Rust's `let x = x;` does.

That single ordering is the totality argument: a name can see neither itself nor anything below it,
so a reference cycle has no spelling — no detector, no fixpoint, no thunk. It gives shadowing for
free too, the scope being a stack read from the top, while rebinding within *one* block is an error
for the reason a repeated field is. That holds inside one file, `docs/imports.md` being the rule
across them, and reduction is **eager**.

And a field is not a binding: `{a = 1 b = a}` fails on an unbound `a`, in a file with no braces
just as much. Were it one, a block would be read as a set for scoping and a sequence for output,
which is what the `=`/`=>` split refuses. So `{port := 8080 port = port}` is legal, and a bound
name can never *become* a field name.

## Projection

`expr.name` projects a field out of a tuple. It is postfix, reads left to right, and what it yields
is projectable again. What stands to its left is a **value** and not only a name, which lets
`@import "./palette.nuke".accent` be written without a binding. `ows` surrounds the dot, so `[a .b]`
is one element and `1.b` is the malformed number `1.`.

One operator, two right operands. A name is literal and reads a tuple's field; a parenthesised
expression reduces to the key an entry is read at, so `m.("accent")` and `m.(k)` read a map and
`l.(0)` a list, whose keys are its positions. Each collection answers one reader, which keeps `.a`
from being a second spelling of `.("a")`. The parenthesis costs nothing, grouping being owed `( )`
anyway, where `[ ]` would read as a list and postfix `m["a"]` cannot be spelled at all.

## Calls

`@import "./palette.nuke"` calls a builtin. The canonical form has no reserved words — which is why
`:=` is not `let` — and a sigil keeps it that way: `import` is a name only after `@`, so
`import := 1` stays legal however many builtins follow.

Which builtins exist is no question a grammar can answer, so the grammar names none and an unknown
one is a fault the evaluator raises. A call takes exactly **one** operand, a collection carrying no
separators, so a builtin wanting more takes a list — and an `operand` rather than a `value`, so
`@import "p.nuke".accent` projects out of the imported document rather than the path.

`@concat ["#" accent]` puts strings end to end. It is the second builtin and the first about
**values** rather than files, which makes `@` a namespace rather than an import sigil, and the
first to spend that last rule: `@concat []` is `""` and `@concat "a"` is refused, a string being no
list of one.

## What is checked when

Parse time is about names, resolution about files, reduction about values — so the parser gains
`DuplicateBinding` while `DuplicateKey` moves to the evaluator, a map key being an expression now.
A repeated *field* stays the parser's, braces or none.

Termination is not feasibility. `a := [1 1]` then `b := [a a]` doubles per line while the text stays
two levels deep, so `nuke_syntax::MAX_DEPTH` bounds none of it — it guards nesting, and sharing
explodes breadth. So the evaluator carries `MAX_VALUES` beside it, one document wide and files with
it, and checks depth as the use site's plus the bound value's, so a reduction can never build a
value the canonical parser would refuse to read back. And a string is one value however long it
is, so `MAX_BYTES` bounds any one a reduction builds.

## Errors carry a span, and an import carries a file

An evaluation error names a place in the source rather than in the value, because `Expr` carries
positions and `Value` deliberately does not — a position inside `Value` would make two identical
map keys different. A fault below a file boundary is wrapped in one naming that file. There are
twenty-three and `nuke_eval::ErrorKind` is the roster: nine belong to a document, ten to a call —
seven of those about files — and four to a hole.
