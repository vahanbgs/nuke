use std::collections::VecDeque;

use crate::MAX_DEPTH;
use crate::error::{Error, ErrorKind, Span};
use crate::lexer::{Lexer, NumberKind, Token, TokenKind, canonical_number, unescape};
use crate::value::{Atom, Float, Ident, Integer, Map, Tuple, Value};

pub(crate) fn parse(source: &str) -> Result<Value, Error> {
    let mut parser = Parser::new(source);
    if parser.peek(0)?.is_none() {
        return Err(Error::new(
            ErrorKind::EmptyDocument,
            Span::new(0, source.len()),
        ));
    }
    let value = parser.value(0)?;
    match parser.peek(0)? {
        Some(token) => Err(Error::new(ErrorKind::TrailingInput, token.span)),
        None => Ok(value),
    }
}

struct Parser<'a> {
    source: &'a str,
    lexer: Lexer<'a>,
    lookahead: VecDeque<Token<'a>>,
    drained: bool,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            lexer: Lexer::new(source),
            lookahead: VecDeque::new(),
            drained: false,
        }
    }

    fn fill(&mut self, wanted: usize) -> Result<(), Error> {
        while !self.drained && self.lookahead.len() <= wanted {
            match self.lexer.next() {
                Some(token) => {
                    let token = token?;
                    if !token.kind.is_trivia() {
                        self.lookahead.push_back(token);
                    }
                }
                None => self.drained = true,
            }
        }
        Ok(())
    }

    fn peek(&mut self, ahead: usize) -> Result<Option<Token<'a>>, Error> {
        self.fill(ahead)?;
        Ok(self.lookahead.get(ahead).copied())
    }

    fn bump(&mut self) -> Result<Option<Token<'a>>, Error> {
        self.fill(0)?;
        Ok(self.lookahead.pop_front())
    }

    fn end(&self) -> Span {
        Span::new(self.source.len(), self.source.len())
    }

    fn value(&mut self, depth: usize) -> Result<Value, Error> {
        let Some(token) = self.peek(0)? else {
            return Err(Error::new(ErrorKind::ExpectedValue, self.end()));
        };
        if depth > MAX_DEPTH {
            return Err(Error::new(ErrorKind::TooDeep, token.span));
        }
        match token.kind {
            TokenKind::BracketOpen => self.list(depth),
            TokenKind::BraceOpen => self.block(depth),
            TokenKind::Atom => {
                self.bump()?;
                Ok(Value::Atom(Atom::new(token.text)))
            }
            TokenKind::String => {
                self.bump()?;
                Ok(Value::String(unescape(self.source, token.span)?))
            }
            TokenKind::Number => {
                self.bump()?;
                number(token)
            }
            TokenKind::Ident => Err(Error::new(
                ErrorKind::IdentAsValue(token.text.to_owned()),
                token.span,
            )),
            _ => Err(Error::new(ErrorKind::ExpectedValue, token.span)),
        }
    }

    fn list(&mut self, depth: usize) -> Result<Value, Error> {
        let open = self.bump()?.map_or_else(|| self.end(), |token| token.span);
        let mut items = Vec::new();
        loop {
            match self.peek(0)? {
                None => return Err(Error::new(ErrorKind::UnterminatedList, open)),
                Some(token) if token.kind == TokenKind::BracketClose => {
                    self.bump()?;
                    break;
                }
                Some(_) => items.push(self.value(depth + 1)?),
            }
        }
        Ok(Value::List(items))
    }

    fn block(&mut self, depth: usize) -> Result<Value, Error> {
        let open = self.bump()?.map_or_else(|| self.end(), |token| token.span);
        match (self.peek(0)?, self.peek(1)?) {
            (None, _) => Err(Error::new(ErrorKind::UnterminatedBlock, open)),
            (Some(token), _) if token.kind == TokenKind::BraceClose => {
                self.bump()?;
                Ok(Value::Tuple(Tuple::new()))
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
            let Some(token) = self.peek(0)? else {
                return Err(Error::new(ErrorKind::UnterminatedBlock, open));
            };
            if token.kind == TokenKind::BraceClose {
                self.bump()?;
                break;
            }
            if token.kind != TokenKind::Ident {
                let mixed = self
                    .peek(1)?
                    .is_some_and(|next| next.kind == TokenKind::Arrow);
                let kind = if mixed {
                    ErrorKind::MixedPairOperators
                } else {
                    ErrorKind::ExpectedFieldName
                };
                return Err(Error::new(kind, token.span));
            }
            self.bump()?;
            match self.bump()? {
                Some(bound) if bound.kind == TokenKind::Equals => {}
                Some(bound) if bound.kind == TokenKind::Arrow => {
                    return Err(Error::new(ErrorKind::MixedPairOperators, bound.span));
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
            let Some(token) = self.peek(0)? else {
                return Err(Error::new(ErrorKind::UnterminatedBlock, open));
            };
            if token.kind == TokenKind::BraceClose {
                self.bump()?;
                break;
            }
            let key = self.value(depth + 1)?;
            match self.bump()? {
                Some(bound) if bound.kind == TokenKind::Arrow => {}
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

fn number(token: Token<'_>) -> Result<Value, Error> {
    match canonical_number(token.text) {
        Some(NumberKind::Integer) => Ok(Value::Integer(Integer::new(token.text))),
        Some(NumberKind::Float) => Float::parse(token.text, token.span).map(Value::Float),
        None => Err(Error::new(
            ErrorKind::NumberSpelling(token.text.to_owned()),
            token.span,
        )),
    }
}
