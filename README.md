# Nuke

A simple total configuration language.

Nuke files are expressions. Evaluating one reduces it to a canonical form, which transpiles
to JSON, YAML, TOML, XML, KDL, Lua, INI, gitconfig, Nix, Ghostty and plist — so a dot file can
written once, in one language, and shared between the programs that need it.

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

`$"…"` interpolates: `$"#{accent}"` and `$"{size}px"` build a string, a hole being the one place a
value becomes text, with Rust's specifier after a `:`. A plain string never pays for it. Its radix
mirrors the literals: `0xFE8019` is hex and `0b101110100xC` nine bits of binary then four of hex.

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

Early. A grammar is [`grammar/tokens.abnf`](grammar/tokens.abnf) — the token layer both languages
share — followed by a syntax layer, and the assembly is normative.
[`docs/canonical-form.md`](docs/canonical-form.md) covers what the grammar cannot state,
`crates/nuke-syntax` parses both forms, and [`docs/serde.md`](docs/serde.md) records what the
`serde` feature carries. `crates/nuke-transpile` writes the eleven targets above, each with a
document arguing what it degrades or refuses: [`docs/json.md`](docs/json.md) settles how atoms,
keys and numbers degrade for the ten that follow it, and each of those names the lesson only
that target teaches — one wider than the canonical form, one of a different shape, one with no
data model, one spelling a value several ways, one that is a family, one with no specification,
one that cannot spell a name Nuke guarantees, one positional, one whose reader cannot be linked,
and one wider than the language itself, typing what the target with no data model erased.

The surface language is finished. [`grammar/surface.abnf`](grammar/surface.abnf) adds bindings,
field access, calls, interpolation and dyadic literals, `crates/nuke-eval` reduces a document to
the canonical form, and [`docs/surface.md`](docs/surface.md) argues what the grammar cannot state,
with [`docs/imports.md`](docs/imports.md), [`docs/interpolation.md`](docs/interpolation.md) and
[`docs/dyadic.md`](docs/dyadic.md) taking files, text and the bases. What holds the two languages
together is a test: evaluating a canonical document is the identity.

Nuke is meant to be embedded. `eval_at` turns a file into a value, `bind::from_path` turns one
into a Rust type, `Target` names a backend so a host chooses one while it runs, and a reduction
reports the files it read. The binary `nuke` is the worked example — `render` writes a document
out, `deps` lists what built it — and it writes no file of its own, because where a dot file goes
belongs to a manager; [`docs/embedding.md`](docs/embedding.md) draws the line.

Editing it works. `nuke fmt` formats a document without reflowing it, and
[`tree-sitter-nuke/`](tree-sitter-nuke) is the grammar Helix takes its highlighting from — a
third grammar, this one for text that is not finished yet, held to the same fixtures with every
place it disagrees with the other two written down;
[`docs/highlighting.md`](docs/highlighting.md) argues them. `nuke lint` reports the style the
grammar admits and the convention does not, which is what the formatter may not fix for you:
[`docs/linting.md`](docs/linting.md) draws that line. `nuke lsp` serves all of it to an editor,
publishing the parser's, the linter's and the reducer's faults without confusing the three and
following a name to the binding it reads — `crates/nuke-resolve` is the scope pass the linter and
the server share, so neither can disagree with the other about what a word means;
[`docs/lsp.md`](docs/lsp.md) says what it refuses. Operators and conditionals come next, and
then `dot`, the manager this workspace was always for.

## Development

```sh
nix develop -c cargo test --workspace --all-features
nix develop -c ./tree-sitter-nuke/conformance.sh
```

`crates/nuke-grammar` assembles each grammar's ABNF, translates it to pest at test time and runs
every fixture through it, so the specification is executable and cannot drift from the code. The
second command does the same for the tree-sitter grammar, which is not a Cargo crate.
