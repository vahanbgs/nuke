# Dyadic literals

`grammar/tokens.abnf` then `grammar/surface.abnf` are the normative grammar; this records what
they cannot. A dyadic literal spells an integer as a bit string. It belongs to the surface
language alone and reduces to a decimal integer, so the base is gone before a value exists and no
backend can see it — `docs/canonical-form.md` fixes the one spelling that survives.

## A marker is a width

A literal opens with `0`, and after it a **marker** says how many bits each digit that follows
carries: `b` one, `q` two, `o` three, `x` four. A marker with its digits is a segment, and the
literal is its segments run together.

| literal         | segments                    | bits | value    |
| --------------- | --------------------------- | ---- | -------- |
| `0b1010`        | binary `1010`               | 4    | 10       |
| `0q3201`        | quaternary `3201`           | 8    | 225      |
| `0o755`         | octal `755`                 | 9    | 493      |
| `0xFE8019`      | hex `FE8019`                | 24   | 16678937 |
| `0b101110100xC` | binary `101110100`, hex `C` | 13   | 5964     |

Concatenating bit groups and reading the digits positionally are the same arithmetic —
`value << bits | digit` — so nothing turns on which reading is taken. The leading `0` carries no
bits. It is there so that a number still begins with a digit, which is what tells a lexer to look
for one; without it `xFF` would be an identifier.

Ten is no power of two, so a decimal digit has no width, there is no `0d`, and `0d5` is the
number `0` beside the name `d5`. That is the whole of the word *dyadic*: every base here is a
power of two, and the marker names the exponent. Four is where it stops because `F` is the last
digit with a spelling anyone agrees on.

## A marker stands anywhere, which is why this is one feature and not four

`0b101110100xC` is nine bits of binary and then four of hex. A bit pattern whose fields do not
divide by four is what HDL notation exists for, and writing each field in the base that fits it
beats writing the whole pattern in the base that fits none of them. The four bases are therefore
one literal form rather than four copies of one, and a literal in a single base is the degenerate
case rather than the rule.

What makes that unambiguous is a rule settled long before it: **hex digits are uppercase**, in
`\u{…}` and here alike. `A`–`F` being the only hex letters leaves `b`, `q`, `o` and `x` free to
mark a base everywhere, including straight after a hex digit — so `0xF35b1` is twelve bits of hex
and then one of binary, and it needs no separator to say so.

The price is `0xff`, which Rust admits and this refuses. Rust's reason does not transfer: it has
no second base inside a literal to protect, and Nuke had already spent lowercase hex once. A
misspelled literal is one bad token rather than two good ones, so `0xff` names itself instead of
reading as `0` beside `xff`, for the reason `1E5` does.

## What there is not

There is **no digit separator**, because a marker repeated is one: `0xDEADxBEEF` is `0xDEADBEEF`,
and the break costs the character `_` would have cost. Spending `_` beside it would be a second
spelling of the same silence.

A dyadic literal has no fraction and no exponent — there is no hex float, and `e` is a hex digit's
neighbour rather than a marker. A number token takes the point with it as it always did, so
`0x1.8` is one bad number, `0xFF.b` is the bad number `0xFF.` beside a name, and only `0xFF . b`
projects.

`-0xFF` is `-255`. The sign belongs to the number and not to the pattern, and refusing it would be
a special case for one form; there is no two's complement here, a Nuke integer having no width for
one to depend on.

Nothing may exceed **128 bits**. A decimal integer is arbitrary width because it is never
computed — it is the text it was written as — while any other base has to be evaluated into
decimal, and that is arithmetic. `docs/interpolation.md` takes the same ceiling going the other
way, where an integer too wide to respell is refused rather than truncated. Leading zeros cost
nothing: the ceiling is on the value, so `0x00FF` is 255 however many zeros precede it.

## The mirror

`{n:06X}` and `0xFE8019` are each other's reflection, and a dot file colour is where the pair
earns its keep: the value is written once in the base its readers think in, and reaches each of
them as text through a hole. The reflection is not perfect. A radix in a hole may be spelled
`x` for lowercase output, which is a spelling this side refuses; `#` writes the prefix as `0x`
whichever case follows it; and there is no quaternary notation to respell `0q3201`, Rust having
none to copy.
