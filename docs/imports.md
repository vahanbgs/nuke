# Imports

`@import "./palette.nuke"` is another document's reduced value. `@` calls a builtin and `import`
is the first of them; `docs/surface.md` argues the operator and this argues the builtin — the
first part of Nuke about **files** rather than text, and so a resolution rule, an identity for a
file and a cycle rule, none of which an ABNF can state. A search path, a registry and remote
imports are all deferred: each answers how to find a file you did not write, which is
distribution, and by-name imports will be a builtin of their own rather than an overload of a
path — which is what spending a sigil rather than a word bought.

## The path is a literal

`@import` takes a string **literal**, never an expression, so `@import n` is refused even where
`n` is bound to a path. What a file imports is then a property of its text, so the formatter, the
linter, the LSP server and a watcher read the dependency graph by walking rather than evaluating.
Nix takes an expression, which is why nothing there can list a file's imports without running it. The grammar cannot say this, since `call` takes any operand, so
the evaluator checks the operand's shape before reducing it. What is given up is a path chosen
while reducing, which conditionals will make expressible by choosing between imported *values*.

## Resolution

A path resolves against **the directory of the file that spells it**, and an import inside an
imported file resolves against *that* file. Never the process's working directory: it is
process-global mutable state, so a document's meaning would follow the caller's shell. A document
with no file of its own resolves no relative import, and says so rather than guessing.

An absolute path is used as written, and `..` is an ordinary component with no ceiling, because a
dot-file tree *is* a shared file a level up and a sandbox belongs to whatever runs Nuke. `~` is a
directory with an odd name, expansion being a shell's job. A directory cannot be read, and there
is no `index.nuke`, which would be a second spelling of one import. The `.nuke` extension is
neither appended nor required — asking for it is the linter's. And how a read failed is the
operating system's report rather than a fault of its own: the rule is only that an import names a
file that can be read.

## What an import denotes

Exactly the imported document's reduced value. No module, no namespace object, no seventh form of
value: `Value` **is** the canonical form, ten backends consume it, and anything else would have
no spelling in `grammar/canonical.abnf` and no way to survive the law that evaluating a canonical
document is the identity. So an imported file's **bindings are private and its fields are its
surface**, which is what `docs/surface.md` meant by a block of bindings being empty data. Nothing
is exported and nothing is marked: a file publishing three things ends in a tuple of three. And
privacy runs both ways without a check anywhere, because each file is reduced in a scope of its
own.

```nuke
# palette.nuke — `base` is private; the three fields are the surface.
base := "#282828"
{bg = base fg = "#EBDBB2" accent = "#FE8019"}

# editor.nuke
palette := @import "./palette.nuke"
{theme = {background = palette.bg cursor = palette.accent} line_numbers = Relative}
```

The binding is a convenience — a call is an operand, so `@import "./palette.nuke".accent`
projects straight out of the imported document — and `#include` and Nix's `with` are refused,
because a name visible with no `:=` above it destroys what the totality argument rests on.

## A cycle across files is detected

A file is identified by its **canonicalised path**, so `"p.nuke"`, `"./p.nuke"` and a symlink to
it are one file, for the cycle stack and the cache alike — one question asked twice, and two hard
links are what it misses, costing nothing since a hard link's text *is* the same text. A stack
holds the files whose reduction is in progress, the entry file first, and an import resolving
onto it is refused there, before that file is opened.

`docs/surface.md` boasts that a reference cycle needs no detector, and that argument does not
survive a filesystem. Sequential scope works because a text has a top, so "above" is a well-order
the author writes down. A directory has no top, and every ordering one could invent between two
files — alphabetical, modification time, "may import only from a parent" — is a rule about where
a user may keep a file. A cross-file cycle cannot even be *read*: nothing in `a.nuke` says
whether `b.nuke` imports it, so the fault is made by editing a file nobody is looking at.
Impossibility by construction is unavailable here, not merely expensive. What replaces it is not
the machinery that argument refuses — thunks, fixpoints and a detector over *values* — but a
stack of paths, with nothing deferred and no partially built value anywhere. **A name has one
above it; a file has nothing above it**, which is why `n := n` is legal and names the one above
while a file importing itself is not.

A diamond is not a cycle: what must be acyclic is what is being reduced *while*, not what is
reached from, and two dot files sharing one palette is the feature. Importing one file twice is
reuse for the same reason — a repeated *use*, like reading a bound name twice, not the repeated
declaration a field name and a map key refuse. The rule is on files and not fields, so `a`
importing `b.x` while `b` imports `a.y` is still a cycle.

## What a file costs

A file is read once per reduction and cached by its canonical path, and that is not first an
optimisation: without it, a file edited between two reads inside one reduction would let one
document mean two things, so the cache is what makes one path denote one value. What it charges
is what a binding charges — built once, then its measured size and depth at **every** site that
names it, including the first. So a file costs building it once plus its size wherever it appears,
word for word the rule for a bound name, at the price of an effective ceiling of half `MAX_VALUES`
for one imported document.

`MAX_VALUES` is one budget for the whole document, files and all, and `MAX_IMPORTS` bounds the
chain — because a cycle bounds a loop and not a chain, and sixty thousand one-line files each
importing the next would overflow the stack while costing almost nothing against the budget.

