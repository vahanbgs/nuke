# Nuke

A simple total configuration language.

Nuke files are expressions. Evaluating one reduces it to a canonical form, which transpiles
to JSON, YAML, TOML, XML, KDL, Lua, INI and gitconfig — so a dot file can be written
once, in one language, and shared between the programs that need it.

```nuke
# A dot file, with what it repeats named once.
accent := "#fe8019"

{
  editor = {
    theme = "gruvbox-dark"
    cursor = accent
    line_numbers = Relative
  }

  shell = {
    prompt_color = accent
    aliases = {
      "ll" => "eza -l"
      "gs" => "git status"
    }
  }
}
```

Braces hold a named tuple when their pairs use `=` and a map when they use `=>`; maps take
any value as a key, not just strings. Brackets hold a list. Unquoted `UpperCamelCase` words
are atoms, which is all `True`, `False` and `Null` are. Nothing is separated by commas, and
whitespace only matters where two tokens would otherwise run together.

`:=` binds a name and puts nothing in the result. A binding is visible below itself and
inside the blocks nested there, and its value is read before its own name exists — so a
reference cycle has no spelling, which is how a language with names stays total.

## Status

Early. A grammar is [`grammar/tokens.abnf`](grammar/tokens.abnf) — the token layer both
languages share — followed by a syntax layer, and the assembly is normative.
[`docs/canonical-form.md`](docs/canonical-form.md) covers what the grammar cannot state.
`crates/nuke-syntax` parses both forms, and its `serde` feature reads a canonical document
straight into a Rust type; [`docs/serde.md`](docs/serde.md) records what that carries and
what it cannot. `crates/nuke-transpile` writes JSON, YAML, TOML, XML, KDL, Lua, INI and gitconfig:
[`docs/json.md`](docs/json.md) settles how atoms, keys and numbers degrade for the targets
that follow, [`docs/yaml.md`](docs/yaml.md) records what a target wider than the canonical
form does with the room, [`docs/toml.md`](docs/toml.md) records what a target of a different
shape refuses, [`docs/xml.md`](docs/xml.md) records what a target with no data model at all
can still carry, [`docs/kdl.md`](docs/kdl.md) records what a backend declines when a target
spells one value several ways, [`docs/lua.md`](docs/lua.md) records which Lua a backend writes
for when the target is a family rather than one language, [`docs/ini.md`](docs/ini.md) records
what is left to write when a target has no specification and no quoted form for a name, and
[`docs/gitconfig.md`](docs/gitconfig.md) records what a backend does when the target cannot
spell a name the language guarantees.

The surface language has begun. [`grammar/surface.abnf`](grammar/surface.abnf) adds bindings,
`crates/nuke-eval` reduces a document to the canonical form, and
[`docs/surface.md`](docs/surface.md) argues the rules the grammar cannot state. What holds the
two languages together is a test: evaluating a canonical document is the identity. Field
access, imports and the tooling are still ahead.

## Development

```sh
nix develop -c cargo test --workspace --all-features
```

`crates/nuke-grammar` assembles each grammar's ABNF, translates it to pest at test time and
runs every fixture under `fixtures/` through it, so the specification is executable and cannot
drift from the implementation.
