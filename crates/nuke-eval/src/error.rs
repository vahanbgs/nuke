use std::fmt;
use std::io;
use std::path::PathBuf;

use nuke_syntax::{Location, MAX_DEPTH, Span};

use crate::{MAX_BYTES, MAX_IMPORTS, MAX_VALUES};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Syntax(nuke_syntax::Error),
    Unbound(String),
    NotATuple,
    NoSuchField(String),
    NotKeyed,
    NoSuchKey,
    DuplicateKey,
    TooDeep,
    TooLarge,
    TooLong,
    NoSuchBuiltin(String),
    NotAList,
    NotAString,
    NoText,
    NeedsPrecision,
    UnfitSpec,
    TooWide,
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
                 positions, and `.(key)` is what reads either"
            ),
            Self::NoSuchField(name) => write!(f, "this tuple has no field `{name}`"),
            Self::NotKeyed => write!(
                f,
                "only a map and a list are read by a key; a tuple's fields are named, so \
                 `.name` reads one"
            ),
            Self::NoSuchKey => write!(
                f,
                "nothing is at this key; a map's keys are the ones its entries spell and a \
                 list's are its positions"
            ),
            Self::DuplicateKey => write!(f, "this key is already in this map"),
            Self::TooDeep => write!(f, "this value nests deeper than {MAX_DEPTH} levels"),
            Self::TooLarge => write!(f, "this document expands past {MAX_VALUES} values"),
            Self::TooLong => write!(f, "this string grows past {MAX_BYTES} bytes"),
            Self::NoSuchBuiltin(name) => write!(
                f,
                "there is no builtin `{name}`; `@` names one, and the name is not a binding"
            ),
            Self::NotAList => write!(
                f,
                "`@concat` takes a list, because a call takes one operand and a builtin \
                 wanting more takes a collection"
            ),
            Self::NotAString => write!(
                f,
                "`@concat` puts strings end to end; a number, an atom or a collection meant \
                 as text is written as text"
            ),
            Self::NoText => write!(
                f,
                "an atom is uninterpreted and a collection's text would be Nuke's own syntax, \
                 so neither is text a hole can hand over; a string and an integer are, and a \
                 float is once it is told a precision"
            ),
            Self::NeedsPrecision => write!(
                f,
                "a float has more than one canonical spelling, so a hole is told how many \
                 digits follow the point: `{{opacity:.2}}`"
            ),
            Self::UnfitSpec => write!(
                f,
                "this specifier asks for what this form has not got: a precision counts the \
                 digits after a point, a radix respells an integer that is not negative, \
                 Nuke's integers having no width for a two's complement to depend on, a sign \
                 belongs to a number, `#` to a radix, and `0` to a width it does not share \
                 with a fill or an alignment"
            ),
            Self::TooWide => write!(
                f,
                "this integer is past what another radix can be computed in; a decimal one is \
                 the text it was written as, and any other is arithmetic"
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
