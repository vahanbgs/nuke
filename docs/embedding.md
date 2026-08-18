# Embedding

`nuke_eval::eval_at` turns a file into a `Value`, and `Target::render` turns that `Value` into a
target's text. A program that wants Nuke needs those two calls and no others. This document
records where the language's job stops and the host program's begins — a line neither an ABNF nor
a backend can draw, because it is not about what can be spelled but about who decides.

Nuke's first host is a dot file manager that already existed, and its second is any program that
reads its own configuration. They are one customer: the first ends at `render`, the second at
`bind::from_path`, and both begin by anchoring a document to a path.

## Nuke ends at text

| the language                             | the host                                    |
| ---------------------------------------- | ------------------------------------------- |
| a path and a text become a `Value`        | which file, and when it is read again        |
| a `Value` becomes a target's text         | where that text goes and under what name     |
| a `Value` becomes a Rust type             | what that type is for                        |
| a reduction reports the files it read     | what to do when one of them changes          |

There is no `write`, no `link` and no `$HOME`. A value is the same value wherever it lands, so a
destination, an extension, an atomic rename, a backup and a permission bit are decisions about a
filesystem rather than about a document, and a language that made them would be a manager wearing
a grammar. The extension was the one of these ever in doubt, and `c425c2d` gave it to the manager
before there was a manager to give it to.

The line also settles which formats Nuke will never write. A backend is defined by what it
refuses, so a Turing-complete target refuses nothing and a backend into one has decided nothing —
which is why the shells have none and will not get one. That is not a hole in the roster. A host
that writes dot files already has a template engine for the files that have no grammar, and the
division is exactly the one this line draws: **Nuke writes what a grammar describes, and the host
writes what only a program can.**

## A target has a name

`Target` is an enum over the nine backends, with `FromStr`, `Display`, `ALL`, `extensions` and
`from_extension`, and `render` calls the one you named. It lives in `nuke-transpile` and not in a
command line of ours, because the caller is not ours: a choice duplicated in every host is a
choice the library never made.

Dispatch needs one error type, and `Refusal` is an enum of the nine rather than a flattening of
them. Each variant carries that backend's own `Error<K>` whole, so the vocabularies the nine
documents spent their length arguing survive contact with a host that does not care which target
it is holding; `target()` and `path()` answer the two questions such a host does ask.

`Target` names targets and not options. `json::to_string_compact` and `xml::to_string_rooted` stay
functions, because a struct carrying two knobs that seven targets ignore is the room that is not a
distinction `docs/kdl.md` refuses — and a host writing into someone else's schema, which is what
the XML root is for, is calling a specific backend on purpose by then.

## An extension is a hint and never a promise

`from_extension` answers an `Option`, and the shape is the honest one rather than a defect: `yaml`
answers to two extensions, and `gitconfig` to none, because `~/.gitconfig` and Ghostty's `config`
are named rather than extended. Where a file's name says nothing, what fills the gap is the host's
table and not a guess of ours — for the same reason the extension itself was the host's.

## A file becomes a Rust type in one call

`nuke_syntax::from_str` reads the **canonical** form, so a document carrying a binding or an
import — that is, a document anyone would actually write — cannot be deserialized through it.
`bind::from_path` and `bind::from_source` are the surface language's counterpart, and they live in
`nuke-eval` because that crate already owns the step where a file becomes a value; `nuke-syntax`
could not host them without taking the filesystem, which is the split the whole layout rests on.

`from_source` takes a path beside its text, and that path is not where the text came from: it is
what the text's relative imports resolve against, the distinction `eval_at` already draws against
`eval`. The error names the file and carries a `Location` rather than a `Span`, because a caller
who handed over a path never saw the source — which is what `ErrorKind::Import` had already
discovered at a file boundary. A value the target type cannot hold is a **binding** fault and not
a reduction one, so a host can tell a document that is wrong from one that is merely not what this
program wanted.

## A reduction says what it read

`eval_at_with_files` returns the value and the files it was built from, the entry file first and
each import after it in the order it was first read. The list was already being kept: a file is
cached by its canonical path because that is what makes one path denote one value, so the cache's
keys **are** the set and asking for it costs a `Vec`. A diamond names the file it shares once, for
the same reason.

That is what a watcher needs and what `docs/imports.md` promised it. It is not what a linter
needs: reading the dependency graph *without* a filesystem is a walk over `ExprKind::Call`, whose
operand is a literal precisely so that walk is possible, and it waits for the tool that wants it.

## The command line is the reference consumer

`nuke render -f <target> FILE` and `nuke deps FILE`, and nothing else. `--format` is optional
because a name can say it — `config.toml.nuke` strips its `.nuke` and reads what is underneath,
which is the host convention met from the other side — and a name that says nothing asks rather
than guessing. The binary exists to be the worked example, to debug a document by hand, and to be
what a host that is not Rust gets. It is not the manager, and it grows no verb that writes a file.
