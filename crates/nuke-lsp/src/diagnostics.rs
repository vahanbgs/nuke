use std::fs;
use std::path::{Path, PathBuf};

use line_index::LineIndex;
use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Range,
    Url,
};
use nuke_eval::error::ErrorKind;
use nuke_lint::Rule;
use nuke_syntax::Span;
use nuke_syntax::expr::Document;

use crate::document::Open;
use crate::position::{self, Encoding};

const SOURCE: &str = "nuke";

pub(crate) struct Report {
    pub(crate) found: Vec<Diagnostic>,
    pub(crate) read: Vec<PathBuf>,
    pub(crate) reduced: bool,
}

pub(crate) fn of(open: &Open, path: Option<&Path>, encoding: Encoding) -> Report {
    let parsed = match &open.parsed {
        Ok(parsed) => parsed,
        Err(error) => {
            return Report {
                found: vec![fault(
                    position::range(&open.index, encoding, error.span()),
                    error.to_string(),
                )],
                read: Vec::new(),
                reduced: false,
            };
        }
    };
    let mut found: Vec<Diagnostic> = nuke_lint::lint_document(&parsed.tree)
        .iter()
        .map(|found| {
            style(
                position::range(&open.index, encoding, found.span()),
                found.to_string(),
                found.rule(),
            )
        })
        .collect();
    match reduce(&parsed.tree, path) {
        Ok(read) => Report {
            found,
            read,
            reduced: true,
        },
        Err(error) => {
            found.push(reduction(&open.index, encoding, &error));
            Report {
                found,
                read: Vec::new(),
                reduced: false,
            }
        }
    }
}

fn reduce(tree: &Document, path: Option<&Path>) -> Result<Vec<PathBuf>, nuke_eval::Error> {
    match path {
        Some(path) => nuke_eval::reduce_at_with_files(tree, path).map(|reduction| reduction.files),
        None => nuke_eval::reduce(tree).map(|_| Vec::new()),
    }
}

fn reduction(index: &LineIndex, encoding: Encoding, error: &nuke_eval::Error) -> Diagnostic {
    let mut diagnostic = fault(
        position::range(index, encoding, error.span()),
        error.to_string(),
    );
    diagnostic.related_information = innermost(error).and_then(|(path, at, message)| {
        let uri = Url::from_file_path(path).ok()?;
        let text = fs::read_to_string(path).unwrap_or_default();
        let index = LineIndex::new(&text);
        let offset = position::point(&text, at);
        Some(vec![DiagnosticRelatedInformation {
            location: Location {
                uri,
                range: position::range(&index, encoding, Span::new(offset, offset)),
            },
            message,
        }])
    });
    diagnostic
}

fn innermost(error: &nuke_eval::Error) -> Option<(&Path, nuke_syntax::Location, String)> {
    let mut deepest = None;
    let mut current = error;
    while let ErrorKind::Import { path, at, cause } = current.kind() {
        deepest = Some((path.as_path(), *at, cause.to_string()));
        current = cause;
    }
    deepest
}

fn fault(range: Range, message: String) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some(SOURCE.to_owned()),
        message,
        ..Diagnostic::default()
    }
}

fn style(range: Range, message: String, rule: Rule) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String(rule.name().to_owned())),
        source: Some(SOURCE.to_owned()),
        message,
        ..Diagnostic::default()
    }
}
