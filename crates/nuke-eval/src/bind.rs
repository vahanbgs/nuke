use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use nuke_syntax::Location;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Unreadable(io::ErrorKind),
    Reduction {
        at: Location,
        cause: Box<crate::Error>,
    },
    Binding(nuke_syntax::de::Error),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(cause) => write!(f, "cannot be read: {cause}"),
            Self::Reduction { at, cause } => write!(f, "{at}: {cause}"),
            Self::Binding(cause) => write!(f, "{cause}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    path: PathBuf,
    kind: ErrorKind,
}

impl Error {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub const fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` {}", self.path.display(), self.kind)
    }
}

impl std::error::Error for Error {}

pub fn from_path<T: DeserializeOwned>(path: &Path) -> Result<T, Error> {
    let source = fs::read_to_string(path).map_err(|cause| Error {
        path: path.to_path_buf(),
        kind: ErrorKind::Unreadable(cause.kind()),
    })?;
    from_source(&source, path)
}

pub fn from_source<T: DeserializeOwned>(source: &str, path: &Path) -> Result<T, Error> {
    let value = crate::eval_at(source, path).map_err(|cause| Error {
        path: path.to_path_buf(),
        kind: ErrorKind::Reduction {
            at: cause.location(source),
            cause: Box::new(cause),
        },
    })?;
    nuke_syntax::from_value(&value).map_err(|cause| Error {
        path: path.to_path_buf(),
        kind: ErrorKind::Binding(cause),
    })
}
