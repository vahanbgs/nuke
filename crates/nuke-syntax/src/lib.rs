#[cfg(feature = "serde")]
pub mod de;
pub mod error;
pub mod lexer;
mod parser;
pub mod value;

#[cfg(feature = "serde")]
pub use de::{from_str, from_value};
pub use error::{Error, ErrorKind, Location, Span};
pub use lexer::{Lexer, Token, TokenKind};
pub use value::{Atom, Float, Ident, Integer, Map, Tuple, Value};

pub const MAX_DEPTH: usize = 128;

pub fn parse(source: &str) -> Result<Value, Error> {
    parser::parse(source)
}
