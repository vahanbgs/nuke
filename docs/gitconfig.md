# gitconfig

`nuke-transpile` writes a canonical `Value` out as a git config file with `gitconfig::to_string`,
which puts a `[header]` on its own line, one tab-indented `name = value` to a line, a blank line
before each header, and does not end with a newline. There is one layout, the one `git-config(1)`'s
own example shows. git is the first target that cannot spell a name this language guarantees: a
variable name admits only alphanumerics and `-` and must begin with a letter, so `tab_width` is not
one. Every backend before this took a field name as given, as `docs/ini.md` says outright, and
`dotfile.nuke` is the first fixture stopped by a *field*. So what this backend settles is what to do
when the target's names are narrower than the language's. Two facts pair with it: the same target
holds the *widest* name of any, a subsection taking every character but a control — what
`docs/ini.md` meant by "git is a later target and one that can quote" — so the narrowest and the
widest stand one level apart in one header; and the shape is finite, a section then an optional
subsection then a variable, so `nuke_syntax::MAX_DEPTH` is unreachable as it was in INI.

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
empty here. Its parser does take such a line, but the key it makes has no section and no
`git config --get` can name one, so `fixtures/valid/tuples.nuke` stops on its first field and
`fixtures/valid/maps.nuke` on its first entry, one earlier than `docs/json.md` and `docs/ini.md`.

One level down, the rule TOML and INI each had to pay for is free. A `[a "s"]` header ends the
plain `[a]` region, but git merges a repeated header, so a variable after a subsection reopens `[a]`
rather than being refused or hoisted. Declaration order survives in every case, which
`docs/canonical-form.md` promises gitconfig by name, and INI's `StrayKey` has no counterpart: it
existed because a bare INI key after a header has no way back, and here there is one. A header that
would stand empty above its own subsection is not written; an empty table writes its own.

A table below a subsection is refused, the shape running out, and a `.` in a section name with it:
git reads `[a.b]` as the deprecated dotted subsection, so taking it would make `{a = {b = {c = 1}}}`
and `{"a.b" => {c = 1}}` one document — `docs/ini.md`'s flattening argument transferred whole.

## A name is narrower than an identifier and another is wider than a string

Three alphabets, and the test is positional, the same text naming a subsection where it cannot name
a section. The two narrow ones cross Nuke's own rather than nesting inside it: `_` belongs to an
identifier and not to git, `-` to git and not to an identifier, so `core` is a section a tuple can
name and `diff-highlight` one only a map can. A subsection is quoted, taking `b_c.d e` and the empty
string alike, and only `"` and `\` are escaped there, git dropping a backslash before anything else.

The fold is the target's and there are two: git lowercases a section and a variable and compares a
subsection exactly, so `{Core => 1 "core" => 2}` is refused while two subsections differing in case
are two. Refusing it is what makes the next section's spelling mean anything — `[Core] X = 1` beside
`[core] x = 2` is *two values of one variable*, the same bytes a two-element list writes.

## A repeated variable is a list

KDL's argument rather than INI's. INI refuses a list because a repeated key gives back its last
value alone; a git variable is multivalued in git's own manual, `--get-all` naming it, so nothing is
lost. Refusing would leave multivalued git config unwritable here at all — a repeated field and a
repeated map key are both errors — and `[include] path` twice over is what a real `.gitconfig` has.

The cost is that `{k = 1}` and `{k = [1]}` are one text — `docs/lua.md`'s array part again, the
aggregate being the target's own rather than a distinction it could have kept. An empty list is
refused, writing no line losing the variable rather than emptying it — Lua's rule that a lost field
is worse than a wrong one — and a list holding a collection has no spelling at all.

## A value is bare where the bare form gives it back

Nothing here is positional, where INI derived a whole section from where a line begins: `#` and `;`
open a comment outside quotes, so the colour INI wrote bare is quoted here. A value is bare when it
holds no whitespace and none of `#`, `;`, `"` or `\` — `docs/yaml.md`'s conditional rule.

Every scalar is text, as in XML and INI, so `1` and `"1"` are one document, and an integer keeps
every digit, git's grammar typing nothing. An atom keeps its spelling and, unlike Lua, loses nothing
by it: git's boolean is case-insensitive, so `True` is already its word for true. `Null` is the
string `Null`, TOML's degradation and worse here — a `--type=bool` reader answers `bad boolean
config value`, so this is the one target where absence degrades into an error, not a wrong value.

Only `"`, `\`, a newline and a tab are escaped, and no control is written literally — the rule every
backend before this kept, by an escape or by a refusal. So the rest are refused, not because git
cannot carry them, a literal `U+0001` crossing it verbatim, but because a dot file stays to
characters a person can type. `U+0008` earns the line: git documents `\b` for it, so it is the one
control git can spell and this backend refuses anyway, `X\bY` being three characters to git and one
to `gix-config`, which applies the backspace. An escape read two ways is not a spelling, as
`docs/lua.md` held of `\u{…}`; nothing at or above `U+0020` is refused, so `U+0085` crosses here.

## Errors carry a path, not a span

As in every backend, an error names where in the value it stands rather than where in the source:
`editor.tab_width` is that field of that tuple, `#1` the first entry of that map. There are
eight — a root that is not a table, a variable outside a section, a value with no place here, an
empty list, a key that is neither a string nor an atom, a control with no spelling, a name git
cannot give a section or a variable, and two names it folds into one. There is no ninth for depth,
and INI's count is reached by another route. The key rule is inherited and no fixture reaches it,
`maps.nuke` stopping at `#1` where `docs/json.md` stops at `#2`. Everything else crosses.
