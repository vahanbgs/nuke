# YAML

`nuke-transpile` writes a canonical `Value` out as YAML with `yaml::to_string`, which lays the
document out in block style over two-space indentation and does not end with a newline. There is
no compact form: flow style is the only candidate, and a document written entirely in it is JSON
with fewer commas. YAML is the first target wider than the canonical form rather than narrower,
so this is where a backend first decides what to do with room JSON did not have.

## The mapping

| canonical form        | YAML                                                        |
| --------------------- | ----------------------------------------------------------- |
| tuple                 | a block mapping, field names as keys, in declaration order  |
| map                   | a block mapping — see keys below                            |
| list                  | a block sequence                                            |
| `True` `False` `Null` | `true` `false` `null`                                       |
| any other atom        | a plain scalar of its spelling: `Relative`                  |
| string                | a double-quoted scalar, always                              |
| integer               | a plain scalar, every digit it came with                    |
| float                 | a plain scalar with a `.`, and a sign on its exponent       |

Every child sits two columns in from its parent, a sequence under a mapping key included. An
empty collection has no block spelling, so `{}` and `[]` are written in flow on the line they
belong to.

## Valid 1.2, stable under 1.1

The target is YAML 1.2 that resolves to the same values under a YAML 1.1 loader, and every
quoting rule below falls out of that one sentence. The two versions disagree about which
unquoted words are booleans, and a dot file is read by whatever the program shipped with.

A string is therefore always quoted, which puts plain-scalar resolution out of reach for the one
form that could contain anything. What is left to guard is atoms and field names, whose
spellings are drawn from a small alphabet — an atom is `Relative`, a field name is `tab_width`.
Neither can hold an indicator or a space and neither can look like a number, so the only hazard
is a word some loader reads as something else: `y`, `n`, `yes`, `no`, `on`, `off`, `true`,
`false` and `null`, in any case. Those are quoted. `on` and `off` are legal field names and
common in dot files, so this is not a corner.

A float carries a point, and a sign on its exponent when it has one, because PyYAML's float
pattern requires both. `1.0e300` reads back as a string there and `1.0e+300` does not.

The escapes go further than JSON's. YAML cannot print `#x7F`–`#x9F`, `#xFFFE` or `#xFFFF`, and
1.1 loaders read `#x85`, `#x2028` and `#x2029` as line breaks, so all of those are written as
`\uXXXX` where JSON lets them through literally. `fixtures/valid/strings.nuke` holds two of
them, so reusing the JSON escaper would have written an invalid document on the first fixture.

## A key is any value

JSON refuses every key that is not a string or an atom, because rendering `42` or `[1 2]` into
one would invent a spelling JSON never had. YAML has the spelling. An entry whose key is a
tuple, a map or a list is written with `?` and `:` on lines of their own, and the key itself in
flow on one line:

```yaml
? [1, 2]
: "list key"
```

So `fixtures/valid/maps.nuke` and `fixtures/valid/collections.nuke`, the two documents JSON has
to refuse, cross whole, and a test says so by name. The cost is real and worth stating: PyYAML
and Go's `yaml.v3` parse a collection key and then fail to load it, and no serde-typed reader
can hold one at all. It is still the right trade, because "does the target have the spelling" is
a question that can be answered for every target still ahead, and "can a typical loader read it"
cannot be answered for XML or gitconfig at all.

An atom key carries its value rather than its spelling, which is the reverse of JSON's rule:
`{True => 1}` is `true: 1`. JSON was forced — its keys are strings, so `True` had to become some
string — and YAML is not. The gain is that one atom rule serves everywhere, with no key-position
special case and no question about what happens inside `{[True] => 1}`.

A string key keeps its quotes, so `"ll":` in a map reads differently from `ll:` in a tuple. YAML
resolves both to the same string; the difference is in the text and not in what a program loads.

## Errors carry a path, not a span

By the time a document is a `Value` the source is gone, so an error names where in the value it
stands: `shell.aliases#3` is the third entry of the map at `shell.aliases`, and `keybindings[1]`
is the second element of that list. There are two errors.

Two keys can still name one YAML node. `Relative` and `"Relative"` are two entries and one
scalar, as are `{x = 1}` and `{"x" => 1}`, so a collision is refused rather than an entry
silently lost — and because the collision is between nodes and not between spellings, the check
compares a structural fold of the key rather than its text. The other error is a value nested
deeper than `nuke_syntax::MAX_DEPTH`, and a key counts toward it, since here a key can be a
whole document. Everything else always crosses.
