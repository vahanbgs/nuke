# The server

`nuke lsp` speaks LSP over stdio and `crates/nuke-lsp` is the library it is one call into. It is the
fourth head of the editor tooling: `docs/formatting.md` decides how a document reads,
`docs/linting.md` what the formatter may not fix, `docs/highlighting.md` what colours it, and this
decides what an editor is *told*.

## Three faults, and they keep their owners

A document can be wrong in three unrelated ways, and folding them would lose what each tool drew.

- A **parse error** is the parser's, published alone: a document which did not parse has exactly
  one fault, `docs/linting.md` argued, so nothing else runs to bury it.
- A **lint** is a `Warning` carrying its rule name in `code`, the name a person searches for.
- A **reduction fault** — an unbound name, a missing import, a cycle — is the reducer's `Error`.

The third is new rather than moved: `nuke lint` cannot report it, a lint being defined by the
formatter and a reducer needing a filesystem the linter refuses to have.

## The server is a host, so it may read

`docs/embedding.md` puts a filesystem on the host's side of the line, and a server is a host. It
reduces the **buffer's** text against the buffer's path — the pair `bind::from_source` takes — so
what is edited is what the editor has while what it imports is what the disk has, and
`reduce_at_with_files` takes the tree already parsed, `eval_at` parsing it twice per keystroke. A
buffer with no file resolves no relative import and says so, `docs/imports.md`'s rule met from the
editor's side, and there is no debounce: `MAX_BYTES` and `MAX_IMPORTS` bound a keystroke.

## An imported fault is reported where it was asked for

`ErrorKind::Import` nests, and the **outer** error's span is the `@import` that asked, in the file
being edited. That is the honest place: its author can act, `Display` spells the chain, and
`docs/linting.md`'s objection to faulting a file someone may not own is answered rather than
ignored. `relatedInformation` points at the innermost fault, so an editor can jump there anyway.

## What a save re-diagnoses

A reduction reports the files it read, the watcher `docs/embedding.md` promised: on save, every
open document whose list holds the saved path is re-diagnosed. One whose reduction *failed* has no
reliable list, so it is re-diagnosed on any save, which lets a broken import clear itself.

## A name is followed, and only inside one file

Definition, references and document-highlight are one lookup and one filter over
`nuke_resolve::Resolution`, the same table `unused-binding` reads, so the linter and the editor
cannot point at different bindings for one word — which is why the scope pass left `nuke-lint`.
They arrive in source order, which costs the resolver one `Direction`: `x |> f` stores the function
first and spells it last.

What it refuses is a name **across** a file. `theme.colour` where `theme` came from `@import` is a
projection off a value, and a value is reduction's — following it means indexing the graph and
keeping that index fresh, a different program from a resolver that walks one tree. Rename is absent
for the reason it is tempting: a field name survives into JSON and an atom **is** a value, so
respelling one edits the document's meaning rather than its text.

## What the outline names

`documentSymbol` walks the tree instead. It descends where a value keeps the name's path — a tuple's
bindings and fields, a map's bindings, and **through a group**, which is a shape and not a step. It
stops elsewhere: the argument of `@concat <| xs` stands at no field's path. A kind is the value's
shape where the syntax knows it, so an application joins access and index as a `Field`; `Function`
is for what **is** one, `@name` now and a lambda when they land.

## The namespace is offered, and only that much

`@` names a namespace rather than an import, so `nuke_eval::Builtin` keeps the roster as data, and
the server offers it after the sigil and describes it on hover. That much is lexical, so `{a = @}`
completes although it does not parse: a token stream survives what a tree cannot. Anywhere else,
completion is what `Resolution::visible` says stands at the cursor — the table the three lookups
above already read, so a lambda's parameter arrives with no line written here. That half wants a
tree, so a buffer which does not parse offers nothing, the rule that gives it one diagnostic.

## Positions are counted the client's way

A span is bytes and LSP positions are not, so `line-index` converts: UTF-16 code units, which the
protocol defaults to, or UTF-8 where the client offers it. A colour is the first thing anyone
writes in a dot file and an emoji the second, so counting characters would be wrong where it tells.

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
auto-format = true
```

`formatter` is not needed — the server formats — and `nuke fmt -` stays for what is not an editor.
That block is half of it: Helix finds a grammar and its queries by **language name** under a runtime
directory, never the grammar's own repository, so `runtime/grammars/nuke.so` and
`runtime/queries/nuke/` are the rest: `programs.nuke.helix.enable`, from `homeModules.default`.
