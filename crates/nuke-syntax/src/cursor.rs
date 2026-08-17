use std::collections::VecDeque;

use crate::error::{Error, ErrorKind, Span};
use crate::lexer::{Lexer, NumberKind, Token, canonical_number};
use crate::value::{Float, Integer};

pub(crate) enum Number {
    Integer(Integer),
    Float(Float),
}

pub(crate) fn number(token: Token<'_>) -> Result<Number, Error> {
    match canonical_number(token.text) {
        Some(NumberKind::Integer) => Ok(Number::Integer(Integer::new(token.text))),
        Some(NumberKind::Float) => Float::parse(token.text, token.span).map(Number::Float),
        None => Err(Error::new(
            ErrorKind::NumberSpelling(token.text.to_owned()),
            token.span,
        )),
    }
}

pub(crate) struct Cursor<'a> {
    source: &'a str,
    lexer: Lexer<'a>,
    lookahead: VecDeque<Token<'a>>,
    drained: bool,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self {
            source,
            lexer: Lexer::new(source),
            lookahead: VecDeque::new(),
            drained: false,
        }
    }

    pub(crate) const fn source(&self) -> &'a str {
        self.source
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

    pub(crate) fn peek(&mut self, ahead: usize) -> Result<Option<Token<'a>>, Error> {
        self.fill(ahead)?;
        Ok(self.lookahead.get(ahead).copied())
    }

    pub(crate) fn bump(&mut self) -> Result<Option<Token<'a>>, Error> {
        self.fill(0)?;
        Ok(self.lookahead.pop_front())
    }

    pub(crate) const fn end(&self) -> Span {
        Span::new(self.source.len(), self.source.len())
    }
}
