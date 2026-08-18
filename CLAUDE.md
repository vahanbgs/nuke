# Nuke

Nuke is a simple total configuration language. This repository holds its specification, its
grammar in ABNF, and implementations in Rust of the tools used to work with it.

## A few facts about Nuke

- Nuke is whitespace insensitive, except inside a comment, a string, and a format specifier.
- Nuke configuration files are Nuke expressions which are evaluated and reduced down to a
  core canonical form, which is to Nuke what JSON is to Javascript.
- Its first host is a dot file manager: every dot file in one language, sharing values by import.

## Layout

- `grammar/` — `tokens.abnf` then `canonical.abnf` or `surface.abnf`; a grammar is that pair.
- `docs/canonical-form.md`, `docs/surface.md` — the rules ABNF cannot express, one per language,
  plus `docs/imports.md` for files, `docs/interpolation.md` for text, `docs/dyadic.md` for bases.
- `docs/serde.md` — where Nuke and serde disagree; `docs/embedding.md` — where the language stops
  and a host begins; then the editor's four: `docs/formatting.md` what the formatter decides,
  `docs/linting.md` what it may not fix, `docs/highlighting.md` what a third grammar may disagree
  with the other two on, `docs/lsp.md` what an editor is told.
- `docs/json.md` and its ten siblings — each mapping, and what it degrades or refuses.
- `fixtures/valid`, `fixtures/invalid` — conformance fixtures. Under `fixtures/surface`, `valid`
  pairs with `reduced` by name, `invalid` is what the parser refuses, `refused` what cannot, and
  `modules` holds what those import, which are inputs rather than fixtures.
- `crates/nuke-fixtures` — reads those trees, so no crate keeps its own copy of the walker.
- `crates/nuke-grammar` — assembles a `Layer`'s ABNF, translates it to pest at test time and
  checks the fixtures against it, so the grammar is executable and cannot drift from the code.
- `crates/nuke-syntax` — one hand-written lexer, whose mode stack is what interpolation needs, and
  two parsers over it: `parse` for the canonical form, `surface::parse` for the surface language,
  each carrying what ABNF cannot state. `serde` adds `from_str`/`to_value`, and `printer` puts a
  document back to text, taking every literal's spelling from its span, never from the tree.
- `crates/nuke-eval` — reduces a `Document` to a `Value`; owns the filesystem (resolution, the
  import cache, the cycle check) and `text.rs`, where a value becomes text under a specifier.
  `bind` reads a Rust type out of one, and a reduction reports the files it read.
- `crates/nuke-transpile` — the backends, each owning its `ErrorKind` and its answer to what the
  target spells: JSON, YAML, TOML, XML, KDL, Lua, INI, gitconfig, Nix, Ghostty and plist, argued
  one per `docs/` file. `Target` names one and `Refusal` wraps the eleven without flattening them.
- `crates/nuke-resolve` — sequential scope kept rather than spent: which binding each name reads,
  and which names nothing read. No filesystem, and the linter and the server share it.
- `crates/nuke-lint` — the style `docs/canonical-form.md` owes: what the formatter may not fix.
  No filesystem, so it follows no import, and it asks `nuke-resolve` for `unused-binding`.
- `crates/nuke-lsp` — the server, over `lsp-server`: the parser's, the linter's and the reducer's
  faults kept apart, navigation from `nuke-resolve`, and the reduction only a host may run.
- `crates/nuke-cli` — the binary `nuke`: `render` writes a document out, `deps` lists what it read,
  `fmt` and `lint` take a path or `-`, and `lsp` serves an editor. None writes a file.
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

- [x] The canonical form and the eleven backends over it, a target earning one when its lesson can
      be named first; then the surface language, then the embedding surface where Nuke ends at
      text; then the editor tooling `dot` waited on, `fmt`, `tree-sitter`, `lint` and `lsp`.
- [x] `dot` linking this workspace, at `0.1.0`. A `.nuke` file in its tree renders on deploy, the
      name giving destination and target both; a name giving no target is a module, imported and
      never deployed; `targets` in its manifest names what `.gitconfig` and `config` cannot.
- [ ] The real tree moved over, every dot file with a grammar becoming a document. Only using it
      can say whether `@merge`, functions and iteration have met their triggers. `dot` reads its
      own manifest through `bind::from_path` now, so what is left is the tree it describes.
