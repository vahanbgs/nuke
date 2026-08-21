mod complete;
mod diagnostics;
mod document;
mod navigate;
mod position;
mod server;

use std::fmt;
use std::io;

use lsp_server::{Connection, ProtocolError};

pub use position::Encoding;

#[derive(Debug)]
pub enum Error {
    Protocol(ProtocolError),
    Json(serde_json::Error),
    Io(io::Error),
    Disconnected,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "a message is not what the protocol says: {error}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::Disconnected => write!(f, "the editor closed the connection"),
        }
    }
}

impl std::error::Error for Error {}

impl From<ProtocolError> for Error {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn serve() -> Result<(), Error> {
    let (connection, threads) = Connection::stdio();
    let served = serve_on(&connection);
    drop(connection);
    threads.join()?;
    served
}

pub fn serve_on(connection: &Connection) -> Result<(), Error> {
    server::serve(connection)
}
