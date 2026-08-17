# Nix

`nuke-transpile` writes a canonical `Value` out as Nix with `nix::to_string`, which lays the
document out over two-space indentation, one binding to a line, and does not end with a newline.
There is one layout: `{}` and `{ }` are the same set, and a `;` closes every binding, so where
Lua's last `,` could be dropped there is nothing to choose. Nix is the first target that is a
programming language rather than a data format, so its grammar must leave room for what a value
is *not*. What this backend settles is what that room costs, and the answer is a value's
spelling.

## The mapping

| canonical form        | Nix                                                       |
| --------------------- | --------------------------------------------------------- |
| the document          | one expression                                            |
| tuple                 | an attribute set, one `name = value;` per field, in order |
| map                   | an attribute set, one `"key" = value;` per entry           |
| list                  | brackets, with nothing between the elements               |
| `True` `False` `Null` | `true` `false` `null`                                     |
| any other atom        | a quoted string of its spelling                           |
| string                | a quoted string, always                                   |
| integer               | its digits, when a minus sign leaves them writable        |
| float                 | its shortest text, always carrying a `.`                  |

## A value's spelling depends on where it stands

Juxtaposition is application, so a list element has to be an expression that no operator opens,
and `-` is an operator rather than a sign. `[-1]` is `syntax error, unexpected '-'` and `[(-1)]`
is a list of one, a distinction no earlier target draws: everywhere else a scalar reads the same
wherever it sits. The writer parenthesises a negative number in list position and nowhere else —
`{a = -1;}` needs nothing, and a number in key position is refused before the question arises.

Any value opens a `.nix` file, so nothing is refused at the root, which is JSON, YAML and Lua's
position rather than TOML, KDL, INI and gitconfig's. An attribute name is a string, so the key
rule is JSON's for JSON's reason once more — rendering `42` or `[1 2]` into a name would invent
a spelling Nix never had. An atom key keeps its spelling and not its value, so `{True => 1}` is
`{"True" = 1;}`, and two keys that name one attribute are an error rather than an entry lost,
which is also Nix's own answer: it refuses a set that binds a name twice.

A tuple field is bare and a map key is always quoted, `docs/lua.md`'s rule, which needs no
alphabet test — an identifier holds only `[a-z0-9_]` and is a Nix name unless it is one of the
nine words the grammar reserves: `assert`, `else`, `if`, `in`, `inherit`, `let`, `rec`, `then`
and `with`. `or` is not among them, the grammar admitting it as an attribute alone, and `true`,
`false` and `null` are ordinary identifiers there, so `{null = 1;}` is a set with a field called
`null`. This is the third backend after KDL and Lua to quote a *field* name.

Order is the price, as it was in Lua, and here the target says so out loud. An attribute set is
keyed by name and prints sorted, so `{b = 1; a = 2;}` evaluates to `{a = 2; b = 1;}` and a
tuple's field order — which `docs/canonical-form.md` calls semantic, a tuple being a sequence of
fields — reaches the file and the diff but not the program. Lua's `pairs` was merely
unspecified; this is specified, so the loss is stated rather than warned about. Writing a map as
XML's run of entries would keep it, and is declined for Lua's reason: Nix names every string, so
a consumer writes `config.shell.aliases."ll"` rather than an index into a list of pairs.

## A string has no escape to fall back on

Nuke's five escapes are the ones Nix spells identically, and the sixth belongs to Nix: `${`
opens an antiquotation, so it is written `\${`. A `$` that no `{` follows is left alone, because
escaping it would change nothing and read as though it did.

There is no numeric escape at all. A backslash before a letter Nix has not claimed is that
letter, so `"\0"` is the character `0` and not `U+0000`, and `\u` is the tempting spelling that
never arrived rather than the one that arrived late, as it had in Lua. A character with no name
is therefore written as itself, and Nix reads a raw control character back exactly —
`builtins.stringLength` counts three in a string of `a`, `U+0001` and `b`. Refusing those would
be `docs/ini.md`'s rule past its reason: INI refuses a control character because it would come
back changed, and nothing here comes back changed.

The one character that cannot be carried is `U+0000`, which Nix refuses by name — *cannot be
represented as Nix string because it contains null bytes*. `fixtures/valid/strings.nuke` stops
at `[5]`, XML's path and XML's character, where JSON, YAML, KDL and Lua all carry it.

## A number Nix will read back

An integer is 64 bits and the range is not symmetric, which is new. `-9223372036854775808` is
not a literal but a negation of `9223372036854775808`, and that has already overflowed by the
time the minus is read, so the least integer Nix writes is one above the least it holds. This is
the third backend where a number can be refused for width, and the first where the bound is not
the type's: TOML's 64 bits are a width its specification declares and Lua's 2^53 is where five
implementations still agree, while this one is an artefact of the minus sign being an operator.

A float always carries a `.` in its mantissa, because `1e10` is not a float — it is the integer
`1` beside an identifier `e10`. That is `docs/yaml.md`'s reshaping of ryu's shortest text
without the `+` YAML needed. And a subnormal is refused: Nix reads a float with `strtod` and
takes `ERANGE` for a parse error, so `1.0e-310` is an `invalid float` where `3.0e-308` is not —
the only target that refuses a number for being too small. Rounding it to zero would hand back a
different number under the same name, and a lost type is what these backends spend, not a value.

## Errors carry a path, not a span

As in every backend, an error names where in the value it stands rather than where in the
source: `shell.aliases#3` is the third entry of that map, `keybindings[1]` the second element of
that list, and `the document` the root. There are six — a key that is neither a string nor an
atom, two keys that name one attribute, a character Nix cannot hold, an integer outside the
range a minus sign leaves writable, a subnormal float, and a value nested deeper than
`nuke_syntax::MAX_DEPTH`. The oracle is `rnix`, the parser nil and statix read Nix with, which
parses and does not evaluate — enough where only literals are written, and it stops at 512
nested expressions, so 128 is again the limit that bites. The last three errors are facts about
Nix's own reader that no parser reaches, and tests hold them by hand. Everything else crosses.
