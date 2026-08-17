# Lua

`nuke-transpile` writes a canonical `Value` out as Lua with `lua::to_string`, which lays the
document out over two-space indentation, one field to a line with a trailing `,`, and does not end
with a newline. There is one layout: `;` also separates fields and the last `,` could be dropped,
and neither is a distinction. Lua is the first target that is not one thing — PUC 5.1, LuaJIT, 5.2,
5.3 and 5.4 disagree about escapes and about whether a number can be an integer, and a dot file goes
to whichever one the program embeds. So what this backend settles is which Lua it writes for, and
the answer is all of them.

## The mapping

| canonical form | Lua                                                    |
| -------------- | ------------------------------------------------------ |
| the document   | `return` and one expression                            |
| tuple          | a table, one `name = value` per field, in order        |
| map            | a table, one `[key] = value` per entry — see keys below |
| list           | a table, one positional field per element              |
| `True` `False` | `true` `false`                                         |
| any other atom | a quoted string of its spelling, `Null` included       |
| string         | a quoted string, always                                |
| integer        | a number, and an integer subtype from 5.3 on           |
| float          | a number, always carrying a `.` or an `e`              |

## The document is a chunk that returns a value

A dot file is `require`d or `dofile`d, so the module convention is what a consumer already expects
and nothing is assigned to a global. `return` takes any expression, so any value opens a document —
`return 42` is an ordinary chunk — and Lua joins JSON and YAML in refusing nothing at the root,
where TOML and KDL both refuse a scalar.

A table is the only aggregate, so a tuple, a map and a list are one construct and each empty one
writes `{}`. The array part *is* integer keys, so `[1 2]` and `{1 => 1 2 => 2}` are one table.

Order is the price. `pairs` is unspecified, so a tuple's field order — which
`docs/canonical-form.md` calls semantic, a tuple being a sequence of fields rather than a map from
names to values — reaches the file and the diff but not the program. A map's order was never
semantic, so only the tuple pays. Writing a map as XML's run of entries would keep it, and is
declined for KDL's reason: Lua names every string, so no entry spelling is needed, and one would
produce a table no Lua program could index, a consumer writing `config.theme` and not
`config[1][2]`. KDL had both constructs and a spec naming which to use; here there is only one.

## A key is any scalar

The widest key rule after YAML's, and the first that drops JSON's rather than inheriting it. JSON
refuses `42` and `[1 2]` as keys because a JSON key is a string and rendering either into one would
invent a spelling; a Lua table key is a *value*, so `[42]`, `[-1.5]` and `[true]` are the target's
own spelling and refusing them would apply JSON's rule past its reason. A collection is refused for
a fact rather than a convention: a table keys by identity, so `[{1, 2}]` is a key no reader can
construct twice. `fixtures/valid/maps.nuke` and `fixtures/valid/collections.nuke` are still the two
that do not cross, but Lua stops at `#5` where JSON stops at `#2`, three keys further in, and a test
pins the coincidence and the difference together.

A key is written the way the same value would be, so an atom key carries its value rather than its
spelling, as in YAML and unlike JSON: `{True => 1}` is `[true] = 1`, and `"True"` stays a separate
entry. A map key is always bracketed and a tuple field is bare, which needs no alphabet test — an
identifier holds only `[a-z0-9_]` and is a Lua name unless it is one of the 22 words Lua reserves,
`goto` included, a keyword from 5.2 on. Those are bracketed too, which makes this the second backend
after KDL to quote a *field* name. The two spellings differ in the text and not in what a loader
holds, which is YAML's position again.

Two keys Lua folds into one are refused, and the fold is Lua's rather than ours: a float with an
integral value indexes as that integer, so `1` and `1.0` collide, and so do `0` and `-0.0`, which
`docs/canonical-form.md` keeps apart. An atom and the string of its spelling collide as well.

## Absence has a spelling and it cannot be used

`Null` becomes the string `"Null"`, which is TOML's degradation for a reason TOML never had. TOML
has no null; Lua has one and it is worse than none. `t.k = nil` is not a key holding nothing but no
key at all, so `{a = Null b = 1}` would arrive as a one-field table and `[1 Null 3]` as a list of
undefined length. A lost field is worse than a wrong one, so `nil` is never written, and `True` and
`False` are the only atoms with a Lua word. Refusing the document instead would refuse a whole dot
file over one absent field, as TOML argued.

## One chunk every Lua reads the same

A string takes Nuke's own five escapes, which Lua spells identically, and every other ASCII control
takes `\ddd`, three digits always so a following digit cannot join it. `\u{…}` is the tempting
spelling and it arrived in 5.3, which makes it exactly the escape a Neovim config must not contain.
Nothing above `U+007F` is escaped and nothing may be, since `\ddd` inserts one byte and `\133` for
`U+0085` would write half a character. No character is refused, so `fixtures/valid/strings.nuke`
crosses whole, `U+0000` included, where XML stopped on it.

An integer past 2^53 is refused, which makes this the second backend where a number can be, and the
line falls elsewhere than TOML's for a different kind of reason. TOML's 64 bits are a width its
specification declares; Lua's answer depends on who is reading, `9007199254740993` being exact in
5.4 and `…992` under LuaJIT, and a decimal literal past `i64` becoming a float in 5.4 without a
word. Taking `i64` would let one document mean two numbers. Floats need no such rule, every Lua
holding the same double, and what is written is the shortest text that reads back as it.

## Errors carry a path, not a span

As in every backend, an error names where in the value it stands rather than where in the source:
`shell.aliases#3` is the third entry of that map, `keybindings[1]` the second element of that list,
and `the document` the root. There are four — a key that is a collection, two keys Lua folds into
one, an integer past 2^53, and a value nested deeper than `nuke_syntax::MAX_DEPTH`, which no key
counts toward, a key being a scalar. Lua caps its own nesting near 200 constructors, so 128 is the
limit that bites, and a test holds that relation rather than a sentence claiming it. Everything
else crosses.
