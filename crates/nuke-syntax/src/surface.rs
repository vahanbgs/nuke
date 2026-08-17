use std::collections::HashSet;

use crate::MAX_DEPTH;
use crate::cursor::{Cursor, Number, number};
use crate::error::{Error, ErrorKind, Span};
use crate::expr::{Binding, Document, Entry, Expr, ExprKind, Field, Name};
use crate::lexer::{TokenKind, unescape};
use crate::value::{Atom, Ident};

pub fn parse(source: &str) -> Result<Document, Error> {
    let mut parser = Parser {
        cursor: Cursor::new(source),
    };
    if parser.cursor.peek(0)?.is_none() {
        return Err(Error::new(
            ErrorKind::EmptyDocument,
            Span::new(0, source.len()),
        ));
    }
    let bindings = parser.bindings(0)?;
    if parser.cursor.peek(0)?.is_none() {
        return Err(Error::new(
            ErrorKind::OnlyBindings,
            Span::new(0, source.len()),
        ));
    }
    let value = parser.value(0)?;
    match parser.cursor.peek(0)? {
        Some(token) => Err(Error::new(ErrorKind::TrailingInput, token.span)),
        None => Ok(Document { bindings, value }),
    }
}

struct Parser<'a> {
    cursor: Cursor<'a>,
}

impl Parser<'_> {
    fn bindings(&mut self, depth: usize) -> Result<Vec<Binding>, Error> {
        let mut bindings: Vec<Binding> = Vec::new();
        let mut bound: HashSet<&str> = HashSet::new();
        loop {
            let (Some(name), Some(binder)) = (self.cursor.peek(0)?, self.cursor.peek(1)?) else {
                return Ok(bindings);
            };
            if binder.kind != TokenKind::Binder {
                return Ok(bindings);
            }
            if name.kind != TokenKind::Ident {
                return Err(Error::new(ErrorKind::ExpectedBindingName, name.span));
            }
            self.cursor.bump()?;
            self.cursor.bump()?;
            let value = self.value(depth)?;
            if !bound.insert(name.text) {
                return Err(Error::new(
                    ErrorKind::DuplicateBinding(name.text.to_owned()),
                    name.span,
                ));
            }
            bindings.push(Binding {
                name: Name {
                    ident: Ident::new(name.text),
                    span: name.span,
                },
                value,
            });
        }
    }

    fn value(&mut self, depth: usize) -> Result<Expr, Error> {
        let mut expr = self.operand(depth)?;
        let mut reach = depth;
        while self
            .cursor
            .peek(0)?
            .is_some_and(|token| token.kind == TokenKind::Dot)
        {
            reach += 1;
            if reach > MAX_DEPTH {
                return Err(Error::new(ErrorKind::TooDeep, expr.span));
            }
            self.cursor.bump()?;
            let field = match self.cursor.bump()? {
                Some(token) if token.kind == TokenKind::Ident => Name {
                    ident: Ident::new(token.text),
                    span: token.span,
                },
                Some(token) => {
                    return Err(Error::new(ErrorKind::ExpectedAccessName, token.span));
                }
                None => {
                    return Err(Error::new(ErrorKind::ExpectedAccessName, self.cursor.end()));
                }
            };
            let span = Span::new(expr.span.start, field.span.end);
            expr = Expr {
                kind: ExprKind::Access {
                    operand: Box::new(expr),
                    field,
                },
                span,
            };
        }
        Ok(expr)
    }

    fn operand(&mut self, depth: usize) -> Result<Expr, Error> {
        let Some(token) = self.cursor.peek(0)? else {
            return Err(Error::new(ErrorKind::ExpectedValue, self.cursor.end()));
        };
        if depth > MAX_DEPTH {
            return Err(Error::new(ErrorKind::TooDeep, token.span));
        }
        let kind = match token.kind {
            TokenKind::BracketOpen => return self.list(depth),
            TokenKind::BraceOpen => return self.block(depth),
            TokenKind::Ident => {
                self.cursor.bump()?;
                ExprKind::Reference(Ident::new(token.text))
            }
            TokenKind::Atom => {
                self.cursor.bump()?;
                ExprKind::Atom(Atom::new(token.text))
            }
            TokenKind::String => {
                self.cursor.bump()?;
                ExprKind::String(unescape(self.cursor.source(), token.span)?)
            }
            TokenKind::Number => {
                self.cursor.bump()?;
                match number(token)? {
                    Number::Integer(integer) => ExprKind::Integer(integer),
                    Number::Float(float) => ExprKind::Float(float),
                }
            }
            TokenKind::Binder => {
                return Err(Error::new(ErrorKind::MisplacedBinding, token.span));
            }
            _ => return Err(Error::new(ErrorKind::ExpectedValue, token.span)),
        };
        Ok(Expr {
            kind,
            span: token.span,
        })
    }

    fn list(&mut self, depth: usize) -> Result<Expr, Error> {
        let open = self
            .cursor
            .bump()?
            .map_or_else(|| self.cursor.end(), |token| token.span);
        let mut items = Vec::new();
        loop {
            match self.cursor.peek(0)? {
                None => return Err(Error::new(ErrorKind::UnterminatedList, open)),
                Some(token) if token.kind == TokenKind::BracketClose => {
                    return self.closed(ExprKind::List(items), open, token.span);
                }
                Some(_) => items.push(self.value(depth + 1)?),
            }
        }
    }

    fn block(&mut self, depth: usize) -> Result<Expr, Error> {
        let open = self
            .cursor
            .bump()?
            .map_or_else(|| self.cursor.end(), |token| token.span);
        let bindings = self.bindings(depth + 1)?;
        match (self.cursor.peek(0)?, self.cursor.peek(1)?) {
            (None, _) => Err(Error::new(ErrorKind::UnterminatedBlock, open)),
            (Some(token), _) if token.kind == TokenKind::BraceClose => {
                let kind = ExprKind::Tuple {
                    bindings,
                    fields: Vec::new(),
                };
                self.closed(kind, open, token.span)
            }
            (Some(name), Some(bound))
                if name.kind == TokenKind::Ident && bound.kind == TokenKind::Equals =>
            {
                self.tuple(bindings, open, depth)
            }
            _ => self.map(bindings, open, depth),
        }
    }

    fn tuple(&mut self, bindings: Vec<Binding>, open: Span, depth: usize) -> Result<Expr, Error> {
        let mut fields: Vec<Field> = Vec::new();
        let mut named: HashSet<&str> = HashSet::new();
        loop {
            let Some(token) = self.cursor.peek(0)? else {
                return Err(Error::new(ErrorKind::UnterminatedBlock, open));
            };
            if token.kind == TokenKind::BraceClose {
                let kind = ExprKind::Tuple { bindings, fields };
                return self.closed(kind, open, token.span);
            }
            if token.kind == TokenKind::Binder {
                return Err(Error::new(ErrorKind::MisplacedBinding, token.span));
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
                    return Err(Error::new(ErrorKind::MisplacedBinding, bound.span));
                }
                Some(bound) => return Err(Error::new(ErrorKind::ExpectedEquals, bound.span)),
                None => return Err(Error::new(ErrorKind::UnterminatedBlock, open)),
            }
            let value = self.value(depth + 1)?;
            if !named.insert(token.text) {
                return Err(Error::new(
                    ErrorKind::DuplicateField(token.text.to_owned()),
                    token.span,
                ));
            }
            fields.push(Field {
                name: Name {
                    ident: Ident::new(token.text),
                    span: token.span,
                },
                value,
            });
        }
    }

    fn map(&mut self, bindings: Vec<Binding>, open: Span, depth: usize) -> Result<Expr, Error> {
        let mut entries = Vec::new();
        loop {
            let Some(token) = self.cursor.peek(0)? else {
                return Err(Error::new(ErrorKind::UnterminatedBlock, open));
            };
            if token.kind == TokenKind::BraceClose {
                let kind = ExprKind::Map { bindings, entries };
                return self.closed(kind, open, token.span);
            }
            let key = self.value(depth + 1)?;
            match self.cursor.bump()? {
                Some(bound) if bound.kind == TokenKind::Arrow => {}
                Some(bound) if bound.kind == TokenKind::Binder => {
                    let kind = if matches!(key.kind, ExprKind::Reference(_)) {
                        ErrorKind::MisplacedBinding
                    } else {
                        ErrorKind::ExpectedBindingName
                    };
                    return Err(Error::new(kind, bound.span));
                }
                Some(bound) => return Err(Error::new(ErrorKind::ExpectedArrow, bound.span)),
                None => return Err(Error::new(ErrorKind::UnterminatedBlock, open)),
            }
            let value = self.value(depth + 1)?;
            entries.push(Entry { key, value });
        }
    }

    fn closed(&mut self, kind: ExprKind, open: Span, close: Span) -> Result<Expr, Error> {
        self.cursor.bump()?;
        Ok(Expr {
            kind,
            span: Span::new(open.start, close.end),
        })
    }
}
