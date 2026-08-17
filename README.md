# Nuke

A simple total configuration language.

Nuke files are expressions. Evaluating one reduces it to a canonical form, which transpiles
to JSON, YAML, TOML, XML, KDL, Lua, INI, cfg and gitconfig — so a dot file can be written
once, in one language, and shared between the programs that need it.

```nuke
# A dot file in the canonical form.
{
  editor = {
    theme = "gruvbox-dark"
    tab_width = 2
    line_numbers = Relative
  }

  shell = {
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

## Status

Early. The canonical form is specified — [`grammar/canonical.abnf`](grammar/canonical.abnf)
is normative and [`docs/canonical-form.md`](docs/canonical-form.md) covers what the grammar
cannot state. `crates/nuke-syntax` parses it, and its `serde` feature reads a document
straight into a Rust type; [`docs/serde.md`](docs/serde.md) records what that carries and
what it cannot. `crates/nuke-transpile` writes JSON, YAML, TOML, XML, KDL, Lua and INI:
[`docs/json.md`](docs/json.md) settles how atoms, keys and numbers degrade for the targets
that follow, [`docs/yaml.md`](docs/yaml.md) records what a target wider than the canonical
form does with the room, [`docs/toml.md`](docs/toml.md) records what a target of a different
shape refuses, [`docs/xml.md`](docs/xml.md) records what a target with no data model at all
can still carry, [`docs/kdl.md`](docs/kdl.md) records what a backend declines when a target
spells one value several ways, [`docs/lua.md`](docs/lua.md) records which Lua a backend writes
for when the target is a family rather than one language, and [`docs/ini.md`](docs/ini.md)
records what is left to write when a target has no specification and no quoted form for a
name. cfg and gitconfig, the surface language and the tooling are still ahead.

## Development

```sh
nix develop -c cargo test --workspace --all-features
```

`crates/nuke-grammar` translates the ABNF to a pest grammar at test time and runs every
fixture under `fixtures/` through it, so the specification is executable and cannot drift
from the implementation.
