# The server

`nuke lsp` speaks LSP over stdio and `crates/nuke-lsp` is the library it is one call into. It is
the fourth head of the editor tooling: `docs/formatting.md` decides how a document reads,
`docs/linting.md` what the formatter may not fix, `docs/highlighting.md` what colours it, and
this decides what an editor is *told*.

## Three faults, and they keep their owners

A document can be wrong in three unrelated ways, and folding them together would lose the
distinction each tool spent its length drawing.

- A **parse error** is the parser's, and it is published alone: `docs/linting.md` argued that a
  document which did not parse has exactly one fault, so nothing else runs to bury it.
- A **lint** is a style fault, published as a `Warning` carrying its rule name in the
  diagnostic's `code` — the name an editor filters and a person searches for.
- A **reduction fault** — an unbound name, a missing import, a cycle — is the reducer's, and
  it is an `Error`.

The third is new here rather than moved: `nuke lint` cannot report it, because a lint is defined
by the formatter and a reducer needs a filesystem the linter refuses to have.

## The server is a host, so it may read

`docs/embedding.md` puts a filesystem on the host's side of the line, and a server is a host. It
reduces the **buffer's** text against the buffer's path — the pair `bind::from_source` already
takes — so what is being edited is what the editor has while what it imports is what the disk
has. `reduce_at_with_files` takes the tree already parsed, because `eval_at` would parse the
same text a second time on every keystroke.

A buffer with no file reduces without an origin and so resolves no relative import, saying so
rather than guessing — `docs/imports.md`'s rule met from the editor's side.

There is no debounce: `MAX_BYTES` and `MAX_IMPORTS` bound a keystroke's work at a megabyte and
64 files, and a dot file is nowhere near either.

## An imported fault is reported where it was asked for

`ErrorKind::Import` nests, and the **outer** error's span is the `@import` that asked — in the
file being edited. So that is where the diagnostic goes, and it is the honest place: it is the
file whose author can act, `Display` already spells the chain, and `docs/linting.md`'s
objection to reporting a fault in a file someone may not own is answered rather than ignored.
`relatedInformation` then points at the innermost fault in the file that actually holds it, so
an editor can jump there without the diagnostic pretending to live there.

## What a save re-diagnoses

A reduction reports the files it read, which `docs/embedding.md` promised a watcher, and this is
the watcher: on save, every open document whose list holds the saved path is re-diagnosed. A
document whose reduction *failed* has no reliable list — the read stopped at the fault — so it is
re-diagnosed on any save, which lets a broken import clear itself when the file is fixed.

## A name is followed, and only inside one file

Definition, references and document-highlight are one lookup and one filter over
`nuke_resolve::Resolution`, the same table `unused-binding` reads. Nothing is resolved twice
and nothing can disagree: the linter and the editor point at the same binding for the same
word, which is the whole reason the scope pass left `nuke-lint`.

What it refuses is a name **across** a file. `theme.colour` where `theme` came from `@import` is
a projection off a value, and a value is reduction's — following it means indexing the graph and
keeping that index fresh, which is a different program from a resolver that walks one tree.
Whoever needs it should build the index rather than teach the resolver to open files.

Rename is absent for the reason it is tempting: a field name survives into JSON and an atom
**is** a value, so respelling one edits the document's meaning rather than its text.

## Positions are counted the client's way

A span is bytes and LSP positions are not, so `line-index` converts: UTF-16 code units, which the
protocol defaults to, or UTF-8 where the client offers it in `positionEncodings`. A colour is the
first thing anyone writes in a dot file and an emoji is the second, so counting characters
instead of units would be wrong on the lines that matter most.

The outline is the one response this crate spells itself: `lsp-types` cannot build a
`DocumentSymbol` without initialising a field it deprecates, and this workspace denies warnings
and forbids silencing them, so a local `Serialize` struct carries the protocol's field names.

## Helix

```toml
[language-server.nuke]
command = "nuke"
args = ["lsp"]

[[language]]
name = "nuke"
scope = "source.nuke"
file-types = ["nuke"]
comment-token = "#"
indent = { tab-width = 8, unit = "\t" }
language-servers = ["nuke"]

[[grammar]]
name = "nuke"
source = { path = "/path/to/nuke/tree-sitter-nuke" }
```

`formatter` is not needed — the server formats — and `nuke fmt -` stays for what is not an editor.
