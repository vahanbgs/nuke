use crate::error::Span;
use crate::spec::Spec;
use crate::value::{Atom, Float, Ident, Integer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub bindings: Vec<Binding>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Tuple {
        bindings: Vec<Binding>,
        fields: Vec<Field>,
    },
    Map {
        bindings: Vec<Binding>,
        entries: Vec<Entry>,
    },
    List(Vec<Expr>),
    Access {
        operand: Box<Expr>,
        field: Name,
    },
    Index {
        operand: Box<Expr>,
        key: Box<Expr>,
    },
    Call {
        name: Name,
        operand: Box<Expr>,
    },
    Interpolation(Vec<Piece>),
    Reference(Ident),
    Atom(Atom),
    String(String),
    Integer(Integer),
    Float(Float),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    Hole { expr: Expr, spec: Option<Spec> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub name: Name,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: Name,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: Expr,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    pub ident: Ident,
    pub span: Span,
}
