# XML

`nuke-transpile` writes a canonical `Value` out as XML with `xml::to_string`, which wraps the
document in a `<nuke>` element and lays it out over two-space indentation without a trailing
newline; `xml::to_string_rooted` names the root something else. XML is the first target with
no data model at all — named elements and text, where the others have maps, lists and scalars
— so it is the first mapping that cannot be inverted, and the job becomes keeping the shape
recoverable where the types cannot be.

## The mapping

| canonical form | XML                                                          |
| -------------- | ------------------------------------------------------------ |
| the document   | one root element, `<nuke>` unless the caller names it         |
| tuple          | one element per field, named by the field, in order           |
| map            | one `<_entry>` per pair, holding a `<_key>` and a `<_value>`  |
| list           | one `<_item>` per element                                     |
| any atom       | text of its spelling: `Relative`, and `True`, `Null` too      |
| string         | text                                                          |
| integer        | text, every digit it came with                                |
| float          | text, the shortest that reads back as the same double         |

Every child sits two columns in from its parent, and a scalar's text sits between its tags on
one line, so nothing the layout adds is ever part of a value. An empty collection and the
empty string are the same empty element, written `<a></a>` and never `<a/>`, which are one
document to a parser anyway.

## A tuple is names and a map is entries

XML gives a tuple an exact spelling: an identifier is already an XML name, so a field can
never be refused and never needs quoting. It gives a map none, because association is not
naming. So this is the first target where the canonical form's `=` / `=>` split changes the
output rather than only what is refused, and the first backend that keeps the two apart.

Making them alike fails in both directions. Narrowing a map to what naming allows would draw
a line *inside* the strings that no author could predict — `"ll"` would key an element and
`"a b"`, `"42"` and `""` would not. Widening a tuple to entry form would destroy the one
place where XML and Nuke agree exactly. So `fixtures/valid/maps.nuke` and
`fixtures/valid/collections.nuke` cross whole, as they do in YAML and do not in JSON or TOML,
and by a different route: YAML has a spelling for a complex key, and XML never needs one. An
author who wants `<ll>eza -l</ll>` writes a tuple, which is legal Nuke and the natural
spelling; the verbosity of `<_entry>` is paid only by the author who wrote `=>`.

The structural names take a leading underscore because `item`, `entry`, `key` and `value` are
ordinary identifiers, and without one `{a = {item = 1}}` and `{a = [1]}` would be one
document. `_` closes that totally rather than probably: an identifier must begin with a
lowercase letter and an XML name may begin with `_`, so nothing can collide.

`<_item>` is what keeps a list a list. Repeating the parent's element name is what a real
schema does, and it loses an empty list, cannot nest, cannot be a root — five of the eight
fixtures are root lists — and leaves a one-element list indistinguishable from its element.
The cost is that a format whose schema repeats an element, `fontconfig`'s `<match>` or
Maven's `<dependency>`, cannot be written here. That fact belongs to a schema and not to a
value, so the surface language is where to supply it.

Nothing is ever an attribute. Attribute-value normalisation replaces every tab, newline and
carriage return with a space, silently, and `fixtures/valid/strings.nuke` holds three strings
that would lose their content that way; an attribute cannot nest or hold order either. So
every value is element content, with no exceptions and no `xml:space`.

## Every scalar is text

XML has a word for no atom, so JSON's rule keeps only its second clause: every atom becomes
its own spelling. That is TOML's reasoning for `Null`, generalised to everything, and one
atom rule then serves every position with no key-position special case. More is lost than in
JSON, because nothing is quoted: `True` and `"True"` arrive as the same four characters, as
do `42` and `"42"`, as do `""`, `{}` and `[]`. The one distinction that survives is that a
float carries a `.` or an `e`, so `1` and `1.0` still read differently. A `type` attribute
would buy the rest back and is refused: no consumer reads one, it would have to appear on
every element, and it would make XML the only backend that pretends to round-trip. An integer
keeps every digit, there being no numeric type to narrow it.

## The character set is the narrowest yet

XML 1.0's `Char` production excludes `U+0000`–`U+0008`, `U+000B`, `U+000C`, `U+000E`–`U+001F`,
`U+FFFE` and `U+FFFF`, and a character outside it has no character reference either, so there
is nothing to escape to. This is the first backend that refuses a *character*, and
`strings.nuke` is the one fixture that stops crossing, for its `U+0000`. Surrogates need no
rule, since a Rust `char` cannot hold one, and only a string can trip the check — an atom is
alphanumeric, an identifier lowercase, a number ASCII.

`&`, `<` and `>` are escaped, the last unconditionally rather than only where it closes
`]]>`. `U+000D` is written `&#xD;`, because a parser normalises a literal one to `U+000A`
before the application sees it, so the only carriage return that survives is a reference.
Everything else stands, `U+007F`–`U+009F`, `U+2028` and `U+FEFF` included, all of which YAML
escapes. YAML's governing sentence does not transfer: a document with no declaration is XML
1.0 by definition, and 1.0 normalises only `\r\n` and `\r`. An XML version is a fact the file
states, not a guess the reader makes, which is why none is written and the rule is this short.

## Errors carry a path, not a span

As in every backend, an error names where in the value it stands rather than where in the
source: `shell.aliases#3` is the third entry of that map, `keybindings[1]` the second element
of that list, and `the document` the root. There are three — a root name XML cannot give an
element, a character XML cannot carry, and a value nested deeper than `nuke_syntax::MAX_DEPTH`,
which a key counts toward as it does in YAML. That is fewer than JSON, and the first time a
backend has had fewer. There is no unrepresentable root, because an element wraps anything; no
unrepresentable key, because a key is content and not a name; and no duplicate key, because
nothing is named, so `{Relative => 1 "Relative" => 2}` is two entries and two elements.
