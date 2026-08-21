# The surface language

A Nuke file is a document, and evaluating it reduces it to the canonical form — `nuke_eval::eval_at`
does both halves. `grammar/tokens.abnf` then `grammar/surface.abnf` are the normative grammar; this
records what they cannot. The surface language adds seven things: **a name stands for a value**, **a
file stands in for its own braces**, **a projection reads out of a collection**, **a group is one
value in parentheses**, **`@` names a builtin** that **`<|` and `|>` apply**, **`$"…"`
interpolates**, and **a number is spelled in a base other than ten** — the last two argued in
`docs/interpolation.md` and `docs/dyadic.md`, and files in `docs/imports.md`.

## Bindings

`name := expr` binds a name and contributes nothing to the result. A binding stands at the **head**
of the document or of a block, before any pair: `{a = 1 n := 2}` is a syntax error rather than an
order trap, and a list holds none, an element's identity being its position. So `{n := 1}` is `{}`.

## A file stands in for its own braces

A file whose value is a tuple may leave the braces to the file: what follows the bindings is either
one value or a **run of fields**, and the boundary of the text is what `{` and `}` would have been.
`accent := "#FE8019"` then `editor = {theme = accent}` is the tuple of one that needed a brace
before. That is Rust's arrangement — `mod foo { … }` and `foo.rs` are one thing — and it makes
`docs/imports.md`'s claim about private bindings and surface fields true of the *text*. Four
backends refuse a root that is no table anyway, so the brace was never a choice.

One field or more, so `{}` keeps its one spelling and a file that only binds names is still no
document. A map has none of it, a field run being settled after one identifier where an entry could
not be told from a value until an `=>` was found past it. The canonical form gains nothing, its
document being one value still, so both spellings stay legal and no tool prefers either.

## Scope is sequential, so a cycle cannot be written

A binding is visible below itself in its own block and inside the blocks nested there, and nowhere
else. **A binding's value is reduced before its own name enters scope**, so `n := n` names the one
above, as Rust's `let x = x;` does.

That single ordering is the totality argument: a name can see neither itself nor anything below it,
so a reference cycle has no spelling — no detector, no fixpoint, no thunk. It gives shadowing for
free too, the scope being a stack read from the top, while rebinding within *one* block is an error
for the reason a repeated field is. That holds inside one file, `docs/imports.md` being the rule
across them, and reduction is **eager**.

And a field is not a binding: `{a = 1 b = a}` fails on an unbound `a`, braces or none. Were it one,
a block would be read as a set for scoping and a sequence for output, which is what the `=`/`=>`
split refuses. So `{port := 8080 port = port}` is legal, and a bound name never *becomes* one.

## Projection, and the group it needed first

`expr.name` projects a field out of a tuple. It is postfix, and what it yields is projectable again.
What stands to its left is a **value**, which lets `(@import <| "./palette.nuke").accent` be written
without a binding; `ows` surrounds the dot, so `[a .b]` holds one element and `1.b` is the malformed
number `1.`.

One operator, two right operands. A name is literal and reads a tuple's field; a group reduces to
the key an entry is read at, so `m.("accent")` and `m.(k)` read a map and `l.(0)` a list, whose keys
are its positions. Each collection answers one reader, which keeps `.a` from being a second spelling
of `.("a")`.

That group now stands wherever a value does, which it could not before: `[f (x)]` is two elements
only while juxtaposition means nothing, and an application operator is what settles that. It pays
the bill owed since a call took an operand rather than a value, so `f <| p.a` reads as it looks and
`(f <| p).a` takes the parenthesis. It is a shape and not a step — `(1)` and `1` reduce alike — and
survives parsing only so that a document reprints as its author spelled it.

## Builtins, and applying one

`@import` names a builtin. The canonical form has no reserved words — which is why `:=` is not `let`
— and a sigil keeps it that way: `import` is a name only after `@`, so `import := 1` stays legal
however many builtins follow. The grammar names none, an unknown one being the evaluator's fault,
and `@concat` is the second and the first about **values** rather than files, which makes `@` a
namespace rather than an import sigil. A builtin is an operand and not yet a **value** — `Value`
spells no function — so a `@name` nothing applies is a fault too, until functions land.

`@import <| "./palette.nuke"` and `"./palette.nuke" |> @import` **apply** it, and reduce alike.
Application is an operator and not juxtaposition: a collection carries no separators, so `[f a b]`
could not tell a call of two arguments from one beside a value. Neither spelling spends a character
— `|` stays owed to disjunction, `<` and `>` to comparison — and each takes exactly **one**
argument, so a builtin wanting more takes a list: `@concat <| []` is `""` and `"a" |> @concat` is
refused. Two directions are not two spellings of one thing: `<|` is right-associative and the
loosest there is, `|>` left-associative and read as written, and no tool prefers either.

## What is checked when, and where a fault stands

Parse time is about names, resolution about files, reduction about values — so the parser gains
`DuplicateBinding` while `DuplicateKey` moves to the evaluator, a map key being an expression now,
and a repeated *field* stays the parser's, braces or none.

Termination is not feasibility. `a := [1 1]` then `b := [a a]` doubles per line while the text stays
two levels deep, so `nuke_syntax::MAX_DEPTH` bounds none of it — it guards nesting, and sharing
explodes breadth. So the evaluator carries `MAX_VALUES` beside it, one document wide and files with
it, and checks depth as the use site's plus the bound value's, so a reduction can never build a
value the canonical parser would refuse to read back. A string being one value however long, it is
`MAX_BYTES` that bounds any one a reduction builds.

An evaluation error names a place in the source rather than in the value, because `Expr` carries
positions and `Value` deliberately does not — a position inside `Value` would make two identical map
keys different — and a fault below a file boundary is wrapped in one naming that file. There are
twenty-five, `nuke_eval::ErrorKind` being the roster: nine to a document, twelve to an application —
seven about files — and four to a hole.
