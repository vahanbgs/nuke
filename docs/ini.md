# INI

`nuke-transpile` writes a canonical `Value` out as INI with `ini::to_string`, which writes one
`key = value` to a line, a blank line before each `[header]`, and does not end with a newline.
There is one layout, `key=value` and `key: value` being the same pair to every reader that
takes them at all. INI is the first target with no single specification — it has several
conflicting ones, which is worse than none, because they disagree at the centre and not at the
edges as Lua's five did. So what this backend settles is what to write when there is no
document to conform to, and the answer is only what every reader it can name reads back whole.

Those readers are Win32's `GetPrivateProfileString`, Python's `configparser`, `rust-ini`,
`inih` and glib's `GKeyFile`; git is a later target and one that can quote. Two facts follow
and share one cause. INI gives a name no quoted form, so this is the first backend that must
draw the line *inside* the strings `docs/xml.md` declined to draw; and INI is two levels deep
by construction, so the writer never recurses and `nuke_syntax::MAX_DEPTH` cannot be reached —
the first backend with no depth error, and the one with the most other kinds instead.

## The mapping

| canonical form | INI                                                                 |
| -------------- | ------------------------------------------------------------------- |
| the document   | the root's leading run of keys, then a section for each of the rest |
| tuple          | one `key = value` per field, in declaration order                   |
| map            | the same, named by the key — see names below                        |
| list           | refused, in every position                                           |
| any atom       | its own spelling: `Relative`, and `True`, `False`, `Null` too       |
| string         | its text, unquoted and unescaped                                     |
| integer        | its digits, every one it came with                                   |
| float          | its shortest text, always carrying a `.` or an `e`                  |

## Keys lead and sections follow

This is TOML's rule reflected. A header opens a region running to the next one, so
`docs/toml.md` makes a section of a table only in a table's *trailing* run; a bare INI key is
readable only before the first header, so INI's keys are the root's *leading* run, a `position`
where TOML's is an `rposition`. Nothing else moves — no hoisting, no inline fallback — and a
scalar after a section is refused rather than lifted above it, for the reason TOML gave: order
outranks giving every table a header. What the run buys is the flat file — `.npmrc`, `.wgetrc`,
`mpv.conf` — which refusing a preamble would refuse too, `configparser` rejecting a wholly flat
document exactly as hard as one with a preamble. So the loss is named instead: a document with
a preamble is written for the readers that have one, and one that is all sections for all.

A list is refused everywhere, for two reasons the crate already owns. A repeated key is the
only spelling available, and Win32 and a non-strict `configparser` give back its last value
alone — an entry silently lost, which is the fault `DuplicateKey` refuses in four other
backends, seen from the writing side. And a repeated key is a list only to a reader that knows
the key is multi-valued, which `docs/xml.md` settled "belongs to a schema and not to a value,
so the surface language is where to supply it". A delimiter has none of that and no escape.

A table below a section is refused rather than flattened to `[a.b]`, which hands back a section
literally *named* `a.b`; with no escape for `.` that makes `{a = {b = {c = 1}}}` and
`{"a.b" => {c = 1}}` one document. That is a collision of shape and not of type, and these
backends lose types freely and never lose shape — XML's `_item` and TOML's refused root too.

## Every scalar is text

XML's rule transfers whole and this is the second target with no value types, so an atom keeps
its own spelling everywhere, `Null` included. More is lost than in XML, which kept a float's
`.`: here `1.0` and `"1.0"` are one text too. Nothing is resolved either — YAML needed a
quoting rule because `on` and `no` are booleans to a 1.1 loader, and INI needs none.

## A reader looks for syntax where a line begins

That one sentence derives the rest. A value never starts a line, so `#ffffff` crosses and `[`,
`]`, `=` and `:` stand inside a value untouched. Quote-stripping is positional too, so a value
beginning with `"` or `'` is refused; so is trimming, so one beginning or ending in whitespace
is refused, `rust-ini` trimming *after* it unescapes; and an inline comment is found after a
space, so a space before a `#` or a `;` is refused and nothing else about either is.

The backslash is the one character with no safe position, de-escaped anywhere by `rust-ini`,
`GKeyFile` and the Desktop Entry Specification alike, so `C:\temp` comes back with a tab in it.
It is refused beside the characters below `U+0020` and `U+007F`, which is the neat part: those
are exactly the ones `docs/canonical-form.md` makes a source escape, and INI has no escape to
offer. `U+0085`, `U+2028` and `U+2029` join them for the reason `docs/yaml.md` escapes them.
`%` does not, `configparser`'s interpolation being a pass over an already-parsed value rather
than anyone's grammar. An empty root is an empty document as in TOML, `{a = {}}` is a bare
`[a]`, and `""` writes `key =` while `" "` is refused.

## A name is narrower than a string

The name alphabet is the value alphabet and more, which is why one rule serves a section and a
key alike: a name is also refused when it is empty, holds any of `[ ] = :`, or begins with `#`
or `;` — all of it syntax where a name stands. `DEFAULT` is refused too, `configparser` giving
that section's keys to every other, and in key position as well so the alphabet stays one rule.
A field name is an identifier and always spellable, so only a map's keys reach the test, and a
key that is neither a string nor an atom is refused, exactly JSON's rule for JSON's reason —
`fixtures/valid/maps.nuke` stops at the integer key it stops at there. Two names the target
folds together are refused, and the fold is ASCII case: `configparser` lowercases an option
name and git lowercases both, so `{Theme => 1 "theme" => 2}` is two entries and one option —
`docs/lua.md`'s fold, which is the target's and not ours, at the coarsest any reader makes.

## Errors carry a path, not a span

As in every backend, an error names where in the value it stands rather than where in the
source: `shell.aliases` is that field of that tuple, `#2` the second entry of that map. There
are eight — a root that is not a table, a value INI has no place for, a key that is neither a
string nor an atom, a character it has no escape for, a name it cannot spell, a value it cannot
write literally, a key standing after a section, and two names it folds into one. There is no
ninth for depth, and every fixture stops at one of the eight. Everything else crosses.
