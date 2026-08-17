# Interpolation

`$"gaps {size}px around {theme.name}"` builds a string. A hole is the **one place Nuke turns a
value into text**, which is why this is syntax and not a builtin: lambdas will make most builtins
user-writeable, and syntax is the one thing they never supply. `grammar/tokens.abnf` then
`grammar/surface.abnf` are normative; this records what they cannot say.

## Why a prefix, and why it is `$`

Tokens are greedy, and whitespace is required only where two of them would otherwise run
together. `f` and `"a"` do not, so `[f "a"]` and `[f"a"]` are the same two elements today; Python's
`f"…"` would need an identifier glued to a following string, which breaks the rule `[12]` and
`[1 2]` rest on. `$` begins no token at all, so `$"` glues with nothing to argue and `$ "a"` stays
two things, neither of which is a value. It costs no operator either: `+ - * / %`, `< > = !`,
`& | !`, `?` and `( )` are owed elsewhere, and `$` was never among them.

A hole lives inside `$"…"` and **never inside `"…"`**. A canonical string may hold any text, so a
sigil given meaning inside one would change what some canonical document already means, against
the invariant that evaluating a canonical document is the identity. Nothing crosses the other way:
the canonical form gains no rule, and an interpolation reduces to an ordinary string. So a plain
string stays literal to its last character, which is what `"{icon} {percentage}%"` needs.

## `{{` and `}}`

A literal brace is doubled, as in Rust. Rust's own reason does not transfer — `format!` is a macro
over an already unescaped literal, so its escape has to survive a prior pass, while `$"…"` is
syntax the lexer reads raw and could have taken `\{`. What applies instead is that `escape` belongs
to the token layer both languages share: a seventh escape would leave them disagreeing about what
an escape is, and `doubled` leaves it untouched. It is also the only convention that would still
work if a template ever arrived as data rather than as syntax.

A lone `}` is a fault, again as in Rust — the one spelling a plain string keeps and this one does
not. And `{{` wins over a hole, so a tuple in a hole is written `$"{ {a = 1}.a }"` with its space.

## What a hole holds

A hole takes a **value** rather than an operand, the braces having done the grouping a call cannot,
so `$"{p.a}"` and `$"{@import "./palette.nuke".accent}"` read the way they look. It is a level of
the expression the way a projection is, since it nests the tree the evaluator walks even though the
string it builds is flat. Of the six forms:

| form              | bare                        | with a specifier                       |
| ----------------- | --------------------------- | -------------------------------------- |
| string            | itself                      | padded and aligned                     |
| integer           | its own source text         | padded, signed, respelled in a radix   |
| float             | **refused**                 | admitted by a precision                |
| atom              | **refused**                 | refused                                |
| tuple, map, list  | **refused**                 | refused                                |

An integer keeps the text it was written as, and `canonical-number` admits one spelling per value,
so the hole hands text over rather than choosing it — `-0` included, settled as `0` when it was
read. A float is refused because `1e5` and `100000.0` are one value with two canonical spellings
and `docs/canonical-form.md` gives that choice to the formatter; a precision picks one, so the
refusal is "say how many digits" and not "you cannot". An atom is uninterpreted, and answering
`"True"` or `"true"` would be Nuke deciding what an atom means, which a backend does only because
its target leaves it nowhere else to put one. A collection's text would be Nuke's own syntax.

`@concat` still refuses a non-string part, and that is a distinction rather than an
inconsistency: a hole *spells* the conversion, the way `.` spells a projection, while a part in a
list has nothing saying it converts.

## The specifier

```abnf
spec     = [ aligned ] [ "+" ] [ "#" ] [ "0" ] [ uint ] [ "." uint ] [ notation ]
aligned  = ( fill align ) / align
align    = "<" / "^" / ">"
notation = %s"b" / %s"o" / %s"x" / %s"X" / %s"e"
```

Rust's, and checked against `format!` output rather than reasoned about: a width alone
right-aligns a number and left-aligns a string, `{:^6}` puts the odd space on the right, `{:06}`
on `-7` is `-00007`, and `{:#08X}` on `255` is `0x0000FF`, the prefix counting toward the width.
A width counts Unicode scalar values, which is what `\u{…}` denotes. A fill is any character but
`"`, `{` and `}`, and is a fill only when an alignment follows it, so `#` can be both.

`ows` stands before the `:` as it stands before every operator and behind it never. That makes the
specifier the one whitespace-sensitive thing outside a string — a fill may be a space, so
`{a: >6}` pads with them while `{a:>6 }` is a specifier that does not end.

Five departures from Rust, each a decision. `?` would mean "spell this as Nuke syntax", which is a
formatter's job and would let a document carry its own grammar as data. `{x:w$}` points into an
argument list a language with expressions in its holes has not got; a nested hole is the spelling
if it is ever wanted. A width is a `uint`, so `{n:007}` is refused for the reason `[01]` is. A
radix refuses a negative integer, a two's complement depending on a width Nuke's integers have
none of, and it narrows to `i128` and reports the loss as a backend does. And a specifier the form
has no use for is refused rather than ignored, where Rust's integer `Display` reads a precision and
quietly drops it.

## What is checked when

Parse time reads the pieces and the specifier: a lone `}`, a hole holding none or two values, one
never closed, a specifier that is not one, and `{a:}`, which the grammar admits and the parser
does not because `{a}` is the one spelling of a hole asking for nothing. Reduction is about
values, and owns the four faults `docs/surface.md` counts to a hole. `MAX_BYTES` bounds what an
interpolation builds, as it bounds `@concat`, a string being one value however long it is.
