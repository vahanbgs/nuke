use std::fmt;

use nuke_syntax::{MAX_DEPTH, Value};

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
pub struct Error<K> {
    kind: K,
    path: Path,
}

impl<K> Error<K> {
    pub(crate) fn new(kind: K, path: Path) -> Self {
        Self { kind, path }
    }

    pub const fn kind(&self) -> &K {
        &self.kind
    }

    pub const fn path(&self) -> &Path {
        &self.path
    }
}

impl<K: fmt::Display> fmt::Display for Error<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, at {}", self.kind, self.path)
    }
}

impl<K: fmt::Debug + fmt::Display> std::error::Error for Error<K> {}

pub(crate) fn too_deep(f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "this value nests deeper than {MAX_DEPTH} levels")
}

pub(crate) fn form(value: &Value) -> &'static str {
    match value {
        Value::Tuple(_) => "tuple",
        Value::Map(_) => "map",
        Value::List(_) => "list",
        Value::Atom(_) => "atom",
        Value::String(_) => "string",
        Value::Integer(_) => "integer",
        Value::Float(_) => "float",
    }
}
