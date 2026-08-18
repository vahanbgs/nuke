use std::path::PathBuf;

use line_index::LineIndex;
use nuke_resolve::Resolution;
use nuke_syntax::expr::Document;
use nuke_syntax::{Error, surface};

pub(crate) struct Open {
    pub(crate) text: String,
    pub(crate) index: LineIndex,
    pub(crate) parsed: Result<Parsed, Error>,
    pub(crate) read: Vec<PathBuf>,
    pub(crate) reduced: bool,
}

pub(crate) struct Parsed {
    pub(crate) tree: Document,
    pub(crate) resolution: Resolution,
}

impl Open {
    pub(crate) fn new(text: String) -> Self {
        let index = LineIndex::new(&text);
        let parsed = surface::parse(&text).map(|tree| Parsed {
            resolution: Resolution::of(&tree),
            tree,
        });
        Self {
            text,
            index,
            parsed,
            read: Vec::new(),
            reduced: false,
        }
    }

    pub(crate) fn parsed(&self) -> Option<&Parsed> {
        self.parsed.as_ref().ok()
    }
}
