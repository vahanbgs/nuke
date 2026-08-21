# gitconfig

`nuke-transpile` writes a canonical `Value` out as a git config file with `gitconfig::to_string`,
which puts a `[header]` on its own line, one tab-indented `name = value` to a line, a blank line
before each header, and does not end with a newline. git is the first target that cannot spell a
name this language guarantees: a variable name admits only alphanumerics and `-` and must begin with
a letter, so `tab_width` is not one. Every backend before this took a field name as given. So what
this backend settles is what to do when the target's names are narrower than the language's, and the
answer is to *transliterate* rather than refuse, the disagreement being exactly one character. The
shape is finite, so `nuke_syntax::MAX_DEPTH` is unreachable as it was in INI.

## The mapping

| canonical form | gitconfig                                                         |
| -------------- | ----------------------------------------------------------------- |
| the document   | a header for each field of the root, and nothing before the first |
| tuple, map     | a section, then a subsection, then the variables of either        |
| list           | the variable repeated, one line per element, in order             |
| any atom       | its own spelling: `Relative`, and `True`, `False`, `Null` too     |
| string         | its text, bare or quoted                                          |
| integer        | its digits, every one it came with                                |
| float          | its shortest text, always carrying a `.` or an `e`                |

## Every variable stands in a section

git reads no variable before a header, so the root holds headers alone and INI's leading run is
empty here. Its parser does take such a line, but no `git config --get` can name the key it makes,
so `tuples.nuke` stops on its first field and `maps.nuke` on its first entry, one earlier than
`docs/json.md` and `docs/ini.md`.

One level down, the rule TOML and INI each had to pay for is free. A `[a "s"]` header ends the plain
`[a]` region, but git merges a repeated header, so a variable after a subsection reopens `[a]`
rather than being refused or hoisted. Declaration order survives, which `docs/canonical-form.md`
promises by name, and INI's `StrayKey` has no counterpart here. A header that would stand empty
above its own subsection is not written; an empty table writes its own.

A table below a subsection is refused, the shape running out, and a `.` in a section name with it:
git reads `[a.b]` as the deprecated dotted subsection, so `{a = {b = {c = 1}}}` and `{"a.b" => {c =
1}}` would be one — `docs/ini.md`'s flattening argument whole.

## An identifier is a name and a string is a literal

Three alphabets, and the test is positional, the same text naming a subsection where it cannot name
a section. The two narrow ones cross Nuke's own rather than nesting inside it: `_` belongs to an
identifier and not to git, `-` to git and not to an identifier. Because they cross in that one
character and no other, a *field* is transliterated into git's — `tab_width` writes `tab-width` —
while a *key* is a literal written as it stands, so `{"diff-highlight" => …}` crosses and
`{"diff_highlight" => …}` is refused, and what a key alone reaches is what no identifier reaches:
`Core`, `1a`, `a.b`.

Never a subsection, which is not a name but an arbitrary string — a URL, a branch — where `_` is
legal and `[remote "my_fork"]` must stay itself. So one field spells two ways according to its
value: `{a = {b_c = 1}}` writes `b-c` and `{a = {b_c = {d = 1}}}` writes `[a "b_c"]`. The value
decides the position and the position decides the alphabet. A subsection is quoted, taking `b_c.d e`
and the empty string alike, and only `"` and `\` are escaped there.

The fold is the target's and there are two: git lowercases a section and a variable and compares a
subsection exactly, so `{Core => 1 "core" => 2}` is refused while two subsections differing in case
are two, `[Core] X = 1` beside `[core] x = 2` being *two values of one variable*. A field never
reaches that fold: `-` cannot occur in an identifier, so the transliteration is injective, and a
tuple already refuses a repeated field.

## A repeated variable is a list

KDL's argument rather than INI's. INI refuses a list because a repeated key gives back its last
value alone; a git variable is multivalued in git's own manual, so nothing is lost, and `[include]
path` twice over is what a real `.gitconfig` has. The cost is that `{k = 1}` and `{k = [1]}` are one
text. An empty list is refused, writing no line losing the variable rather than emptying it, and a
list holding a collection has none.

## A value is bare where the bare form gives it back

Nothing here is positional. `#` and `;` open a comment outside quotes, so the colour INI wrote bare
is quoted here. A value is bare when it holds no whitespace and none of `#`, `;`, `"` or `\`.

Every scalar is text, as in XML and INI, so `1` and `"1"` are one document, and an integer keeps
every digit. An atom keeps its spelling and, unlike Lua, loses nothing by it: git's boolean is
case-insensitive, so `True` is already its word for true. `Null` is the string `Null`, and a
`--type=bool` reader answers `bad boolean config value`, so this is the one target where absence
degrades into an error rather than a wrong value.

Only `"`, `\`, a newline and a tab are escaped, and no control is written literally — the rule every
backend before this kept. The rest are refused, not because git cannot carry them, a literal
`U+0001` crossing it verbatim, but because a dot file stays to characters a person can type.
`U+0008` earns the line: git documents `\b` for it and this backend refuses it anyway, `X\bY` being
three to git and one to `gix-config`, and an escape read two ways is no spelling.

## Errors carry a path, not a span

As in every backend, an error names where in the value it stands rather than where in the source:
`editor.line_numbers` is that field of that tuple, the path naming what the document wrote where the
message names what git would spell, and `#1` the first entry of that map. There are eight — a root
that is not a table, a variable outside a section, a value with no place here, an empty list, a key
that is neither a string nor an atom, a control with no spelling, a name git cannot give a section
or a variable, and two names it folds into one. The name error is reachable from a key alone now,
every identifier transliterating into an alphabet git spells. `dotfile.nuke` crosses every name it
has to stop at `keybindings`, a list outside a section and the latest of the three flat stops where
it was once the earliest. Everything else crosses.
