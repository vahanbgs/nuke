use std::collections::HashSet;

use crate::MAX_DEPTH;
use crate::cursor::{Cursor, Number, surface_number};
use crate::error::{Error, ErrorKind, Span};
use crate::expr::{Binding, Document, Entry, Expr, ExprKind, Field, Form, Name, Piece};
use crate::lexer::{TokenKind, cook, unescape};
use crate::spec::Spec;
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
    if parser.heads_a_field()? {
        let value = parser.tuple(Vec::new(), Close::Eof, 0)?;
        return Ok(Document {
            bindings,
            value,
            form: Form::Fields,
        });
    }
    let value = parser.value(0)?;
    match parser.cursor.peek(0)? {
        Some(token) => Err(Error::new(ErrorKind::TrailingInput, token.span)),
        None => Ok(Document {
            bindings,
            value,
            form: Form::Value,
        }),
    }
}

#[derive(Clone, Copy)]
enum Close {
    Brace(Span),
    Eof,
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
            expr = match self.cursor.bump()? {
                Some(token) if token.kind == TokenKind::Ident => {
                    let field = Name {
                        ident: Ident::new(token.text),
                        span: token.span,
                    };
                    let span = Span::new(expr.span.start, field.span.end);
                    Expr {
                        kind: ExprKind::Access {
                            operand: Box::new(expr),
                            field,
                        },
                        span,
                    }
                }
                Some(token) if token.kind == TokenKind::ParenOpen => {
                    let key = self.value(reach + 1)?;
                    let close = match self.cursor.bump()? {
                        Some(token) if token.kind == TokenKind::ParenClose => token.span,
                        Some(token) => {
                            return Err(Error::new(ErrorKind::ExpectedKeyClose, token.span));
                        }
                        None => return Err(Error::new(ErrorKind::UnterminatedKey, token.span)),
                    };
                    let span = Span::new(expr.span.start, close.end);
                    Expr {
                        kind: ExprKind::Index {
                            operand: Box::new(expr),
                            key: Box::new(key),
                        },
                        span,
                    }
                }
                Some(token) => {
                    return Err(Error::new(ErrorKind::ExpectedAccessName, token.span));
                }
                None => {
                    return Err(Error::new(ErrorKind::ExpectedAccessName, self.cursor.end()));
                }
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
            TokenKind::At => return self.call(token.span, depth),
            TokenKind::InterpolationOpen => return self.interpolation(token.span, depth),
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
                match surface_number(token)? {
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

    fn call(&mut self, at: Span, depth: usize) -> Result<Expr, Error> {
        self.cursor.bump()?;
        let name = match self.cursor.bump()? {
            Some(token) if token.kind == TokenKind::Ident => Name {
                ident: Ident::new(token.text),
                span: token.span,
            },
            Some(token) => return Err(Error::new(ErrorKind::ExpectedBuiltinName, token.span)),
            None => {
                return Err(Error::new(
                    ErrorKind::ExpectedBuiltinName,
                    self.cursor.end(),
                ));
            }
        };
        let operand = self.operand(depth + 1)?;
        let span = Span::new(at.start, operand.span.end);
        Ok(Expr {
            kind: ExprKind::Call {
                name,
                operand: Box::new(operand),
            },
            span,
        })
    }

    fn interpolation(&mut self, at: Span, depth: usize) -> Result<Expr, Error> {
        self.cursor.bump()?;
        let mut pieces = Vec::new();
        loop {
            let Some(token) = self.cursor.peek(0)? else {
                return Err(Error::new(ErrorKind::UnterminatedString, at));
            };
            match token.kind {
                TokenKind::InterpolationClose => {
                    self.cursor.bump()?;
                    return Ok(Expr {
                        kind: ExprKind::Interpolation(pieces),
                        span: Span::new(at.start, token.span.end),
                    });
                }
                TokenKind::Text => {
                    self.cursor.bump()?;
                    let text = cook(self.cursor.source(), token.span, true)?;
                    pieces.push(Piece::Text(text));
                }
                TokenKind::HoleOpen => {
                    self.cursor.bump()?;
                    pieces.push(self.hole(token.span, depth + 1)?);
                }
                _ => return Err(Error::new(ErrorKind::ExpectedValue, token.span)),
            }
        }
    }

    fn hole(&mut self, open: Span, depth: usize) -> Result<Piece, Error> {
        let expr = self.value(depth)?;
        let mut spec = None;
        if let Some(token) = self
            .cursor
            .peek(0)?
            .filter(|next| next.kind == TokenKind::Spec)
        {
            self.cursor.bump()?;
            spec = Some(Spec::parse(token.text, token.span)?);
        }
        match self.cursor.bump()? {
            Some(token) if token.kind == TokenKind::HoleClose => Ok(Piece::Hole { expr, spec }),
            Some(token) if token.kind == TokenKind::Binder => {
                Err(Error::new(ErrorKind::MisplacedBinding, token.span))
            }
            Some(token) => Err(Error::new(ErrorKind::ExpectedHoleClose, token.span)),
            None => Err(Error::new(ErrorKind::UnterminatedHole, open)),
        }
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

    fn heads_a_field(&mut self) -> Result<bool, Error> {
        Ok(matches!(
            (self.cursor.peek(0)?, self.cursor.peek(1)?),
            (Some(name), Some(bound))
                if name.kind == TokenKind::Ident && bound.kind == TokenKind::Equals
        ))
    }

    fn block(&mut self, depth: usize) -> Result<Expr, Error> {
        let open = self
            .cursor
            .bump()?
            .map_or_else(|| self.cursor.end(), |token| token.span);
        let bindings = self.bindings(depth + 1)?;
        match self.cursor.peek(0)? {
            None => Err(Error::new(ErrorKind::UnterminatedBlock, open)),
            Some(token) if token.kind == TokenKind::BraceClose => {
                let kind = ExprKind::Tuple {
                    bindings,
                    fields: Vec::new(),
                };
                self.closed(kind, open, token.span)
            }
            Some(_) if self.heads_a_field()? => self.tuple(bindings, Close::Brace(open), depth),
            Some(_) => self.map(bindings, open, depth),
        }
    }

    fn tuple(&mut self, bindings: Vec<Binding>, close: Close, depth: usize) -> Result<Expr, Error> {
        let mut fields: Vec<Field> = Vec::new();
        let mut named: HashSet<&str> = HashSet::new();
        loop {
            let Some(token) = self.cursor.peek(0)? else {
                return match close {
                    Close::Brace(open) => Err(Error::new(ErrorKind::UnterminatedBlock, open)),
                    Close::Eof => Ok(run(bindings, fields, self.cursor.end())),
                };
            };
            if token.kind == TokenKind::BraceClose {
                return match close {
                    Close::Brace(open) => {
                        let kind = ExprKind::Tuple { bindings, fields };
                        self.closed(kind, open, token.span)
                    }
                    Close::Eof => Err(Error::new(ErrorKind::TrailingInput, token.span)),
                };
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
                None => {
                    return match close {
                        Close::Brace(open) => Err(Error::new(ErrorKind::UnterminatedBlock, open)),
                        Close::Eof => Err(Error::new(ErrorKind::ExpectedEquals, self.cursor.end())),
                    };
                }
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

fn run(bindings: Vec<Binding>, fields: Vec<Field>, end: Span) -> Expr {
    let span = fields
        .first()
        .zip(fields.last())
        .map_or(end, |(first, last)| {
            Span::new(first.name.span.start, last.value.span.end)
        });
    Expr {
        kind: ExprKind::Tuple { bindings, fields },
        span,
    }
}
