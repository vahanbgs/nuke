# The canonical form

Every Nuke file is an expression. Evaluating it reduces it to the **canonical form**: the
subset of Nuke that carries data and nothing else. It is to Nuke what JSON is to JavaScript,
and it is the input every transpiler backend consumes. `grammar/tokens.abnf` then
`grammar/canonical.abnf` are the normative grammar; this document records what it cannot say.

## Values

A document is exactly one value. There are six:

| form   | example                 |
| ------ | ----------------------- |
| tuple  | `{host = "localhost" port = 80}` |
| map    | `{"a" => 1 [2 3] => True}` |
| list   | `[1 "two" Three]`       |
| atom   | `True` `False` `Null` `Relative` |
| string | `"text"`                |
| number | `42` `-1.5` `2.5e-3`    |

Collections carry no separators. Tuples name their fields with lowercase identifiers and
bind them with `=`; maps key arbitrary values with `=>`. `True`, `False` and `Null` are
ordinary atoms with no special treatment.

## Greedy tokens

Whitespace is insignificant, but tokens are matched greedily: at any position a token
extends as far as it can. Whitespace is therefore never *required* between two values, only
between two values that would otherwise run together into a single token. `[12]` holds one
element and `[1 2]` holds two; `[[1][2]]` and `{"a"=>1 "b"=>2}` need no spaces at all.

The rule cuts where a token genuinely cannot continue: `[1.2.3]` is an error, because the
first token is `1.2` and `.3` is not a value.

It is also why a number token takes the *whole* run of characters that could belong to a
number rather than only the well-formed ones. Were it narrower, a misspelling would split
into two correct tokens and parse silently: `[01]` would be `0` and `1`, and `[1E5]` would
be the number `1` beside the atom `E5`. So `number` is permissive, and every token it takes
must also match `canonical-number`, where the one-spelling rules live — a misspelled number
is one bad token that names itself, not two good ones.

## Encoding

Files are UTF-8 with no byte order mark. The only whitespace characters are space, tab and
line feed; a carriage return is a syntax error wherever it appears, so canonical files are
LF-only. Parsers may accept CRLF input and formatters normalise it; what they write is canonical.

## Strings

The escapes are `\"`, `\\`, `\n`, `\r`, `\t` and `\u{…}`. JSON's `\/` is gone because it
never earned its place, and `\b` and `\f` are gone because `\u{8}` and `\u{C}` say the same
thing. Escape letters and the `e` of an exponent are lowercase; hex digits are uppercase.

`\u{…}` takes one to six hex digits and denotes a Unicode scalar value directly, so JSON's
surrogate pairs are unnecessary. A value in `D800`–`DFFF` is a surrogate, not a scalar
value, and is rejected — the grammar admits it, so this is a check the parser makes.

Characters below `U+0020` must be escaped. Everything else stands for itself, including `#`,
which starts a comment only outside a string.

## Numbers

Integers and floats are distinct: a number is a float if and only if it carries a fraction
or an exponent. `1` and `1.0` are different values, which matters to TOML and to any backend
with a typed number model. Integers are arbitrary width in the grammar; a backend narrows
them and reports the loss. Floats are IEEE-754 doubles.

There is one way to write a given number, enforced by `canonical-number`: no leading `+` and
no leading zeros, in the exponent or before the point; no uppercase `E`; no digit-less
fraction such as `1.` or `.5`; and no base but ten, the dyadic literals of `docs/dyadic.md`,
which reduce to a decimal integer. That governs spelling, not the choice among equal floats —
`1e5` and `100000.0` are both canonical, and deciding between them is the author's: the
formatter copies every leaf from its own span, so it changes neither.
There is no infinity and no NaN; a backend that needs them takes them from atoms.

`-0` is admitted; as an integer it is `0`. `-0.0` and `0.0` are floats, and IEEE-754 keeps
them apart.

## Tuples and maps

A brace block is a tuple when its pairs use `=` and a map when they use `=>`. The two cannot
mix in one block. `{}` satisfies both readings and denotes the same empty collection either
way, so nothing turns on which one a parser records.

**Tuple fields are ordered.** Declaration order survives into YAML, TOML, gitconfig and
every other ordered target, and into diffs, so a tuple is a sequence of fields rather than a
map from names to values. Repeating a field name is an error.

**Map keys are compared structurally.** Two keys collide when they are the same value, not
when they are the same text: `1` and `1.0` are different keys, `[1 2]` and `[1 2]` are the
same one. A repeated key is an error. Entry order is preserved for the same reasons as
tuple field order, but it carries no meaning.

## Grammar, then lint

The grammar admits an atom starting with any uppercase letter and an identifier containing
any run of underscores. Style is narrower: atoms are `UpperCamelCase` and identifiers are
`snake_case` with no leading, trailing or doubled `_`. The linter enforces it and the parser
does not, so a file that trips a lint still parses.
