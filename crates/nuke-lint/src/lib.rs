use std::fmt;

use nuke_syntax::expr::{Binding, Document, Expr, ExprKind, Name, Piece};
use nuke_syntax::{Error, Location, Span};

const IMPORT: &str = "import";

const EXTENSION: &str = ".nuke";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rule {
    AtomCase,
    IdentCase,
    ImportExtension,
}

impl Rule {
    pub const ALL: [Self; 3] = [Self::AtomCase, Self::IdentCase, Self::ImportExtension];

    pub const fn name(self) -> &'static str {
        match self {
            Self::AtomCase => "atom-case",
            Self::IdentCase => "ident-case",
            Self::ImportExtension => "import-extension",
        }
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    rule: Rule,
    spelling: Box<str>,
    span: Span,
}

impl Diagnostic {
    pub const fn rule(&self) -> Rule {
        self.rule
    }

    pub const fn span(&self) -> Span {
        self.span
    }

    pub fn spelling(&self) -> &str {
        &self.spelling
    }

    pub fn location(&self, source: &str) -> Location {
        self.span.location(source)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.rule {
            Rule::AtomCase => write!(
                f,
                "`{}` is an atom, and an atom is spelled in UpperCamelCase: one capital opens a word and no two stand together",
                self.spelling
            ),
            Rule::IdentCase => write!(
                f,
                "`{}` is a name, and a name is spelled in snake_case: one `_` separates two words and none trails or doubles",
                self.spelling
            ),
            Rule::ImportExtension => write!(
                f,
                "`{}` is imported without the `{EXTENSION}` extension, which is not required to resolve and is asked for anyway",
                self.spelling
            ),
        }
    }
}

pub fn lint(source: &str) -> Result<Vec<Diagnostic>, Error> {
    Ok(lint_document(&nuke_syntax::surface::parse(source)?))
}

pub fn lint_document(document: &Document) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    for binding in &document.bindings {
        bound(&mut found, binding);
    }
    walk(&mut found, &document.value);
    found.sort_by_key(|diagnostic| diagnostic.span.start);
    found
}

fn walk(found: &mut Vec<Diagnostic>, expr: &Expr) {
    match &expr.kind {
        ExprKind::Tuple { bindings, fields } => {
            for binding in bindings {
                bound(found, binding);
            }
            for field in fields {
                named(found, &field.name);
                walk(found, &field.value);
            }
        }
        ExprKind::Map { bindings, entries } => {
            for binding in bindings {
                bound(found, binding);
            }
            for entry in entries {
                walk(found, &entry.key);
                walk(found, &entry.value);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                walk(found, item);
            }
        }
        ExprKind::Access { operand, field } => {
            walk(found, operand);
            named(found, field);
        }
        ExprKind::Index { operand, key } => {
            walk(found, operand);
            walk(found, key);
        }
        ExprKind::Call { name, operand } => {
            named(found, name);
            imported(found, name, operand);
            walk(found, operand);
        }
        ExprKind::Interpolation(pieces) => {
            for piece in pieces {
                if let Piece::Hole { expr, .. } = piece {
                    walk(found, expr);
                }
            }
        }
        ExprKind::Reference(ident) => ident_case(found, ident.as_str(), expr.span),
        ExprKind::Atom(atom) => atom_case(found, atom.as_str(), expr.span),
        ExprKind::String(_) | ExprKind::Integer(_) | ExprKind::Float(_) => {}
    }
}

fn bound(found: &mut Vec<Diagnostic>, binding: &Binding) {
    named(found, &binding.name);
    walk(found, &binding.value);
}

fn named(found: &mut Vec<Diagnostic>, name: &Name) {
    ident_case(found, name.ident.as_str(), name.span);
}

fn atom_case(found: &mut Vec<Diagnostic>, spelling: &str, span: Span) {
    let doubled = spelling
        .as_bytes()
        .windows(2)
        .any(|pair| pair[0].is_ascii_uppercase() && pair[1].is_ascii_uppercase());
    if doubled {
        found.push(Diagnostic {
            rule: Rule::AtomCase,
            spelling: spelling.into(),
            span,
        });
    }
}

fn ident_case(found: &mut Vec<Diagnostic>, spelling: &str, span: Span) {
    if spelling.ends_with('_') || spelling.contains("__") {
        found.push(Diagnostic {
            rule: Rule::IdentCase,
            spelling: spelling.into(),
            span,
        });
    }
}

fn imported(found: &mut Vec<Diagnostic>, name: &Name, operand: &Expr) {
    if name.ident.as_str() != IMPORT {
        return;
    }
    let ExprKind::String(path) = &operand.kind else {
        return;
    };
    if !path.ends_with(EXTENSION) {
        found.push(Diagnostic {
            rule: Rule::ImportExtension,
            spelling: path.as_str().into(),
            span: operand.span,
        });
    }
}
