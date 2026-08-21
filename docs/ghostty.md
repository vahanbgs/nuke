# Ghostty

`nuke-transpile` writes a canonical `Value` out as a Ghostty config file with `ghostty::to_string`,
which puts one `name = value` on a line, writes no blank line and no comment, and does not end with
a newline. There is one layout because the file admits one: no header, no indentation, nothing
positional. Ghostty is the first target whose reader cannot be linked, only run, so this is the only
backend with no oracle in `[dev-dependencies]` and the only test file not ending in a round trip.
What settles the mapping instead is the binary, every rule here having been probed against Ghostty
1.3.1 with `+validate-config` and `+show-config`, and here that is the whole of the evidence.

What the probe found is what this backend settles: an empty value is an erasure, so it is the first
target where writing something can un-write something else.

## The mapping

| canonical form  | Ghostty                                                        |
| --------------- | -------------------------------------------------------------- |
| the document    | one line for each field of the root, and nothing above the first |
| tuple, map      | refused below the root; there is no section to put one in       |
| list            | the key repeated, one line per element, in order                |
| atom            | its spelling lowered, `-` where a word begins                   |
| string          | its text, bare or fenced                                        |
| integer         | its digits, every one it came with                              |
| float           | its shortest text, always carrying a `.` or an `e`              |

## An empty value is an erasure

`title =` does not set the title to nothing; it restores the default. So does `title = ""`, the part
no documentation states and only the probe gives: the empty string has no spelling at all here,
where INI writes `k=` and gitconfig writes `""`. `keybind =` clears every binding Ghostty ships
with, so an empty value is an instruction to forget rather than a short value.

The empty string is therefore refused, and an empty list with it. `docs/gitconfig.md` refused an
empty list because writing no line loses the variable; here the line is worse than lost, because a
file Ghostty reads before this one may have set the option and the erasure would reach back into it.
Two refusals from one fact, and the fact is that this target's syntax has a verb in it.

## The file is one level deep

The shallowest of any target. INI has a leading run of keys and then sections, gitconfig has no
leading run at all, and Ghostty is nothing but the leading run: a table below the root is refused,
which is `docs/ini.md`'s flattening argument one level higher and for the same reason, `{a = {b =
1}}` and `{"a-b" => 1}` being one document if it were taken. `fixtures/valid/tuples.nuke` shows the
three apart, gitconfig stopping at `name`, Ghostty at `nested` and INI at `nested.a`, and the prefix
each takes is how deep its shape goes.

## A repeated key is a list, and nothing here knows for which options

`docs/gitconfig.md`'s rule, taken for its reason — a repeated `keybind` is how a real Ghostty config
is written — but with a cost git did not have. git's manual calls every variable multivalued, so
nothing is lost by repeating one. Ghostty has both kinds: `keybind` accumulates and `title` takes
the last, and no rule tells them apart. So a list on an option that holds one value degrades to its
last element, and `{k = 1}` and `{k = [1]}` are one text as in gitconfig and Lua.

The backend cannot tell them apart for the same reason it cannot tell `font-family` from
`nonexistent-key`: Ghostty's names are a vocabulary in one Zig file, not a grammar. So the name rule
is a shape test and nothing more — lowercase letters, digits and `-`, beginning with a letter, the
narrowest alphabet of any target. A field is transliterated into it, `docs/gitconfig.md`'s rule and
for its reason, and here the one character they disagree on is the only one either holds alone, so
the map is a *bijection* and no field reaches the shape test at all — of Ghostty 1.3.1's 200 default
options none holds a `_` and 185 hold a `-`. `{font_family = 1}` and `{"font-family" => 1}` write
one line, while `{"font_family" => 1}` stays refused, a string being a literal and not a name. What
the test still catches is `Title` and `2x`.

Ghostty spells a value out of that same alphabet, so an atom is carried into it the same way, out of
`UpperCamelCase` rather than an identifier's case: `{cursor_style = Bar}` writes `cursor-style =
bar`, and `True` and `False` stop being a table of two to become the rule's first instance. It
reaches a value and never a name, an atom beginning uppercase where an option begins lowercase. Here
the separator is a majority and not a bijection — of the 107 values Ghostty 1.3.1 enumerates, 75 are
one word and 30 hold `-` against `block_hollow` and `io_uring` — and nothing normalises either side,
`cursor_style` failing as a name exactly as `bold_italic` fails as a value. So this is the first
target where crossing and being read are different questions, and the two the majority loses are the
plain case: `cursor-style = block-hollow` crosses and Ghostty answers `invalid value`, wanting
`block_hollow`, a string here and not an atom.

## The quote is a fence and not a syntax

There are no escapes whatsoever, and the whole of quoting is that an outer pair comes off after the
value is trimmed. So a value is bare unless its edge is whitespace or it already begins and ends
with a quote, and in that second case one more pair goes round it and comes back off, `""` writing
as `""""`. `#` and `;` are bare, unlike gitconfig, a comment owning its own line here, so `palette =
0=#FE8019` needs nothing.

Having no escape at all, every C0 control is refused, tab and newline included. `docs/nix.md` kept
its controls because Nix reads a raw one back exactly, and Ghostty does too — but a newline ends the
line, and once it is refused the rest are what `docs/gitconfig.md` called the characters a person
cannot type. Nothing at or above `U+0020` is refused, so `U+007F` and `U+0085` cross.

## Errors carry a path, not a span

As in every backend, an error names where in the value it stands: `editor` is that field of the root
and `a[1]` the second element of that list. There are seven — a root that is not a table, a value
with no place here, a key that is neither string nor atom, a name Ghostty could not give an option,
the empty string, an empty list, and a control. The name one is reachable from a key alone. There is
no eighth for two names folding into one, and it is unreachable rather than dropped: a `Map` admits
no two equal keys, the atom-beside-string collision `docs/json.md` refuses is capitalised, and a
field cannot fold either — a tuple refuses a repeated field, `-` never occurs in an identifier, and
a table is a tuple or a map and never both, so a field and a key are never siblings. There is no
depth error, the shape running out at one level as in INI. Everything else crosses.
