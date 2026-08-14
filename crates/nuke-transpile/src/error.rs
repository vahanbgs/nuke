use std::fmt;

use nuke_syntax::MAX_DEPTH;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Field(String),
    Index(usize),
    Entry(usize),
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Field(name) => write!(f, ".{name}"),
            Self::Index(index) => write!(f, "[{index}]"),
            Self::Entry(position) => write!(f, "#{}", position + 1),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Path(Vec<Segment>);

impl Path {
    pub(crate) fn new(segments: Vec<Segment>) -> Self {
        Self(segments)
    }

    pub fn segments(&self) -> &[Segment] {
        &self.0
    }

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some((first, rest)) = self.0.split_first() else {
            return f.write_str("the document");
        };
        match first {
            Segment::Field(name) => f.write_str(name)?,
            other => write!(f, "{other}")?,
        }
        rest.iter().try_for_each(|segment| write!(f, "{segment}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    UnrepresentableKey(&'static str),
    DuplicateKey(String),
    TooDeep,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnrepresentableKey(form) => write!(
                f,
                "a {form} cannot key a JSON object; only a string or an atom names one"
            ),
            Self::DuplicateKey(name) => {
                write!(f, "two keys of this map both name `{name}` in JSON")
            }
            Self::TooDeep => write!(f, "this value nests deeper than {MAX_DEPTH} levels"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    path: Path,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind, path: Path) -> Self {
        Self { kind, path }
    }

    pub const fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub const fn path(&self) -> &Path {
        &self.path
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, at {}", self.kind, self.path)
    }
}

impl std::error::Error for Error {}
