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
- `docs/serde.md` — where Nuke and serde's data model disagree, and how the binding settles it.
- `docs/json.md` and its eight siblings — each mapping, and what it degrades or refuses.
- `fixtures/valid`, `fixtures/invalid` — conformance fixtures. Under `fixtures/surface`, `valid`
  pairs with `reduced` by name, `invalid` is what the parser refuses, `refused` what cannot, and
  `modules` holds the files those fixtures import, which are inputs rather than fixtures.
- `crates/nuke-fixtures` — reads those trees, so no crate keeps its own copy of the walker.
- `crates/nuke-grammar` — assembles a `Layer`'s ABNF, translates it to pest at test time and
  checks the fixtures against it, so the grammar is executable and cannot drift from the code.
- `crates/nuke-syntax` — one hand-written lexer, whose mode stack is what interpolation needs, and
  two parsers over it: `parse` for the canonical form, `surface::parse` for the surface language.
  Each carries what ABNF cannot state. `serde` adds `from_str`/`to_value`.
- `crates/nuke-eval` — reduces a `Document` to a `Value`; owns the filesystem (resolution, the
  import cache, the cycle check) and `text.rs`, where a value becomes text under a specifier.
- `crates/nuke-transpile` — the backends, each owning its own `ErrorKind` and its own answer to what
  the target can spell: JSON, YAML, TOML, XML, KDL, Lua, INI, gitconfig, Nix. `docs/` argues each.

Work in the devshell: `nix develop -c cargo test --workspace --all-features` covers serde too.

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

- [x] Design the grammar of Nuke's canonical form in ABNF.
- [x] A hand-written lexer and parser for the canonical form, checked against the grammar itself
  and against the same fixtures.
- [x] Serde support: `Value` as a self-describing data model, and `from_str` into a user's type.
- [x] The transpiler: nine backends, each owning its `ErrorKind` and its answer to what the target
      can spell. A target earns one when its lesson can be named before it is written.
- [x] The surface language: the expressions that reduce to the canonical form.
  - [x] Names. `:=` binds and contributes nothing; scope is sequential, so a cycle cannot be
        written; a field is not a binding. The invariant holding the surface language to the
        canonical form is that evaluating a canonical document is the identity.
  - [x] Imports. `@` calls a builtin and `import` is the first; its path is a literal, so what a
        file imports is a property of its text. A file is its canonical path, its bindings are
        private, and a cycle needs a detector because a directory has no top.
  - [x] `@concat`, the second builtin and the first about values rather than files, which makes
        `@` a namespace. It does not stringify, and a string wants `MAX_BYTES` of its own.
  - [x] Interpolation. `$"a{expr:spec}b"` with Rust's specifier, and a hole is the one place a
        value becomes text, so no `@text` was spent — lambdas are owed most builtins and no syntax.
  - [x] Projection `.`. Postfix, whitespace insensitive, its operand any value, so `1.b` stays a
        malformed number. Two right operands: a name reads a tuple's field, `(expr)` the key a map
        or a list is read at — `5a7fa5a`'s `[ ]` overturned, its case against `m["a"]` kept.
  - [x] Dyadic literals, the last of the surface language. A marker per width — `b`, `q`, `o`, `x` —
        so one number mixes bases. Uppercase hex frees the markers; `{n:06X}` shares the ceiling.
- [ ] The dot file manager — a CLI, a manifest, and the targets the roster misses. This, and not
      the language, is what the dot files are waiting on.
- [ ] The formatter, the linter, the LSP server.
