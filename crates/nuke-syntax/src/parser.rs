use crate::MAX_DEPTH;
use crate::cursor::{Cursor, Number, number};
use crate::error::{Error, ErrorKind, Span};
use crate::lexer::{TokenKind, unescape};
use crate::value::{Atom, Ident, Map, Tuple, Value};

pub(crate) fn parse(source: &str) -> Result<Value, Error> {
    let mut parser = Parser {
        cursor: Cursor::new(source),
    };
    if parser.cursor.peek(0)?.is_none() {
        return Err(Error::new(
            ErrorKind::EmptyDocument,
            Span::new(0, source.len()),
        ));
    }
    let value = parser.value(0)?;
    match parser.cursor.peek(0)? {
        Some(token) if token.kind == TokenKind::Dot => {
            Err(Error::new(ErrorKind::SurfaceDot, token.span))
        }
        Some(token) if token.kind == TokenKind::At => {
            Err(Error::new(ErrorKind::SurfaceCall, token.span))
        }
        Some(token) => Err(Error::new(ErrorKind::TrailingInput, token.span)),
        None => Ok(value),
    }
}

struct Parser<'a> {
    cursor: Cursor<'a>,
}

impl Parser<'_> {
    fn value(&mut self, depth: usize) -> Result<Value, Error> {
        let Some(token) = self.cursor.peek(0)? else {
            return Err(Error::new(ErrorKind::ExpectedValue, self.cursor.end()));
        };
        if depth > MAX_DEPTH {
            return Err(Error::new(ErrorKind::TooDeep, token.span));
        }
        match token.kind {
            TokenKind::BracketOpen => self.list(depth),
            TokenKind::BraceOpen => self.block(depth),
            TokenKind::Atom => {
                self.cursor.bump()?;
                Ok(Value::Atom(Atom::new(token.text)))
            }
            TokenKind::String => {
                self.cursor.bump()?;
                Ok(Value::String(unescape(self.cursor.source(), token.span)?))
            }
            TokenKind::Number => {
                self.cursor.bump()?;
                match number(token)? {
                    Number::Integer(integer) => Ok(Value::Integer(integer)),
                    Number::Float(float) => Ok(Value::Float(float)),
                }
            }
            TokenKind::Ident => Err(Error::new(
                ErrorKind::IdentAsValue(token.text.to_owned()),
                token.span,
            )),
            TokenKind::Binder => Err(Error::new(ErrorKind::SurfaceBinder, token.span)),
            TokenKind::Dot => Err(Error::new(ErrorKind::SurfaceDot, token.span)),
            TokenKind::At => Err(Error::new(ErrorKind::SurfaceCall, token.span)),
            TokenKind::InterpolationOpen => {
                Err(Error::new(ErrorKind::SurfaceInterpolation, token.span))
            }
            _ => Err(Error::new(ErrorKind::ExpectedValue, token.span)),
        }
    }

    fn list(&mut self, depth: usize) -> Result<Value, Error> {
        let open = self
            .cursor
            .bump()?
            .map_or_else(|| self.cursor.end(), |token| token.span);
        let mut items = Vec::new();
        loop {
            match self.cursor.peek(0)? {
                None => return Err(Error::new(ErrorKind::UnterminatedList, open)),
                Some(token) if token.kind == TokenKind::BracketClose => {
                    self.cursor.bump()?;
                    break;
                }
                Some(_) => items.push(self.value(depth + 1)?),
            }
        }
        Ok(Value::List(items))
    }

    fn block(&mut self, depth: usize) -> Result<Value, Error> {
        let open = self
            .cursor
            .bump()?
            .map_or_else(|| self.cursor.end(), |token| token.span);
        match (self.cursor.peek(0)?, self.cursor.peek(1)?) {
            (None, _) => Err(Error::new(ErrorKind::UnterminatedBlock, open)),
            (Some(token), _) if token.kind == TokenKind::BraceClose => {
                self.cursor.bump()?;
                Ok(Value::Tuple(Tuple::new()))
            }
            (Some(name), Some(bound))
                if name.kind == TokenKind::Ident && bound.kind == TokenKind::Binder =>
            {
                Err(Error::new(ErrorKind::SurfaceBinder, bound.span))
            }
            (Some(name), Some(bound))
                if name.kind == TokenKind::Ident && bound.kind == TokenKind::Equals =>
            {
                self.tuple(open, depth)
            }
            _ => self.map(open, depth),
        }
    }

    fn tuple(&mut self, open: Span, depth: usize) -> Result<Value, Error> {
        let mut fields = Tuple::new();
        loop {
            let Some(token) = self.cursor.peek(0)? else {
                return Err(Error::new(ErrorKind::UnterminatedBlock, open));
            };
            if token.kind == TokenKind::BraceClose {
                self.cursor.bump()?;
                break;
            }
            if token.kind != TokenKind::Ident {
                let mixed = self
                    .cursor
                    .peek(1)?
                    .is_some_and(|next| next.kind == TokenKind::Arrow);
                let kind = if mixed {
                    ErrorKind::MixedPairOperators
                } else {
                    ErrorKind::ExpectedFieldName
                };
                return Err(Error::new(kind, token.span));
            }
            self.cursor.bump()?;
            match self.cursor.bump()? {
                Some(bound) if bound.kind == TokenKind::Equals => {}
                Some(bound) if bound.kind == TokenKind::Arrow => {
                    return Err(Error::new(ErrorKind::MixedPairOperators, bound.span));
                }
                Some(bound) if bound.kind == TokenKind::Binder => {
                    return Err(Error::new(ErrorKind::SurfaceBinder, bound.span));
                }
                Some(bound) if bound.kind == TokenKind::Dot => {
                    return Err(Error::new(ErrorKind::SurfaceDot, bound.span));
                }
                Some(bound) if bound.kind == TokenKind::At => {
                    return Err(Error::new(ErrorKind::SurfaceCall, bound.span));
                }
                Some(bound) => return Err(Error::new(ErrorKind::ExpectedEquals, bound.span)),
                None => return Err(Error::new(ErrorKind::UnterminatedBlock, open)),
            }
            let value = self.value(depth + 1)?;
            if fields.insert(Ident::new(token.text), value).is_some() {
                return Err(Error::new(
                    ErrorKind::DuplicateField(token.text.to_owned()),
                    token.span,
                ));
            }
        }
        Ok(Value::Tuple(fields))
    }

    fn map(&mut self, open: Span, depth: usize) -> Result<Value, Error> {
        let mut entries = Map::new();
        loop {
            let Some(token) = self.cursor.peek(0)? else {
                return Err(Error::new(ErrorKind::UnterminatedBlock, open));
            };
            if token.kind == TokenKind::BraceClose {
                self.cursor.bump()?;
                break;
            }
            let key = self.value(depth + 1)?;
            match self.cursor.bump()? {
                Some(bound) if bound.kind == TokenKind::Arrow => {}
                Some(bound) if bound.kind == TokenKind::Binder => {
                    return Err(Error::new(ErrorKind::SurfaceBinder, bound.span));
                }
                Some(bound) if bound.kind == TokenKind::Dot => {
                    return Err(Error::new(ErrorKind::SurfaceDot, bound.span));
                }
                Some(bound) if bound.kind == TokenKind::At => {
                    return Err(Error::new(ErrorKind::SurfaceCall, bound.span));
                }
                Some(bound) => return Err(Error::new(ErrorKind::ExpectedArrow, bound.span)),
                None => return Err(Error::new(ErrorKind::UnterminatedBlock, open)),
            }
            let value = self.value(depth + 1)?;
            if entries.insert(key, value).is_some() {
                return Err(Error::new(ErrorKind::DuplicateKey, token.span));
            }
        }
        Ok(Value::Map(entries))
    }
}
