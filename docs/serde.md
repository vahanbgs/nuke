# Serde

`nuke-syntax` reads a Rust type out of a canonical document with `from_str`, and writes one
back into a `Value` with `to_value`. Both sit behind the crate's `serde` feature. This
document records where Nuke and serde's data model disagree and how the binding settles it.

## The mapping

| Rust                  | canonical form                                     |
| --------------------- | -------------------------------------------------- |
| `bool`                | the atoms `True` and `False`, and nothing else      |
| any integer width     | an integer, never a float                           |
| `f32`, `f64`          | a float, never an integer                           |
| `char`, `String`      | a string, never an atom                             |
| `()`, unit struct     | the empty block `{}`                                |
| `Option<T>`           | presence and absence — see below                    |
| `Vec<T>`, Rust tuples | a list                                              |
| a struct              | a tuple, and a map is refused                       |
| a map                 | a map, or a tuple, whose field names become keys    |
| a unit variant        | a bare atom, or a string if the name is not one     |
| any other variant     | a map of one entry, `{Ipv4 => [127 0 0 1]}`         |
| `Vec<u8>` as bytes    | a list of integers                                  |

Map keys are whatever the Rust type can build: `HashMap<i64, T>` reads `{1 => "one"}`.

## Atoms, and why `Option` is about presence

`True` and `False` are the only atoms the binding gives meaning to, and only because a Rust
`bool` has nowhere else to come from. **`Null` is an ordinary atom.** It fills a unit variant
like `Relative` does, and nothing else.

So `None` comes from a **missing tuple field**, and writing a `None` **leaves the field out**.
An optional field round-trips through omission in both directions. The consequence is that
absence has no spelling, and therefore cannot be written where a field cannot simply be
dropped — in a list, as a map key or value, or at the top of a document. `Vec<Option<T>>` is
not writable, which is truthful rather than convenient.

## Tuples and maps

A struct reads a tuple and refuses a map, because the spec makes the two different types.

A map reads either. That is forced rather than chosen. `serde` compiles a struct carrying
`#[serde(flatten)]` down to a map, so refusing a tuple there would silently change a struct's
syntax the moment a field gained the attribute. And `{}` parses as an empty tuple, so refusing
it would leave an empty `HashMap` unwritable. The asymmetry has one visible effect: with
`flatten`, reading accepts both spellings but writing produces a map.

A field name that is not an identifier — `rename_all = "camelCase"`, say — is an error rather
than a quiet fallback to a map.

## Enums

Rust spells variants in `UpperCamelCase`, which is atom shape and cannot be a field name, so
the map form is the only one available for a variant that carries anything. `rename_all` can
produce a name no atom can hold, so the tag position also accepts a string in both directions.

## `Value` itself

`Value` implements both traits, so it works as a catch-all field, and `from_str::<Value>`
agrees with `parse` on every fixture. That agreement needs help: serde has no `visit_struct`
beside `visit_map` and no visit for an atom, so a plain self-describing reading would turn
every tuple into a map and every atom into a string. `Value` therefore carries a private type
hint by magic name — the mechanism `serde_json::RawValue` and `toml`'s datetime use — which
our own deserializer answers exactly and any other ignores. The hint also carries an integer
too wide for serde's widest visit, since integers here are arbitrary width and serde stops at
128 bits.

A foreign format sees through the wrapper: a tuple becomes an object and an atom becomes a
string, which is the degradation a JSON backend would want anyway.

`Atom`, `Integer`, `Float`, `Tuple` and `Map` implement both traits too, so a field typed
`Atom` accepts atoms only and one typed `Integer` keeps its full width instead of narrowing.

## What serde cannot carry

`#[serde(untagged)]`, `#[serde(flatten)]` and internally tagged enums buffer their input
through serde's own `Content` type before deciding what to do with it. That type has no atom,
no tuple and no 128-bit integer, and it errors outright on an enum. Inside such a buffer:

- an atom other than `True` and `False` arrives as a string, so `Relative` and `"Relative"`
  become the same thing;
- a tuple and a string-keyed map become indistinguishable;
- an integer past 64 bits fails;
- a map with non-string keys in a `flatten` remainder fails.

These are limits of serde's model rather than of Nuke, and every format meets them. The first
is asserted in a test so that it stays a contract rather than a surprise.
