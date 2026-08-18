# Property list

`nuke-transpile` writes a canonical `Value` out as an XML property list with `plist::to_string`,
which puts the XML declaration and Apple's DOCTYPE above one `<plist version="1.0">`, lays the
value out over two-space indentation, and does not end with a newline. Only the XML encoding is
written; the binary one would be a second format behind one function, which is the room
`docs/embedding.md` refuses.

`docs/xml.md` designed this backend and then declined to build it. Having erased every type it
carried, it weighed buying them back and refused: "a `type` attribute would buy the rest back and
is refused: no consumer reads one, it would have to appear on every element, and it would make XML
the only backend that pretends to round-trip." A property list is the format where all three
clauses fail at once — the type is the element's own name rather than an attribute, the DTD puts
one on every value, and every Apple framework reads it. So the rescue XML was right to decline is
what this target is made of, and it is the first target wider than the language: it types what
`docs/xml.md` erased, and then has room left over that nothing here can fill.

## The mapping

| canonical form | property list                                                     |
| -------------- | ----------------------------------------------------------------- |
| the document   | one object inside `<plist version="1.0">`, and any value may be it |
| tuple          | `<dict>`, a `<key>` naming each field, in order                    |
| map            | `<dict>`, the key's own text naming each entry, in order           |
| list           | `<array>`, one child per element, in order                         |
| `True` `False` | `<true/>` `<false/>`                                               |
| any other atom | `<string>` of its spelling, `Null` included                        |
| string         | `<string>`, the empty one written `<string></string>`              |
| integer        | `<integer>`, refused past 64 bits                                  |
| float          | `<real>`, the shortest text that reads back as the same double     |

An empty tuple and an empty map write `<dict/>`, an empty list `<array/>`.

## Three erasures come undone and one does not

`docs/xml.md` lost four distinctions and this target buys back three. `42` and `"42"` are
`<integer>` and `<string>`; `True` and `"True"` are `<true/>` and `<string>`; `""`, `{}` and `[]`
are three documents where XML wrote one. The fourth stays lost: there is no word for null here, so
`Null` falls to its own spelling under `docs/json.md`'s rule and `[Null]` and `["Null"]` are one.

Refusing `Null` was the alternative and is declined: it would make this the only target that
refuses a scalar the language guarantees, and stop `fixtures/valid/scalars.nuke` at a value no
other backend finds a fault in. The loss is worth stating rather than patching — this target is
wider than the language nearly everywhere and narrower in exactly one place, and the room it has
spare is `<data>` and `<date>`, not the one word it is missing.

## A tuple and a map are both a dict

This reverses the one structural decision `docs/xml.md` made. XML kept `=` and `=>` apart because
association is not naming and XML had no spelling for a map, so a map went to `<_entry>` holding a
`<_key>` and a `<_value>`. A `<key>` here **is** a string, so a map whose keys are strings is a
dict exactly, and `{"ll" => "eza -l"}` writes what an author who wrote a tuple would have got.

Carrying XML's entry form across was possible and is refused, because it would make a dict named
`_key` and `_value` — a schema no consumer reads, which is the one thing a target defined by being
read may not write. The cost is the two errors XML got to drop. XML had no unrepresentable key,
a key being content there, and no fold, nothing being named; both come back from `docs/json.md`
unchanged, with `{[1] => 1}` refused and `{Relative => 1 "Relative" => 2}` refused. That is the
trade whole: naming a thing is what typing it costs.

## Two types nothing here can write

`<data>` is bytes and `<date>` is a moment, and the canonical form has no value that means either.
`<uid>` makes three, being a plist type at all only inside an archive. Every earlier target was
narrower than the language, or wider in *spelling* — YAML's complex key, KDL's room that is not a
distinction — where this one is wider in *type*, and the room stays empty.

It stays empty rather than waiting for a builtin. A `Value` is seven variants and that is the whole
vocabulary; an eighth for one target would be a type only one backend could write and every other
would have to refuse. What `docs/xml.md` said of a repeating schema holds here — a fact belonging
to a target is not a fact belonging to a value. The difference is that this room is asserted empty
rather than merely described: a test walks every document this backend writes and fails on a
`<data>`, a `<date>` or a `<uid>`, so the claim rots loudly if it stops being true.

## The prologue is written where XML's was not

`docs/xml.md` writes no declaration, on the ground that "an XML version is a fact the file states,
not a guess the reader makes." The same sentence settles this the other way. A file with no DOCTYPE
is XML that resembles a property list, and the DOCTYPE is the whole of what makes it one, so the
file states it. Every framework reader accepts a plist without it, which is what makes writing it
a decision rather than a requirement: the file is for a reader that has other kinds of file.

## Errors carry a path, not a span

As in every backend, an error names where in the value it stands: `shell.aliases#3` is the third
entry of that map, `keybindings[1]` the second element of that list, and `the document` the root.
There are five, and they are the sum of the two documents this target stands between rather than a
set of its own. `docs/json.md`'s two, a key that is neither a string nor an atom and two keys
folding into one, because this target names. `docs/toml.md`'s wide integer, `<integer>` being
signed 64-bit where this language's is arbitrary. `docs/xml.md`'s character, XML 1.0's `Char`
production being no more negotiable for a plist than for XML. And the depth every nesting backend
carries, which a key counts toward only where a key is a value, which here it is not.

There is no unrepresentable root, a `<plist>` wrapping any object, so this is the only *typed*
target with no root restriction — TOML, KDL, INI, gitconfig and Ghostty all have one. There is no
float error, a `Float` being finite already and `<real>` a double. `fixtures/valid/collections.nuke`
and `fixtures/valid/maps.nuke` stop where JSON stops them, `fixtures/valid/strings.nuke` where XML
does, and those three are the three Nix refuses, at the same paths and by the same sum. Everything
else crosses.
