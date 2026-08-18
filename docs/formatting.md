# Formatting

`nuke_syntax::printer::format` turns a document's text into the same document's text, and
`nuke fmt -` is that function on a pipe. What it decides is narrower than it sounds: a formatter
for a whitespace-insensitive language could rewrite every line, and this one deliberately does not.

## Spelling comes from the source, never from the tree

`0xFE8019` reduces to the integer `16678937`, and `Float` holds an `f64`, so `1.50` and `1.5` are
one value. A printer reading literals out of the AST would rewrite `docs/dyadic.md`'s whole
argument into decimal and drop a trailing zero. So every leaf is copied from its own span, and the
tree is consulted only for **structure**. Strings, interpolations and specifiers ride along on the
same rule: an escape is reproduced as written, the quotes' contents never being ours to change.

That is the formatter's half of the line `docs/embedding.md` draws. Reduction decides what a
document *means*; formatting decides only how it *reads*, and the two must not meet.

## The author's lines are kept, and the spacing between them is not

A block is written on one line or on several, and the formatter keeps whichever the author chose.
`{a = 1 b = 2}` stays, and the same fields split across lines stay split. Within a block that is
broken, items the author grouped onto one line stay grouped — which is what lets
`fixtures/valid/scalars.nuke` keep its four rows.

The reason is that **a list has no separators**. `[1 2 3]` is three elements and `[p . a . b]` is
one, so a reader finds an element's end by parsing rather than by looking, and the line the author
put it on is the only hint there is. A formatter that reflowed would be destroying the sole
signal a human has. Prettier keeps an object expanded for a weaker version of this reason; here it
is not taste but the grammar.

What is *not* the author's is everything between tokens: `{a:=1}` becomes `{a := 1}`, `p . a . b`
becomes `p.a.b`, and a run of blank lines becomes one. Three fixtures exist to demonstrate that
this whitespace is optional — `whitespace.nuke` twice and `access-whitespace.nuke` — and the
formatter putting it back is what they now also test.

The dot has one exception, and it is not cosmetic. `1 . b` is a projection off a number, which
reduces to nothing but parses; `1.b` is a malformed *number*, which does not. So the dot closes up
only when its operand is not a numeric literal, and `1 . b` becomes `1 .b` with the room that
keeps it an operator. A formatter is allowed to change how a document reads and never whether it
reads at all, and this is the one place in the grammar where closing a gap crosses that line.

Two more rules are the formatter's own rather than the author's. An expanded block starts its
items on the line after the delimiter, because `{ a = 1` with a broken tail reads as neither
shape. And a block that does not fit in **100 columns** is broken one item per line, because there
the author's single line is not a grouping they chose but the absence of one.

## Indentation is a tab, and alignment would be spaces

One tab per level of nesting, and nothing else. The argument is accessibility and it is the whole
argument: a tab is a width the **reader** chooses, so someone who needs eight columns to see the
structure and someone reading in a narrow split are looking at the same bytes and each getting
what they need. Spaces would freeze one of those two out in favour of whoever ran the formatter.
Nuke is read and edited by hand more than most languages are, being what a person writes to
configure their own machine, so the reader's choice matters here more rather than less. `gofmt`
settled this correctly and rustfmt did not; Nuke follows Rust where it has no reason of its own,
and here it has one.

Nothing is aligned today — every space the printer emits is a single separator between two tokens
— so the second half of the convention costs nothing yet. It is written down because it binds
later work: **if the formatter ever aligns anything, the alignment is spaces and only the leading
indentation is tabs.** Alignment that used tabs would move when the reader changed their tab
width, which is the one thing alignment must never do.

The cost is that **100 columns** stops being something the formatter can know. It assumes **8**
for the fit decision only, never for what it writes, and the asymmetry picks the number: assume
narrow and a reader on wide tabs gets lines running off the screen, assume wide and a reader on
narrow tabs gets lines broken a little earlier than they had to be. Only one of those hurts. No
fixture reflows at 8, so today the assumption costs nothing at all.

## Comments are re-attached, not preserved in place

The parser drops trivia, so `Document` has no comments in it. The lexer does not: it emits
`TokenKind::Comment` with a span, and `is_trivia` was already there to mark it. The printer walks
the tree and, before emitting anything at source offset *n*, flushes every comment that began
before *n* — on the same line if no newline separated it from what came before, on its own line
otherwise.

That is re-attachment by arithmetic rather than a concrete syntax tree. A comment between a
binding's name and its `:=` needs the binder's position, which `Binding` does not carry, so the
printer scans the gap for the operator, skipping comments — the gap holds nothing else. What it
does not buy is incremental reparsing; if the LSP server wants that, a rowan tree replaces it.

## What holds it

Every fixture that parses — `fixtures/valid` and `fixtures/surface/{valid,reduced,refused,modules}`
— is held to four properties: the output parses, `fmt(fmt(x))` is `fmt(x)`, every comment comes out
character for character in the same order, and the value the document reduces to is unchanged. The
last matters most and lives in `nuke-eval`, because that is where reduction lives. A fifth says the
corpus is *already* formatted, so a fixture added in the wrong shape fails the suite rather than
quietly setting a second precedent; a sixth reads every formatted line back and checks that no
indentation is a space and no tab stands past it.

## What it does not do

There is no configuration: no width flag, no indent flag, no way to ask for the other brace style.
A formatter with knobs is a formatter each project answers differently, which is the argument
`docs/kdl.md` makes against a room that is not a distinction. `nuke fmt` writes no file either —
it reads a path or `-` and prints, because `docs/embedding.md` says the command line grows no verb
that writes one, and a filter is not a writer. What rewrites the file is the editor that called it.
