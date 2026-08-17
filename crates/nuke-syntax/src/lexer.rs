use crate::error::{Error, ErrorKind, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    BraceOpen,
    BraceClose,
    BracketOpen,
    BracketClose,
    Equals,
    Arrow,
    Binder,
    Atom,
    Ident,
    String,
    Number,
    Whitespace,
    Comment,
}

impl TokenKind {
    pub const fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::Comment)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub span: Span,
    pub text: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumberKind {
    Integer,
    Float,
}

pub struct Lexer<'a> {
    source: &'a str,
    offset: usize,
}

impl<'a> Lexer<'a> {
    pub const fn new(source: &'a str) -> Self {
        Self { source, offset: 0 }
    }

    fn token(&mut self, kind: TokenKind, end: usize) -> Token<'a> {
        let span = Span::new(self.offset, end);
        let text = &self.source[span.start..span.end];
        self.offset = end;
        Token { kind, span, text }
    }

    fn scan(&mut self) -> Result<Token<'a>, Error> {
        let bytes = self.source.as_bytes();
        let at = self.offset;
        match bytes[at] {
            b' ' | b'\t' | b'\n' => {
                let mut end = at;
                while matches!(bytes.get(end), Some(b' ' | b'\t' | b'\n')) {
                    end += 1;
                }
                Ok(self.token(TokenKind::Whitespace, end))
            }
            b'\r' => Err(Error::new(ErrorKind::CarriageReturn, Span::new(at, at + 1))),
            b'#' => self.scan_comment(),
            b'{' => Ok(self.token(TokenKind::BraceOpen, at + 1)),
            b'}' => Ok(self.token(TokenKind::BraceClose, at + 1)),
            b'[' => Ok(self.token(TokenKind::BracketOpen, at + 1)),
            b']' => Ok(self.token(TokenKind::BracketClose, at + 1)),
            b'=' if bytes.get(at + 1) == Some(&b'>') => Ok(self.token(TokenKind::Arrow, at + 2)),
            b'=' => Ok(self.token(TokenKind::Equals, at + 1)),
            b':' if bytes.get(at + 1) == Some(&b'=') => Ok(self.token(TokenKind::Binder, at + 2)),
            b'"' => self.scan_string(),
            b'A'..=b'Z' => {
                let mut end = at + 1;
                while bytes.get(end).is_some_and(u8::is_ascii_alphanumeric) {
                    end += 1;
                }
                Ok(self.token(TokenKind::Atom, end))
            }
            b'a'..=b'z' => {
                let mut end = at + 1;
                while matches!(bytes.get(end), Some(b'a'..=b'z' | b'0'..=b'9' | b'_')) {
                    end += 1;
                }
                Ok(self.token(TokenKind::Ident, end))
            }
            b'-' | b'0'..=b'9' => self.scan_number(),
            _ => {
                let character = self.source[at..].chars().next().unwrap_or('\u{FFFD}');
                let span = Span::new(at, at + character.len_utf8());
                if at == 0 && character == '\u{FEFF}' {
                    Err(Error::new(ErrorKind::ByteOrderMark, span))
                } else {
                    Err(Error::new(ErrorKind::UnexpectedCharacter(character), span))
                }
            }
        }
    }

    fn scan_comment(&mut self) -> Result<Token<'a>, Error> {
        let mut end = self.offset + 1;
        while let Some(character) = self.source[end..].chars().next() {
            match character {
                '\n' => break,
                '\r' => {
                    return Err(Error::new(
                        ErrorKind::CarriageReturn,
                        Span::new(end, end + 1),
                    ));
                }
                control if control < '\u{20}' && control != '\t' => {
                    return Err(Error::new(
                        ErrorKind::ControlCharacter(control),
                        Span::new(end, end + 1),
                    ));
                }
                other => end += other.len_utf8(),
            }
        }
        Ok(self.token(TokenKind::Comment, end))
    }

    fn scan_string(&mut self) -> Result<Token<'a>, Error> {
        let start = self.offset;
        let mut end = start + 1;
        loop {
            let Some(character) = self.source[end..].chars().next() else {
                return Err(Error::new(
                    ErrorKind::UnterminatedString,
                    Span::new(start, end),
                ));
            };
            match character {
                '"' => {
                    end += 1;
                    break;
                }
                '\\' => end = escape(self.source, end)?.1,
                '\n' => {
                    return Err(Error::new(
                        ErrorKind::UnterminatedString,
                        Span::new(start, end),
                    ));
                }
                '\r' => {
                    return Err(Error::new(
                        ErrorKind::CarriageReturn,
                        Span::new(end, end + 1),
                    ));
                }
                control if control < '\u{20}' => {
                    return Err(Error::new(
                        ErrorKind::ControlCharacter(control),
                        Span::new(end, end + 1),
                    ));
                }
                other => end += other.len_utf8(),
            }
        }
        Ok(self.token(TokenKind::String, end))
    }

    fn scan_number(&mut self) -> Result<Token<'a>, Error> {
        let bytes = self.source.as_bytes();
        let start = self.offset;
        let mut end = start;
        if bytes.get(end) == Some(&b'-') {
            end += 1;
        }
        let digits = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if end == digits {
            return Err(Error::new(
                ErrorKind::UnexpectedCharacter('-'),
                Span::new(start, end),
            ));
        }
        if bytes.get(end) == Some(&b'.') {
            end += 1;
            while bytes.get(end).is_some_and(u8::is_ascii_digit) {
                end += 1;
            }
        }
        if matches!(bytes.get(end), Some(b'e' | b'E')) {
            end += 1;
            if matches!(bytes.get(end), Some(b'-' | b'+')) {
                end += 1;
            }
            while bytes.get(end).is_some_and(u8::is_ascii_digit) {
                end += 1;
            }
        }
        let token = self.token(TokenKind::Number, end);
        if canonical_number(token.text).is_none() {
            return Err(Error::new(
                ErrorKind::NumberSpelling(token.text.to_owned()),
                token.span,
            ));
        }
        Ok(token)
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.source.len() {
            return None;
        }
        let scanned = self.scan();
        if scanned.is_err() {
            self.offset = self.source.len();
        }
        Some(scanned)
    }
}

pub fn escape(source: &str, at: usize) -> Result<(char, usize), Error> {
    let unterminated = || Error::new(ErrorKind::UnterminatedString, Span::new(at, source.len()));
    let Some(marker) = source.get(at + 1..).and_then(|rest| rest.chars().next()) else {
        return Err(unterminated());
    };
    let simple = match marker {
        '"' => Some('"'),
        '\\' => Some('\\'),
        'n' => Some('\n'),
        'r' => Some('\r'),
        't' => Some('\t'),
        _ => None,
    };
    if let Some(character) = simple {
        return Ok((character, at + 2));
    }
    if marker != 'u' {
        return Err(Error::new(
            ErrorKind::UnknownEscape(marker),
            Span::new(at, at + 1 + marker.len_utf8()),
        ));
    }
    let bytes = source.as_bytes();
    let malformed = |end: usize| Error::new(ErrorKind::MalformedUnicodeEscape, Span::new(at, end));
    if bytes.get(at + 2) != Some(&b'{') {
        return Err(malformed(source.len().min(at + 3)));
    }
    let digits = at + 3;
    let mut end = digits;
    while matches!(bytes.get(end), Some(b'0'..=b'9' | b'A'..=b'F')) {
        end += 1;
    }
    if end == digits || end - digits > 6 || bytes.get(end) != Some(&b'}') {
        return Err(malformed(source.len().min(end + 1)));
    }
    let Ok(value) = u32::from_str_radix(&source[digits..end], 16) else {
        return Err(malformed(end + 1));
    };
    match char::from_u32(value) {
        Some(character) => Ok((character, end + 1)),
        None if (0xD800..=0xDFFF).contains(&value) => Err(Error::new(
            ErrorKind::SurrogateEscape(value),
            Span::new(at, end + 1),
        )),
        None => Err(Error::new(
            ErrorKind::ScalarOutOfRange(value),
            Span::new(at, end + 1),
        )),
    }
}

pub fn unescape(source: &str, span: Span) -> Result<String, Error> {
    let end = span.end.saturating_sub(1);
    let mut at = span.start.saturating_add(1);
    let mut cooked = String::new();
    while at < end {
        let Some(character) = source
            .get(at..end)
            .and_then(|remaining| remaining.chars().next())
        else {
            return Err(Error::new(ErrorKind::UnterminatedString, span));
        };
        if character == '\\' {
            let (decoded, next) = escape(source, at)?;
            cooked.push(decoded);
            at = next;
        } else {
            cooked.push(character);
            at += character.len_utf8();
        }
    }
    Ok(cooked)
}

pub(crate) fn canonical_number(text: &str) -> Option<NumberKind> {
    let bytes = text.as_bytes();
    let mut at = usize::from(bytes.first() == Some(&b'-'));
    at = uint(bytes, at)?;
    let mut kind = NumberKind::Integer;
    if bytes.get(at) == Some(&b'.') {
        at += 1;
        let digits = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if at == digits {
            return None;
        }
        kind = NumberKind::Float;
    }
    if bytes.get(at) == Some(&b'e') {
        at += 1;
        if bytes.get(at) == Some(&b'-') {
            at += 1;
        }
        at = uint(bytes, at)?;
        kind = NumberKind::Float;
    }
    (at == bytes.len()).then_some(kind)
}

fn uint(bytes: &[u8], at: usize) -> Option<usize> {
    match bytes.get(at)? {
        b'0' => Some(at + 1),
        b'1'..=b'9' => {
            let mut end = at + 1;
            while bytes.get(end).is_some_and(u8::is_ascii_digit) {
                end += 1;
            }
            Some(end)
        }
        _ => None,
    }
}
