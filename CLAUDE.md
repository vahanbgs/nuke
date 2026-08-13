# Nuke

Nuke is a simple total configuration language.
This repository contains its specification, its grammar in ABNF and implementations in Rust
of the various tools used to work with the language.

## A few facts about Nuke

- Nuke is whitespace insensitive (except for its single-line comment syntax of course).
- Nuke configuration files are Nuke expressions which are evaluated and reduced down to a
  core canonical form.
- This canonical form can be thought of as being what JSON is to Javascript.
- Nuke will transpile to all other mainstream configuration languages (JSON, YAML, TOML,
  XML, KDL, Lua, INI, cfg, gitconfig).
- Nuke will first be used in a dot file manager which will allow users to write all of
  their dot files using a single language.
- Nuke will support imports which will make it easy to share values and logic between dot
  files for different programs.
- Nuke files use the `.nuke` extension.

## Tools for working with Nuke

This repository will implement a few tools in Rust for working in Nuke:
- A formatter
- A linter
- An lsp server
- A transpiler
- A parser with serde support

## Layout

- `grammar/canonical.abnf` — the normative grammar of the canonical form.
- `docs/canonical-form.md` — the rules ABNF cannot express.
- `fixtures/valid`, `fixtures/invalid` — conformance fixtures, shared by every crate.
- `crates/nuke-fixtures` — reads that tree, so no crate keeps its own copy of the walker.
- `crates/nuke-grammar` — translates the ABNF to pest at test time and checks the fixtures
  against it, so the grammar file is executable and cannot drift from the implementation.
- `crates/nuke-syntax` — the hand-written lexer and parser. It carries the rules ABNF
  cannot state, and a test asserts it agrees with the grammar on every fixture.

Work inside the devshell: `nix develop -c cargo test --workspace`.

## Rules

- Never write comments in Rust code.
- Never use unsafe Rust code. It's okay to depend on crates that do if they are staples in
  the Rust community.
- Never hand-roll something if there are popular crates which are staples in the Rust
  community that do it better.
- All documents that only contain prose should stay under 100 lines long (including
  CLAUDE.md). Do not hesitate to remove old information that is obvious, useless,
  unimportant or outdated to replace it with new relevant, useful or important information.
- Use best practices for code maintainability, correctness, performance and compilation
  speed.
- Always write tests.
- Make compilation fail if clippy emits warnings.
- Never use directives to silence warnings.
- Always run cargo fmt/rustfmt.
- Use a Nix flake with a devshell to make our dev environment reproducible and to
  distribute our tooling. Never install tools system-wide and try not to rely on tools
  installed system-wide.
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

- [x] Design the grammar of Nuke's canonical form in ABNF. Differences with JSON: no
  separators in collection literals; arbitrary values as map keys; atoms; both map and
  named tuple literals; choice removed wherever possible; `#` comments.
- [x] A hand-written lexer and parser for the canonical form, checked against the same
  fixtures as the grammar and against the grammar itself.
- [ ] Serde support: `Value` as a self-describing data model, then a deserializer that
  goes from text straight to a user's type.
- [ ] A transpiler from the canonical form to JSON, then to the other targets.
- [ ] The surface language: the expressions that reduce to the canonical form, and imports.
- [ ] The formatter, the linter, the LSP server.
