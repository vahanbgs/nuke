# The surface language

A Nuke file is an expression, and evaluating it reduces it to the canonical form —
`nuke_eval::eval_at` does both halves. `grammar/tokens.abnf` then `grammar/surface.abnf` are the
normative grammar; this records what they cannot. The surface language adds four things: **a name
stands for a value**, **a field projects out of a tuple**, **`@` calls a builtin**, and **`$"…"`
interpolates** — the one place a value becomes text, which is syntax and not a fourth builtin
because lambdas will make most builtins user-writeable and never supply syntax.
`docs/interpolation.md` argues that one, as `docs/imports.md` argues the rules about files.

## Bindings

`name := expr` binds a name and contributes nothing to the result, so
`accent := "#FE8019" {editor = {theme = accent}}` is a tuple of one. A bare `:` is a token only
inside a hole, where it opens a format specifier, and nowhere else.

A binding stands at the **head** of the document or of a brace block, before any pair. `{a = 1
n := 2}` is a syntax error rather than an order trap, and a list holds none, an element's identity
there being its position. So `{n := 1}` is `{}`: bindings are no namespace anyone can be handed.

## Scope is sequential, so a cycle cannot be written

A binding is visible below itself in its own block and inside the blocks nested there, and nowhere
else. **A binding's value is reduced before its own name enters scope**, so `n := n` names the one
above, exactly as Rust's `let x = x;` does.

That single ordering is the totality argument: a name can see neither itself nor anything below it,
so a reference cycle has no spelling — no detector, no fixpoint, no thunk. It gives shadowing for
free too, the scope being a stack read from the top, while rebinding within *one* block is an error
for the reason a repeated field and a repeated key are. The claim holds inside one file;
`docs/imports.md` is the rule across them. Reduction is **eager**, so a fault in a binding nothing
reads is still a fault.

And a field is not a binding: `{a = 1 b = a}` fails on an unbound `a`. Were it one, a block would
be read as a set for scoping and a sequence for output — two readings of one construct, which is
what the `=`/`=>` split refuses. So `{port := 8080 port = port}` is legal, and a bound name can
never *become* a field name.

## Field access

`expr.name` projects a field out of a tuple. It is postfix, reads left to right, and what it yields
is projectable again. What stands to its left is a **value** and not only a name, which is what
lets `@import "./palette.nuke".accent` be written without a binding.

A tuple has fields; a map has entries keyed by values and a list has positions, so `{"a" => 1}.a`
and `[1 2].a` are refused at reduction rather than at parse time: no grammar can say "an expression
denoting a tuple", so a narrower operand removes no fault, only the spellings where the mistake
shows. And `{a.b = 1}` is no nested field — a field *name* is an identifier, never an expression.

`ows` surrounds the dot as it surrounds every other operator, so `[a .b]` is one element and a list
element does not end at a newline. It is also where greedy tokens bite: `1.b` is the malformed
number `1.` beside a name, and only `1 . b` is a projection reduction turns down.

## Calls

`@import "./palette.nuke"` calls a builtin. The canonical form has no reserved words — which is why
`:=` is not `let` — and a sigil is what keeps it that way: `import` is a name only after `@`, so
`import := 1` stays legal and no word is spent however many builtins follow.

Which builtins exist is no question a grammar can answer, so the grammar names none and an unknown
one is a fault the evaluator raises. A call takes exactly **one** operand, a collection carrying no
separators, so a builtin wanting more takes a list. It takes an `operand` rather than a `value`, so
`@import "p.nuke".accent` projects out of the imported document rather than the path, at the price
that `@f p.a` is `(@f p).a` until grouping arrives.

`@concat ["#" accent]` puts strings end to end. It is the second builtin and the first about
**values** rather than files, which is what makes `@` a namespace rather than an import sigil, and
the first to spend that last rule: `@concat []` is `""`, and `@concat "a"` is refused because a
string is no list of one. It does not stringify, which a hole does instead, so it is for the parts
that arrive as a list rather than in a fixed shape. `join` stays unspent: it means "with a
separator" everywhere else, and a separator wants a built list.

## What is checked when

Parse time is about names, resolution about files, reduction about values — so the parser gains
`DuplicateBinding` while `DuplicateKey` moves to the evaluator, a map key being an expression now.

Termination is not feasibility. `a := [1 1]` then `b := [a a]` doubles per line while the text stays
two levels deep, so `nuke_syntax::MAX_DEPTH` bounds none of it — it guards nesting, and sharing
explodes breadth. So the evaluator carries `MAX_VALUES` beside it, one document wide and files with
it, and checks depth as the use site's plus the bound value's, so a reduction can never build a
value the canonical parser would refuse to read back. `MAX_DEPTH` bounds the *expression* now, a
chain of projections and a hole both nesting the tree the evaluator walks without nesting data. And
a string is one value however long it is, so twenty lines of `s := @concat [s s]` reach a megabyte
while that budget barely moves: `MAX_BYTES` bounds any one string a reduction builds.

## Errors carry a span, and an import carries a file

An evaluation error names a place in the source rather than in the value, because `Expr` carries
positions and `Value` deliberately does not — a position inside `Value` would make two identical
map keys different. With several texts rather than one, a fault below a file boundary is wrapped in
one naming that file and where in it it stands.

There are twenty-one. Seven belong to a document — a syntax error, wrapped rather than flattened; a
name nothing binds; a projection out of what is not a tuple, and one of a field a tuple has not got;
a repeated key; a value past `MAX_DEPTH`; a document past `MAX_VALUES`. Ten belong to a call, seven
of them about files and argued in `docs/imports.md`, the rest an operand that is not a list, a part
that is not a string and a string past `MAX_BYTES`. Four belong to a hole: a form with no text, a
float with no precision, a specifier the form has no use for, an integer too wide to respell.
