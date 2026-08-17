use std::fmt;
use std::io;
use std::path::PathBuf;

use nuke_syntax::{Location, MAX_DEPTH, Span};

use crate::{MAX_IMPORTS, MAX_VALUES};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Syntax(nuke_syntax::Error),
    Unbound(String),
    NotATuple,
    NoSuchField(String),
    DuplicateKey,
    TooDeep,
    TooLarge,
    NoSuchBuiltin(String),
    ExpectedImportPath,
    NoOrigin,
    Unreadable {
        path: String,
        cause: io::ErrorKind,
    },
    ImportCycle(PathBuf),
    ImportsTooDeep,
    Import {
        path: PathBuf,
        at: Location,
        cause: Box<Error>,
    },
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(error) => write!(f, "{error}"),
            Self::Unbound(name) => write!(
                f,
                "`{name}` is not bound here; a name is visible below its own binding, and \
                 only inside the block that makes it"
            ),
            Self::NotATuple => write!(
                f,
                "only a tuple has fields; a map has entries keyed by values and a list has \
                 positions"
            ),
            Self::NoSuchField(name) => write!(f, "this tuple has no field `{name}`"),
            Self::DuplicateKey => write!(f, "this key is already in this map"),
            Self::TooDeep => write!(f, "this value nests deeper than {MAX_DEPTH} levels"),
            Self::TooLarge => write!(f, "this document expands past {MAX_VALUES} values"),
            Self::NoSuchBuiltin(name) => write!(
                f,
                "there is no builtin `{name}`; `@` names one, and the name is not a binding"
            ),
            Self::ExpectedImportPath => write!(
                f,
                "`@import` takes a string literal, so that what a file imports is a property \
                 of its text"
            ),
            Self::NoOrigin => write!(
                f,
                "an import resolves against the file that spells it, and this document has no \
                 file of its own"
            ),
            Self::Unreadable { path, cause } => {
                write!(f, "`{path}` cannot be read: {cause}")
            }
            Self::ImportCycle(path) => write!(
                f,
                "`{}` is already being imported, and a file cannot be part of its own value",
                path.display()
            ),
            Self::ImportsTooDeep => {
                write!(f, "an import chain reaches deeper than {MAX_IMPORTS} files")
            }
            Self::Import { path, at, cause } => {
                write!(f, "importing `{}` failed at {at}: {cause}", path.display())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    span: Span,
}

impl Error {
    pub fn new(kind: ErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub const fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub const fn span(&self) -> Span {
        self.span
    }

    pub fn location(&self, source: &str) -> Location {
        self.span.location(source)
    }
}

impl From<nuke_syntax::Error> for Error {
    fn from(error: nuke_syntax::Error) -> Self {
        let span = error.span();
        Self::new(ErrorKind::Syntax(error), span)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for Error {}
