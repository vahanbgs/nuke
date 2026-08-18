use line_index::LineIndex;
use lsp_types::{DocumentHighlight, DocumentHighlightKind, Range, SymbolKind};
use nuke_syntax::Span;
use nuke_syntax::expr::{Binding, Document, Expr, ExprKind, Field};
use serde::Serialize;

use crate::document::Open;
use crate::position::{self, Encoding};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Symbol {
    name: String,
    kind: SymbolKind,
    range: Range,
    selection_range: Range,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<Symbol>,
}

pub(crate) fn definition(open: &Open, offset: usize) -> Option<Span> {
    let resolution = &open.parsed()?.resolution;
    resolution.at(offset).map(|id| resolution.bound(id).span)
}

pub(crate) fn references(open: &Open, offset: usize, declaration: bool) -> Vec<Span> {
    let Some(parsed) = open.parsed() else {
        return Vec::new();
    };
    let resolution = &parsed.resolution;
    let Some(id) = resolution.at(offset) else {
        return Vec::new();
    };
    let declared = declaration.then(|| resolution.bound(id).span);
    declared
        .into_iter()
        .chain(resolution.reads_of(id).map(|read| read.span))
        .collect()
}

pub(crate) fn highlights(open: &Open, offset: usize, encoding: Encoding) -> Vec<DocumentHighlight> {
    let Some(parsed) = open.parsed() else {
        return Vec::new();
    };
    let resolution = &parsed.resolution;
    let Some(id) = resolution.at(offset) else {
        return Vec::new();
    };
    let written = highlight(
        &open.index,
        encoding,
        resolution.bound(id).span,
        DocumentHighlightKind::WRITE,
    );
    std::iter::once(written)
        .chain(resolution.reads_of(id).map(|read| {
            highlight(
                &open.index,
                encoding,
                read.span,
                DocumentHighlightKind::READ,
            )
        }))
        .collect()
}

pub(crate) fn symbols(open: &Open, encoding: Encoding) -> Vec<Symbol> {
    let Some(parsed) = open.parsed() else {
        return Vec::new();
    };
    let outline = Outline {
        index: &open.index,
        encoding,
    };
    outline.document(&parsed.tree)
}

struct Outline<'a> {
    index: &'a LineIndex,
    encoding: Encoding,
}

impl Outline<'_> {
    fn document(&self, document: &Document) -> Vec<Symbol> {
        document
            .bindings
            .iter()
            .map(|binding| self.binding(binding))
            .chain(self.expr(&document.value))
            .collect()
    }

    fn expr(&self, expr: &Expr) -> Vec<Symbol> {
        match &expr.kind {
            ExprKind::Tuple { bindings, fields } => bindings
                .iter()
                .map(|binding| self.binding(binding))
                .chain(fields.iter().map(|field| self.field(field)))
                .collect(),
            ExprKind::Map { bindings, .. } => bindings
                .iter()
                .map(|binding| self.binding(binding))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn binding(&self, binding: &Binding) -> Symbol {
        Symbol {
            name: binding.name.ident.as_str().to_owned(),
            kind: SymbolKind::VARIABLE,
            range: self.range(Span::new(binding.name.span.start, binding.value.span.end)),
            selection_range: self.range(binding.name.span),
            children: self.expr(&binding.value),
        }
    }

    fn field(&self, field: &Field) -> Symbol {
        Symbol {
            name: field.name.ident.as_str().to_owned(),
            kind: kind(&field.value),
            range: self.range(Span::new(field.name.span.start, field.value.span.end)),
            selection_range: self.range(field.name.span),
            children: self.expr(&field.value),
        }
    }

    fn range(&self, span: Span) -> Range {
        position::range(self.index, self.encoding, span)
    }
}

fn highlight(
    index: &LineIndex,
    encoding: Encoding,
    span: Span,
    kind: DocumentHighlightKind,
) -> DocumentHighlight {
    DocumentHighlight {
        range: position::range(index, encoding, span),
        kind: Some(kind),
    }
}

fn kind(value: &Expr) -> SymbolKind {
    match &value.kind {
        ExprKind::Tuple { .. } | ExprKind::Map { .. } => SymbolKind::OBJECT,
        ExprKind::List(_) => SymbolKind::ARRAY,
        ExprKind::String(_) | ExprKind::Interpolation(_) => SymbolKind::STRING,
        ExprKind::Integer(_) | ExprKind::Float(_) => SymbolKind::NUMBER,
        ExprKind::Atom(_) => SymbolKind::ENUM_MEMBER,
        ExprKind::Access { .. } | ExprKind::Index { .. } | ExprKind::Call { .. } => {
            SymbolKind::FIELD
        }
        ExprKind::Reference(_) => SymbolKind::VARIABLE,
    }
}
