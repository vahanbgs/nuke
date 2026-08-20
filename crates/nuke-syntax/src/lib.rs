mod cursor;
#[cfg(feature = "serde")]
pub mod de;
pub mod error;
pub mod expr;
#[cfg(feature = "serde")]
mod hint;
pub mod lexer;
mod parser;
pub mod printer;
#[cfg(feature = "serde")]
pub mod ser;
pub mod spec;
pub mod surface;
pub mod value;

#[cfg(feature = "serde")]
pub use de::{from_str, from_value};
pub use error::{Error, ErrorKind, Location, Span};
pub use expr::{Binding, Document, Entry, Expr, ExprKind, Field, Form, Name, Piece};
pub use lexer::{Lexer, Token, TokenKind};
#[cfg(feature = "serde")]
pub use ser::to_value;
pub use spec::{Align, Notation, Spec};
pub use value::{Atom, Float, Ident, Integer, Map, Tuple, Value};

pub const MAX_DEPTH: usize = 128;

pub fn parse(source: &str) -> Result<Value, Error> {
    parser::parse(source)
}
