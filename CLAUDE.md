# Nuke

Nuke is a simple total configuration language. This repository holds its specification, its
grammar in ABNF, and implementations in Rust of the tools used to work with it.

## A few facts about Nuke

- Nuke is whitespace insensitive, except inside a comment, a string, and a format specifier.
- Nuke configuration files are Nuke expressions which are evaluated and reduced down to a
  core canonical form, which is to Nuke what JSON is to Javascript.
- Nuke will first be used in a dot file manager which will allow users to write all of
  their dot files using a single language, sharing values between them with imports.

## Layout

- `grammar/` — `tokens.abnf` then `canonical.abnf` or `surface.abnf`; a grammar is that pair.
- `docs/canonical-form.md`, `docs/surface.md` — the rules ABNF cannot express, one per language,
  plus `docs/imports.md` for files, `docs/interpolation.md` for text, `docs/dyadic.md` for bases.
- `docs/serde.md` — where Nuke and serde's data model disagree; `docs/embedding.md` — where the
  language stops and a host begins; `docs/formatting.md` — what the formatter decides and leaves
  alone; `docs/highlighting.md` — what a grammar for unfinished text may disagree about.
- `docs/json.md` and its eight siblings — each mapping, and what it degrades or refuses.
- `fixtures/valid`, `fixtures/invalid` — conformance fixtures. Under `fixtures/surface`, `valid`
  pairs with `reduced` by name, `invalid` is what the parser refuses, `refused` what cannot, and
  `modules` holds the files those fixtures import, which are inputs rather than fixtures.
- `crates/nuke-fixtures` — reads those trees, so no crate keeps its own copy of the walker.
- `crates/nuke-grammar` — assembles a `Layer`'s ABNF, translates it to pest at test time and
  checks the fixtures against it, so the grammar is executable and cannot drift from the code.
- `crates/nuke-syntax` — one hand-written lexer, whose mode stack is what interpolation needs, and
  two parsers over it: `parse` for the canonical form, `surface::parse` for the surface language.
  Each carries what ABNF cannot state. `serde` adds `from_str`/`to_value`, and `printer` puts a
  document back to text, taking every literal's spelling from its span rather than from the tree.
- `crates/nuke-eval` — reduces a `Document` to a `Value`; owns the filesystem (resolution, the
  import cache, the cycle check) and `text.rs`, where a value becomes text under a specifier.
  `bind` reads a Rust type out of a surface document, and a reduction reports the files it read.
- `crates/nuke-transpile` — the backends, each owning its own `ErrorKind` and its own answer to what
  the target can spell: JSON, YAML, TOML, XML, KDL, Lua, INI, gitconfig, Nix. `docs/` argues each.
  `Target` names one and `Refusal` wraps the nine errors without flattening them.
- `crates/nuke-cli` — the binary `nuke`: `render` writes a document out, `deps` lists what it read,
  `fmt` formats one from a path or `-`. None of the three writes a file.
- `tree-sitter-nuke/` — the surface language for editors, outside the workspace because a Rust
  binding would be unsafe. `test/verdicts` names every fixture it does not simply accept.

Work in the devshell: `nix develop -c cargo test --workspace --all-features` covers serde too,
and `-c ./tree-sitter-nuke/conformance.sh` the grammar an editor loads, which is no Cargo crate.

## Rules

- Never write comments in Rust code.
- Never use unsafe Rust code. It's okay to depend on crates that do if they are staples in
  the Rust community.
- Never hand-roll something if there are popular crates which are staples in the Rust
  community that do it better.
- All documents that only contain prose should stay under 100 lines long (including
  CLAUDE.md). Do not hesitate to remove old information that is obvious, useless,
  unimportant or outdated to replace it with new relevant, useful or important information.
- Use best practices for code maintainability, correctness, performance and compilation speed.
- Always write tests.
- Make compilation fail if clippy emits warnings.
- Never use directives to silence warnings.
- Always run cargo fmt/rustfmt.
- Use a Nix flake with a devshell to make our dev environment reproducible and to distribute
  our tooling. Never install tools system-wide, and try not to rely on ones that are.
- When possible use modern equivalents of CLI tools when interacting with the file system
  (ripgrep instead of grep, fd instead of find, etc..).

## Commits

Conventional Commits for the subject line, and a prose body that carries the argument. The
convention says nothing about bodies, so the two compose.

- `<type>(<scope>): <imperative subject>`, no trailing period, 72 characters or fewer.
- Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`. There is
  no `style` — rustfmt is enforced, so a style-only commit cannot exist.
- Scope is the area changed: `grammar`, `spec`, `fixtures`, or a crate name without its
  `nuke-` prefix (`syntax`, `transpile`, `cli`). Omit it when a change is repository-wide.
- A body is required for anything but a trivial change: what was decided, and what tradeoff
  it accepted. For a language decision the body is the deliverable, not a formality.
- `!` before the colon for a breaking change, with a `BREAKING CHANGE:` footer.

## Plan

We are taking baby steps.

- [x] The canonical form: its ABNF, a hand-written lexer and parser checked against that grammar
      and against the fixtures, and serde's `Value` with `from_str` into a user's type.
- [x] The transpiler: nine backends, each owning its `ErrorKind` and its answer to what the target
      can spell. A target earns one when its lesson can be named before it is written.
- [x] The surface language: sequential scope and `:=`, `@import` and `@concat`, `$"…"` with Rust's
      specifier, projection by name and by computed key, and dyadic literals, argued by
      `docs/surface.md` and its three companions. Evaluating a canonical document is the identity.
- [x] The embedding surface: `Target`, `bind::from_path`, the files a reduction read, and
      `crates/nuke-cli`. Nuke ends at text and the host begins at files, which strikes the
      manifest — a name says where a file goes. `docs/embedding.md` argues the line.
- [ ] The editor tooling, which `dot` waits on. `nuke fmt` is done and so is the `tree-sitter`
      grammar Helix highlights from; the linter `docs/canonical-form.md` owes a style is not, nor
      is the LSP server.
- [ ] `ghostty` and `plist`, the two targets the roster misses, and then `dot` linking this
      workspace. The shells stay declined: a template engine writes what has no grammar.
