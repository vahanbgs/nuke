# KDL

`nuke-transpile` writes a canonical `Value` out as KDL with `kdl::to_string`, which lays the
document out over two-space indentation, one node to a line, and does not end with a newline.
There is one layout: `;` would pack a block onto one line, and a dot file is not a wire. KDL is the
widest target yet — an argument or a property, a bare word or a quoted string, four number bases,
two further string forms, a type annotation — while a node is only a name, and a name is not a
value. So what this backend settles is what to do with room that is not a distinction.

## The mapping

| canonical form        | KDL                                                     |
| --------------------- | ------------------------------------------------------- |
| tuple                 | one node per field, named by the field, in order        |
| map                   | one node per entry, named by the key — see keys below   |
| list                  | one `_item` node per element                            |
| `True` `False` `Null` | `#true` `#false` `#null`                                |
| any other atom        | a bare word of its spelling: `Relative`                 |
| string                | a quoted string, always                                 |
| integer               | a decimal number, every digit it came with              |
| float                 | a decimal number, always carrying a `.` or an `e`       |

## A node is a name and one value

A scalar is the node's argument and a collection is its block, so `tab_width 2` and
`editor { … }` are the whole shape, and a node never holds both. An empty tuple, map or list is
neither, so `{a = {} b = []}` writes two bare names; those three are one spelling as in XML, but
the empty string is not among them, `c = ""` writing `c ""`. An empty root is an empty document, as
in TOML.

Nothing is written as a property, and KDL settles that rather than taste: properties "SHOULD NOT
be assumed to be presented in a given order", and "children should be used if an order-sensitive
key/value data structure must be represented in KDL". A tuple is one, its fields being ordered,
which is why `docs/canonical-form.md` calls it a sequence of fields rather than a map from names
to values. A property cannot hold a collection either, so admitting them would make a tuple's shape
turn on whether its fields happen to be scalars.

A list takes `_item` for XML's reasons, which transfer whole: repeating the parent's name loses an
empty list, cannot nest, cannot be a root — five of eight fixtures are root lists — and leaves a
one-element list indistinguishable from its element. An argument run holds only scalars, and `k 1`
would be both `1` and `[1]`.

## A key is a string or an atom

Exactly JSON's rule, for JSON's reason, a third time after TOML: a KDL node name is a string, so
a string and an atom are the two values that already have a spelling as a word, and rendering
`42` or `[1 2]` into one would invent a spelling KDL never had. `fixtures/valid/maps.nuke` and
`fixtures/valid/collections.nuke` are the two that do not cross, at the same two paths JSON
refuses them, and a test asserts the two tables are one table.

XML's objection does not transfer, which is why a map is named here and bracketed there. XML
could not name most strings, so narrowing a map to what naming allows would have drawn a line
*inside* the strings that no author could predict. A KDL node name is a String rather than a
restricted alphabet, so `"a b"`, `"42"` and `""` all name nodes and that line does not exist.
Association is naming here — a block is an ordered, arbitrary-string-keyed run — so a tuple and
a map are honestly one construct, as in JSON, YAML and TOML.

A name is written the way the same value would be: an atom bare, a string quoted, a field name
bare. No alphabet test is needed, an identifier holding only `[a-z0-9_]` and an atom only
`[A-Z][A-Za-z0-9]*`, so both are always bare and it is the arbitrary string that is always
quoted. One rule then serves key and value position, and it buys YAML's position: `Relative` and
`"Relative"`, and `{a = 1}` and `{"a" => 1}`, differ in the text and not in what a loader holds.
An atom key keeps its spelling, and the target forces it — `#true` is a keyword, not a name. Five
words KDL reserves are legal field names — `true`, `false`, `null`, `inf`, `nan` — and are quoted,
which makes this the first backend to quote a *field* name.

Two keys naming one node is not an error, the one rule dropped rather than inherited. JSON
refuses a collision because a parser keeps the last pair, TOML because it is malformed, YAML
because two keys collapse into one node; an entry is lost in each. KDL permits a repeated node
name and orders its children, so `{Relative => 1 "Relative" => 2}` is two nodes and loses none.

## One spelling per value

Numbers are decimal only, the other three bases and the digit separator being the same integers
spelled differently. An integer keeps every digit, KDL's grammar bounding no width, though a
parser will narrow it — `kdl` stops at 128 bits. A string is always the single-line quoted form, the
raw and multi-line forms being formatting rather than data, as TOML argued. `#inf` and `#nan` are
unreachable, a `Float` being finite.

No type annotation is written, and that refusal differs from XML's: a `type` attribute was
invisible to a consumer, while a KDL annotation is in the grammar and every parser surfaces one,
so the reason is not that it would go unread but that it would make this the only backend
pretending to round-trip.

No character is refused. KDL forbids some code points *literally* but admits every one as a value
through `\u{…}`, with uppercase hex and no padding, which is Nuke's own spelling and makes this the
one target whose escape reads the way the source wrote it. The set is wider than XML's, taking the
bidi controls and `U+FEFF` that XML carries, and narrower where it matters — the one fixture XML
stops on, `fixtures/valid/strings.nuke`, crosses whole, and a test says so by name.

## Errors carry a path, not a span

As in every backend, an error names where in the value it stands rather than where in the source:
`shell.aliases#3` is the third entry of that map, `keybindings[1]` the second element of that
list, and `the document` the root. There are three — a root that is not a collection, a key that is
neither a string nor an atom, and a value nested deeper than `nuke_syntax::MAX_DEPTH`, which no key
counts toward, a key being a word. The root rests on a different fact from TOML's: KDL's README
names `-` as the conventional node for an anonymous value, so a spelling does exist, and taking it
would make `42` and `[42]` one document to a reader who knows it. Everything else crosses.
