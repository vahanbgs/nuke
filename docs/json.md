# JSON

`nuke-transpile` writes a canonical `Value` out as JSON with `json::to_string`, which lays the
document out over two-space indentation, and `json::to_string_compact`, which writes it with no
whitespace at all. Neither ends with a newline. This document records the mapping and what it
loses; JSON is the narrowest of the targets, so the decisions here are the ones the later
backends inherit.

## The mapping

| canonical form        | JSON                                                |
| --------------------- | --------------------------------------------------- |
| tuple                 | an object, field names as keys, in declaration order |
| map                   | an object — see keys below                          |
| list                  | an array                                             |
| `True` `False` `Null` | `true` `false` `null`                               |
| any other atom        | a string of its spelling: `Relative` → `"Relative"` |
| string                | a string                                            |
| integer               | a number, every digit it came with                  |
| float                 | a number, always carrying a `.` or an `e`           |

## Atoms degrade to strings

JSON has no atom, and refusing every one would leave nothing to transpile: `Relative`,
`OpenTerminal` and `CloseWindow` are what a dot file is made of. So an atom that JSON has a
word for becomes that word, and every other atom becomes a string of its own spelling. What is
lost is that `Relative` and `"Relative"` arrive as the same JSON, which is the degradation a
program reading `settings.json` wanted anyway.

## A key is a string or an atom

JSON keys are strings, and these are the two values that already have a spelling as a word.
Rendering `42` or `[1 2]` into a key would invent a spelling JSON never had, so those are an
error rather than a guess — `fixtures/valid/maps.nuke` is a legal document that does not cross,
and a test says so by name. The atom form is not optional: serde writes an enum variant as
`{Ipv4 => …}`, so refusing atom keys would refuse every serialized enum.

A key keeps its atom spelling rather than its value mapping. `{True => 1}` is `{"True": 1}` and
not `{"true": 1}`, because in key position an atom is a name and not a boolean.

Because two different keys can name the same JSON key — `{Relative => 1 "Relative" => 2}` is a
legal map with two entries — a collision is an error too, rather than an entry silently lost.

## Numbers keep the shape they had

An integer goes out as the digits it was written with. JSON's grammar admits any number of
them, so an arbitrary-width integer crosses intact and what a reader does with it is the
reader's business: a consumer parsing into a double will round, and no backend can prevent
that.

A float always goes out with a `.` or an `e`, so `1.0` does not arrive as `1`. That keeps the
distinction the canonical form draws readable to a typed consumer, even though JSON itself does
not draw it. The text is the shortest that reads back as the same double.

## Errors carry a path, not a span

By the time a document is a `Value` the source is gone, so an error names where in the value it
stands: `shell.aliases#3` is the third entry of the map at `shell.aliases`, and `keybindings[1]`
is the second element of that list. A map entry is named by its ordinal because the thing that
cannot be printed may be the key itself. There are three errors — a key that is neither a string
nor an atom, two keys that name the same JSON key, and a value nested deeper than
`nuke_syntax::MAX_DEPTH`. Everything else always crosses.
