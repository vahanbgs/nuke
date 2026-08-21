use std::collections::BTreeMap;

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Hover, HoverContents, MarkupContent,
    MarkupKind, TextEdit,
};
use nuke_eval::Builtin;
use nuke_syntax::{Lexer, Span, TokenKind};

use crate::document::Open;
use crate::position::{self, Encoding};

pub(crate) fn completions(open: &Open, offset: usize, encoding: Encoding) -> Vec<CompletionItem> {
    match sigil(&open.text, offset) {
        Some(span) => {
            let range = position::range(&open.index, encoding, span);
            Builtin::ALL
                .into_iter()
                .map(|builtin| CompletionItem {
                    label: builtin.name().to_owned(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some(builtin.summary().to_owned()),
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                        range,
                        new_text: builtin.name().to_owned(),
                    })),
                    ..CompletionItem::default()
                })
                .collect()
        }
        None => in_scope(open, offset),
    }
}

pub(crate) fn hover(open: &Open, offset: usize, encoding: Encoding) -> Option<Hover> {
    let span = sigil(&open.text, offset)?;
    let builtin = Builtin::of(open.text.get(span.start..span.end)?)?;
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("`{builtin}`\n\n{}", builtin.summary()),
        }),
        range: Some(position::range(&open.index, encoding, span)),
    })
}

fn in_scope(open: &Open, offset: usize) -> Vec<CompletionItem> {
    let Some(parsed) = open.parsed() else {
        return Vec::new();
    };
    parsed
        .resolution
        .visible(offset)
        .map(|bound| (bound.ident.as_str(), bound))
        .collect::<BTreeMap<_, _>>()
        .into_keys()
        .map(|name| CompletionItem {
            label: name.to_owned(),
            kind: Some(CompletionItemKind::VARIABLE),
            ..CompletionItem::default()
        })
        .collect()
}

fn sigil(source: &str, offset: usize) -> Option<Span> {
    let mut previous = None;
    for token in Lexer::new(source)
        .map_while(Result::ok)
        .filter(|token| !token.kind.is_trivia())
    {
        if token.kind == TokenKind::Ident
            && token.span.start <= offset
            && offset <= token.span.end
            && previous == Some(TokenKind::At)
        {
            return Some(token.span);
        }
        if token.span.start >= offset {
            break;
        }
        previous = Some(token.kind);
    }
    (previous == Some(TokenKind::At)).then(|| Span::new(offset, offset))
}
