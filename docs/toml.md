# TOML

`nuke-transpile` writes a canonical `Value` out as TOML with `toml::to_string`. There is one
layout and it does not end with a newline. TOML is the first target that is neither wider nor
narrower than the canonical form but *differently shaped*: a document is not a tree of values
but a sequence of regions, so the question this backend settles is where a header may stand.

## The mapping

| canonical form  | TOML                                                          |
| --------------- | ------------------------------------------------------------- |
| tuple           | a table, field names as keys, in declaration order            |
| map             | a table — see keys below                                      |
| list            | an array, or a run of `[[headers]]` — see sections below      |
| `True` `False`  | `true` `false`                                                |
| any other atom  | a basic string of its spelling: `Relative`, and `Null` too    |
| string          | a basic string                                                |
| integer         | an integer, refused if it does not fit 64 bits                |
| float           | a float, the shortest text that reads back as the same double |

## The document is a table

A TOML document *is* a table, so a root that is not a tuple or a map is refused — five of the
eight valid fixtures are root lists and none of them cross. An empty root table is an empty
document.

`Null` has no TOML spelling at all, so it takes the second clause of JSON's atom rule rather
than a rule of its own: an atom the target has no word for becomes a string of its own
spelling. Refusing it instead would refuse a whole dot file over one absent field.

## A key is a string or an atom

Exactly JSON's rule, for JSON's reason: a TOML key is a string, and these are the two values
that already have a spelling as a word. A key keeps its atom spelling rather than its value
mapping, so `{True => 1}` is `True = 1`. Two keys naming one TOML key is an error rather than
an entry silently lost.

A key is written bare where TOML's bare-key alphabet allows and quoted where it does not.
Unlike YAML there is nothing to resolve — a TOML key is a string whichever way it is written —
so quoting only what must be quoted is both safe and what a hand-written file looks like.

## A section is written where a section fits

A header opens a region that runs until the next one, so a table written as `[a]` swallows
every sibling declared after it. The entries that can be sections are therefore the ones in a
table's *trailing run*: a non-empty tuple or map takes `[path]`, and a non-empty list of
non-empty tuples or maps takes one `[[path]]` per element. Everything before that run is
written as `key = value` in declaration order, as an inline table or an inline array where the
value is a collection. An empty collection is always inline, which keeps it out of the run for
the same reason YAML writes it in flow.

So a table is a section when nothing follows it that a header would swallow, and the
declaration order the canonical form promises survives intact — `fixtures/valid/dotfile.nuke`
comes out as the TOML a person would have written, except that `aliases` stays inline because
`history_size` follows it:

```toml
[shell]
aliases = {ll = "eza -l", gs = "git status"}
history_size = 10000

[[keybindings]]
keys = "ctrl+t"
action = "OpenTerminal"
```

The alternative is what most TOML writers do: hoist every scalar above every sub-table so that
each one earns a header. That buys `[shell.aliases]` at the price of reordering the document
its author wrote, and the order carries into diffs. The cost of keeping the order is one long
line where a large table precedes a scalar, since TOML 1.0 has no multi-line inline table. A
document that wants headers throughout is one whose tables come last, and the formatter — not
the transpiler — is what should say so.

## Numbers narrow, strings do not

TOML gives an integer 64 bits, so this is the first backend where a number can be refused:
one wider than `i64` is an error rather than a silent rounding, which is the narrowing
`docs/canonical-form.md` leaves to a backend. Anything that fits goes out as the digits it came
with. A float is written as ryu writes it, as in JSON — the text always carries a `.` or an
`e`, and TOML's own grammar makes either one a float, so nothing needs the shaping YAML's 1.1
compatibility forced.

A string is always a basic string. The literal and multi-line forms are formatting rather than
data, and no loader can tell which one a value came from. The escapes are JSON's plus `U+007F`,
which TOML forbids raw; hex digits are uppercase, as they are in Nuke's own `\u{…}`.

## Errors carry a path, not a span

As in every backend, an error names where in the value it stands rather than where in the
source: `shell.aliases#3` is the third entry of that map, `keybindings[1]` the second element
of that list, and `the document` the root. There are five — a root that is not a table, a key
that is neither a string nor an atom, two keys that name one TOML key, an integer too wide for
64 bits, and a value nested deeper than `nuke_syntax::MAX_DEPTH`. Everything else crosses.
