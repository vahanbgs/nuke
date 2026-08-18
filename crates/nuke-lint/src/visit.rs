use nuke_syntax::Span;
use nuke_syntax::expr::{Binding, Document, Expr, ExprKind, Name, Piece};
use nuke_syntax::value::Ident;

use crate::{Diagnostic, EXTENSION, IMPORT, Rule};

struct Bound {
    ident: Ident,
    span: Span,
    read: bool,
}

#[derive(Default)]
pub(crate) struct Pass {
    found: Vec<Diagnostic>,
    scope: Vec<Bound>,
}

impl Pass {
    pub(crate) fn document(mut self, document: &Document) -> Vec<Diagnostic> {
        let frame = self.open(&document.bindings);
        self.expr(&document.value);
        self.close(frame);
        self.found.sort_by_key(|diagnostic| diagnostic.span.start);
        self.found
    }

    fn open(&mut self, bindings: &[Binding]) -> usize {
        let frame = self.scope.len();
        for binding in bindings {
            self.named(&binding.name);
            self.expr(&binding.value);
            self.scope.push(Bound {
                ident: binding.name.ident.clone(),
                span: binding.name.span,
                read: false,
            });
        }
        frame
    }

    fn close(&mut self, frame: usize) {
        for bound in self.scope.split_off(frame) {
            if !bound.read {
                self.report(Rule::UnusedBinding, bound.ident.as_str(), bound.span);
            }
        }
    }

    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Tuple { bindings, fields } => {
                let frame = self.open(bindings);
                for field in fields {
                    self.named(&field.name);
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
            ExprKind::Access { operand, field } => {
                self.expr(operand);
                self.named(field);
            }
            ExprKind::Index { operand, key } => {
                self.expr(operand);
                self.expr(key);
            }
            ExprKind::Call { name, operand } => {
                self.named(name);
                self.imported(name, operand);
                self.expr(operand);
            }
            ExprKind::Interpolation(pieces) => {
                for piece in pieces {
                    if let Piece::Hole { expr, .. } = piece {
                        self.expr(expr);
                    }
                }
            }
            ExprKind::Reference(ident) => {
                self.ident_case(ident.as_str(), expr.span);
                if let Some(at) = self.scope.iter().rposition(|bound| bound.ident == *ident) {
                    self.scope[at].read = true;
                }
            }
            ExprKind::Atom(atom) => self.atom_case(atom.as_str(), expr.span),
            ExprKind::String(_) | ExprKind::Integer(_) | ExprKind::Float(_) => {}
        }
    }

    fn named(&mut self, name: &Name) {
        self.ident_case(name.ident.as_str(), name.span);
    }

    fn atom_case(&mut self, spelling: &str, span: Span) {
        let doubled = spelling
            .as_bytes()
            .windows(2)
            .any(|pair| pair[0].is_ascii_uppercase() && pair[1].is_ascii_uppercase());
        if doubled {
            self.report(Rule::AtomCase, spelling, span);
        }
    }

    fn ident_case(&mut self, spelling: &str, span: Span) {
        if spelling.ends_with('_') || spelling.contains("__") {
            self.report(Rule::IdentCase, spelling, span);
        }
    }

    fn imported(&mut self, name: &Name, operand: &Expr) {
        if name.ident.as_str() != IMPORT {
            return;
        }
        let ExprKind::String(path) = &operand.kind else {
            return;
        };
        if !path.ends_with(EXTENSION) {
            self.report(Rule::ImportExtension, path, operand.span);
        }
    }

    fn report(&mut self, rule: Rule, spelling: &str, span: Span) {
        self.found.push(Diagnostic {
            rule,
            spelling: spelling.into(),
            span,
        });
    }
}
