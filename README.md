# Nuke

A simple total configuration language.

Nuke files are expressions. Evaluating one reduces it to a canonical form, which transpiles
to JSON, YAML, TOML, XML, KDL, Lua, INI, gitconfig and Nix — so a dot file can be written
once, in one language, and shared between the programs that need it.

```nuke
# A dot file, with what it repeats named once.
palette := {accent = "#fe8019" muted = "#928374"}

{
  editor = {
    theme = "gruvbox-dark"
    cursor = palette.accent
    line_numbers = Relative
  }

  shell = {
    prompt_color = palette.accent
    comment_color = palette.muted
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
reference cycle has no spelling, which is how a language with names stays total. `.`
projects a field out of a tuple, so one named value can hold what several places need.

`@` calls a builtin, and `@import` reads another file's value — which is what lets one
palette be written once and read by every program that needs it:

```nuke
# ~/.config/nuke/palette.nuke
{accent = "#fe8019" muted = "#928374"}
```

```nuke
# ~/.config/nuke/shell.nuke
palette := @import "./palette.nuke"

{prompt_color = palette.accent comment_color = palette.muted}
```

The path is a string and never an expression, so a tool can list a file's dependencies
without running it. An imported file's bindings stay private and its fields are its surface,
so nothing needs exporting; a cycle between files is refused by name, because the ordering
that makes a cycle unspellable inside one file has no counterpart in a directory.

## Status

Early. A grammar is [`grammar/tokens.abnf`](grammar/tokens.abnf) — the token layer both
languages share — followed by a syntax layer, and the assembly is normative.
[`docs/canonical-form.md`](docs/canonical-form.md) covers what the grammar cannot state.
`crates/nuke-syntax` parses both forms, and its `serde` feature reads a canonical document
straight into a Rust type; [`docs/serde.md`](docs/serde.md) records what that carries and
what it cannot. `crates/nuke-transpile` writes JSON, YAML, TOML, XML, KDL, Lua, INI, gitconfig
and Nix: [`docs/json.md`](docs/json.md) settles how atoms, keys and numbers degrade for the
targets that follow, [`docs/yaml.md`](docs/yaml.md) records what a target wider than the
canonical form does with the room, [`docs/toml.md`](docs/toml.md) records what a target of a
different shape refuses, [`docs/xml.md`](docs/xml.md) records what a target with no data model
at all can still carry, [`docs/kdl.md`](docs/kdl.md) records what a backend declines when a
target spells one value several ways, [`docs/lua.md`](docs/lua.md) records which Lua a backend
writes for when the target is a family rather than one language, [`docs/ini.md`](docs/ini.md)
records what is left to write when a target has no specification and no quoted form for a name,
[`docs/gitconfig.md`](docs/gitconfig.md) records what a backend does when the target cannot
spell a name the language guarantees, and [`docs/nix.md`](docs/nix.md) records what a value
costs when the target is a programming language and its spelling depends on where it stands.

The surface language has begun. [`grammar/surface.abnf`](grammar/surface.abnf) adds bindings,
field access and calls, `crates/nuke-eval` reduces a document to the canonical form, and
[`docs/surface.md`](docs/surface.md) argues the rules the grammar cannot state, with
[`docs/imports.md`](docs/imports.md) taking the ones about files rather than text. There are two
builtins: `@import` reads a file, and `@concat` puts strings end to end — the first one about
values rather than files, which is what makes `@` a namespace rather than an import sigil. What
holds the two languages together is a test: evaluating a canonical document is the identity.
Operators, conditionals and the tooling are still ahead.

## Development

```sh
nix develop -c cargo test --workspace --all-features
```

`crates/nuke-grammar` assembles each grammar's ABNF, translates it to pest at test time and
runs every fixture under `fixtures/` through it, so the specification is executable and cannot
drift from the implementation.
