mod visit;

use std::fmt;

use nuke_resolve::Resolution;
use nuke_syntax::expr::Document;
use nuke_syntax::{Error, Location, Span};

const IMPORT: &str = "import";

const EXTENSION: &str = ".nuke";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rule {
    AtomCase,
    IdentCase,
    ImportExtension,
    UnusedBinding,
}

impl Rule {
    pub const ALL: [Self; 4] = [
        Self::AtomCase,
        Self::IdentCase,
        Self::ImportExtension,
        Self::UnusedBinding,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::AtomCase => "atom-case",
            Self::IdentCase => "ident-case",
            Self::ImportExtension => "import-extension",
            Self::UnusedBinding => "unused-binding",
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
            Rule::UnusedBinding => write!(
                f,
                "`{}` is bound and nothing below it reads the name, and a binding is no part of the value, so the document says the same without it",
                self.spelling
            ),
        }
    }
}

pub fn lint(source: &str) -> Result<Vec<Diagnostic>, Error> {
    Ok(lint_document(&nuke_syntax::surface::parse(source)?))
}

pub fn lint_document(document: &Document) -> Vec<Diagnostic> {
    let mut found = visit::Pass::default().document(document);
    found.extend(Resolution::of(document).unread().map(|bound| Diagnostic {
        rule: Rule::UnusedBinding,
        spelling: bound.ident.as_str().into(),
        span: bound.span,
    }));
    found.sort_by_key(|diagnostic| diagnostic.span.start);
    found
}
