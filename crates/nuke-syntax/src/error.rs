use std::fmt;

use crate::MAX_DEPTH;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn location(&self, source: &str) -> Location {
        let upto = source.get(..self.start).unwrap_or(source);
        Location {
            line: upto.matches('\n').count() + 1,
            column: upto
                .rsplit('\n')
                .next()
                .map_or(0, |last| last.chars().count())
                + 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    ByteOrderMark,
    CarriageReturn,
    ControlCharacter(char),
    UnexpectedCharacter(char),
    UnknownEscape(char),
    MalformedUnicodeEscape,
    SurrogateEscape(u32),
    ScalarOutOfRange(u32),
    UnterminatedString,
    UnterminatedHole,
    UnmatchedBrace,
    NumberSpelling(String),
    FloatOutOfRange,
    EmptyDocument,
    IdentAsValue(String),
    SurfaceBinder,
    SurfaceDot,
    SurfaceCall,
    SurfaceInterpolation,
    ExpectedValue,
    ExpectedHoleClose,
    ExpectedFieldName,
    ExpectedAccessName,
    ExpectedBuiltinName,
    ExpectedEquals,
    ExpectedArrow,
    ExpectedBindingName,
    MixedPairOperators,
    MisplacedBinding,
    OnlyBindings,
    UnterminatedList,
    UnterminatedBlock,
    DuplicateField(String),
    DuplicateBinding(String),
    DuplicateKey,
    TrailingInput,
    TooDeep,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ByteOrderMark => {
                write!(
                    f,
                    "a byte order mark is not allowed; files are UTF-8 without one"
                )
            }
            Self::CarriageReturn => {
                write!(
                    f,
                    "a carriage return is not allowed; canonical files are LF-only"
                )
            }
            Self::ControlCharacter(character) => write!(
                f,
                "the control character U+{:04X} must be escaped",
                *character as u32
            ),
            Self::UnexpectedCharacter(character) => {
                write!(f, "`{character}` cannot start a value")
            }
            Self::UnknownEscape(character) => write!(
                f,
                "`\\{character}` is not an escape; they are \\\" \\\\ \\n \\r \\t and \\u{{…}}"
            ),
            Self::MalformedUnicodeEscape => write!(
                f,
                "a unicode escape is `\\u{{…}}` around one to six uppercase hex digits"
            ),
            Self::SurrogateEscape(value) => {
                write!(f, "U+{value:04X} is a surrogate, not a scalar value")
            }
            Self::ScalarOutOfRange(value) => {
                write!(f, "U+{value:04X} is past the last scalar value, U+10FFFF")
            }
            Self::UnterminatedString => write!(f, "this string is never closed"),
            Self::UnterminatedHole => write!(f, "this hole is never closed"),
            Self::UnmatchedBrace => write!(
                f,
                "a lone `}}` is no brace inside `$\"…\"`; a literal one is written `}}}}`, \
                 as `{{` is written `{{{{`"
            ),
            Self::NumberSpelling(token) => {
                write!(f, "`{token}` is not how the canonical form spells a number")
            }
            Self::FloatOutOfRange => write!(
                f,
                "this float is past what a double holds, and there is no infinity"
            ),
            Self::EmptyDocument => write!(f, "a document is one value, and this one is empty"),
            Self::IdentAsValue(name) => write!(
                f,
                "`{name}` is an identifier, not a value; the canonical form names only a \
                 tuple's fields"
            ),
            Self::SurfaceBinder => write!(
                f,
                "`:=` binds a name in the surface language, and the canonical form carries \
                 data alone"
            ),
            Self::SurfaceDot => write!(
                f,
                "`.` projects a field in the surface language, and the canonical form carries \
                 data alone"
            ),
            Self::SurfaceCall => write!(
                f,
                "`@` calls a builtin in the surface language, and the canonical form carries \
                 data alone"
            ),
            Self::SurfaceInterpolation => write!(
                f,
                "`$\"…\"` interpolates in the surface language, and the canonical form carries \
                 data alone"
            ),
            Self::ExpectedValue => write!(f, "expected a value"),
            Self::ExpectedHoleClose => write!(
                f,
                "a hole holds one value, and `}}` closes it; a builtin wanting more takes a \
                 collection, and so does a hole"
            ),
            Self::ExpectedFieldName => write!(
                f,
                "expected a field name; this block is a tuple, because its first pair uses `=`"
            ),
            Self::ExpectedAccessName => {
                write!(
                    f,
                    "`.` projects a field, and a field is named by an identifier"
                )
            }
            Self::ExpectedBuiltinName => {
                write!(
                    f,
                    "`@` calls a builtin, and a builtin is named by an identifier"
                )
            }
            Self::ExpectedEquals => write!(f, "expected `=` after a field name"),
            Self::ExpectedArrow => write!(
                f,
                "expected `=>` after a map key; a pair bound with `=` is a tuple field, \
                 and a field is named by an identifier"
            ),
            Self::ExpectedBindingName => {
                write!(f, "`:=` binds a name, and a name is an identifier")
            }
            Self::MixedPairOperators => write!(
                f,
                "a block is a tuple or a map, and cannot hold both `=` and `=>`"
            ),
            Self::MisplacedBinding => write!(
                f,
                "a binding stands at the head of its block or of the document, before any \
                 pair, and a list holds none"
            ),
            Self::OnlyBindings => {
                write!(f, "a document is one value, and this one only binds names")
            }
            Self::UnterminatedList => write!(f, "this list is never closed"),
            Self::UnterminatedBlock => write!(f, "this block is never closed"),
            Self::DuplicateField(name) => {
                write!(f, "the field `{name}` is already in this tuple")
            }
            Self::DuplicateBinding(name) => {
                write!(f, "the name `{name}` is already bound in this block")
            }
            Self::DuplicateKey => write!(f, "this key is already in this map"),
            Self::TrailingInput => {
                write!(f, "a document is one value, and this one runs past its end")
            }
            Self::TooDeep => write!(f, "this expression nests deeper than {MAX_DEPTH} levels"),
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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for Error {}
