use nuke_syntax::Span;
use nuke_syntax::expr::{Binding, Document, Expr, ExprKind, Name, Piece};

use crate::{Diagnostic, EXTENSION, IMPORT, Rule};

#[derive(Default)]
pub(crate) struct Pass {
    found: Vec<Diagnostic>,
}

impl Pass {
    pub(crate) fn document(mut self, document: &Document) -> Vec<Diagnostic> {
        self.bindings(&document.bindings);
        self.expr(&document.value);
        self.found
    }

    fn bindings(&mut self, bindings: &[Binding]) {
        for binding in bindings {
            self.named(&binding.name);
            self.expr(&binding.value);
        }
    }

    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Tuple { bindings, fields } => {
                self.bindings(bindings);
                for field in fields {
                    self.named(&field.name);
                    self.expr(&field.value);
                }
            }
            ExprKind::Map { bindings, entries } => {
                self.bindings(bindings);
                for entry in entries {
                    self.expr(&entry.key);
                    self.expr(&entry.value);
                }
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
            ExprKind::Apply {
                function, argument, ..
            } => {
                self.imported(function, argument);
                self.expr(function);
                self.expr(argument);
            }
            ExprKind::Builtin(name) => self.named(name),
            ExprKind::Group(inner) => self.expr(inner),
            ExprKind::Interpolation(pieces) => {
                for piece in pieces {
                    if let Piece::Hole { expr, .. } = piece {
                        self.expr(expr);
                    }
                }
            }
            ExprKind::Reference(ident) => self.ident_case(ident.as_str(), expr.span),
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

    fn imported(&mut self, function: &Expr, argument: &Expr) {
        let ExprKind::Builtin(name) = &function.kind else {
            return;
        };
        if name.ident.as_str() != IMPORT {
            return;
        }
        let ExprKind::String(path) = &argument.kind else {
            return;
        };
        if !path.ends_with(EXTENSION) {
            self.report(Rule::ImportExtension, path, argument.span);
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
