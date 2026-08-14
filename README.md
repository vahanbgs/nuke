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
what it cannot. `crates/nuke-transpile` writes JSON, the first of the targets and the one
that settles how atoms, keys and numbers degrade; [`docs/json.md`](docs/json.md) records what
that mapping loses. The other targets, the surface language and the tooling are still ahead.

## Development

```sh
nix develop -c cargo test --workspace --all-features
```

`crates/nuke-grammar` translates the ABNF to a pest grammar at test time and runs every
fixture under `fixtures/` through it, so the specification is executable and cannot drift
from the implementation.
