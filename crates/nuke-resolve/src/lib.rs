use nuke_syntax::Span;
use nuke_syntax::expr::{Binding, Document, Expr, ExprKind, Piece};
use nuke_syntax::value::Ident;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bound {
    pub ident: Ident,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Read {
    pub ident: Ident,
    pub span: Span,
    pub bound: Option<Id>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolution {
    bounds: Vec<Bound>,
    reads: Vec<Read>,
    was_read: Vec<bool>,
    scope: Vec<Id>,
}

impl Resolution {
    pub fn of(document: &Document) -> Self {
        let mut resolution = Self::default();
        let frame = resolution.open(&document.bindings);
        resolution.expr(&document.value);
        resolution.close(frame);
        resolution.scope = Vec::new();
        resolution
    }

    pub fn bounds(&self) -> &[Bound] {
        &self.bounds
    }

    pub fn reads(&self) -> &[Read] {
        &self.reads
    }

    pub fn bound(&self, id: Id) -> &Bound {
        &self.bounds[id.0]
    }

    pub fn at(&self, offset: usize) -> Option<Id> {
        self.reads
            .iter()
            .find(|read| covers(read.span, offset))
            .map_or_else(
                || {
                    self.bounds
                        .iter()
                        .position(|bound| covers(bound.span, offset))
                        .map(Id)
                },
                |read| read.bound,
            )
    }

    pub fn reads_of(&self, id: Id) -> impl Iterator<Item = &Read> {
        self.reads.iter().filter(move |read| read.bound == Some(id))
    }

    pub fn unread(&self) -> impl Iterator<Item = &Bound> {
        self.bounds
            .iter()
            .zip(&self.was_read)
            .filter_map(|(bound, was_read)| (!was_read).then_some(bound))
    }

    fn open(&mut self, bindings: &[Binding]) -> usize {
        let frame = self.scope.len();
        for binding in bindings {
            let id = Id(self.bounds.len());
            self.bounds.push(Bound {
                ident: binding.name.ident.clone(),
                span: binding.name.span,
            });
            self.was_read.push(false);
            self.expr(&binding.value);
            self.scope.push(id);
        }
        frame
    }

    fn close(&mut self, frame: usize) {
        self.scope.truncate(frame);
    }

    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Tuple { bindings, fields } => {
                let frame = self.open(bindings);
                for field in fields {
                    self.expr(&field.value);
                }
                self.close(frame);
            }
            ExprKind::Map { bindings, entries } => {
                let frame = self.open(bindings);
                for entry in entries {
                    self.expr(&entry.key);
                    self.expr(&entry.value);
                }
                self.close(frame);
            }
            ExprKind::List(items) => {
                for item in items {
                    self.expr(item);
                }
            }
            ExprKind::Access { operand, .. } => self.expr(operand),
            ExprKind::Index { operand, key } => {
                self.expr(operand);
                self.expr(key);
            }
            ExprKind::Apply {
                function, argument, ..
            } => {
                self.expr(function);
                self.expr(argument);
            }
            ExprKind::Builtin(_) => {}
            ExprKind::Group(inner) => self.expr(inner),
            ExprKind::Interpolation(pieces) => {
                for piece in pieces {
                    if let Piece::Hole { expr, .. } = piece {
                        self.expr(expr);
                    }
                }
            }
            ExprKind::Reference(ident) => self.read(ident, expr.span),
            ExprKind::Atom(_) | ExprKind::String(_) | ExprKind::Integer(_) | ExprKind::Float(_) => {
            }
        }
    }

    fn read(&mut self, ident: &Ident, span: Span) {
        let bound = self
            .scope
            .iter()
            .rev()
            .find(|id| self.bounds[id.0].ident == *ident)
            .copied();
        if let Some(id) = bound {
            self.was_read[id.0] = true;
        }
        self.reads.push(Read {
            ident: ident.clone(),
            span,
            bound,
        });
    }
}

const fn covers(span: Span, offset: usize) -> bool {
    span.start <= offset && offset < span.end
}
