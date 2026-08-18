use line_index::{LineCol, LineIndex, TextSize, WideEncoding, WideLineCol};
use lsp_types::{Position, PositionEncodingKind, Range};
use nuke_syntax::{Location, Span};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Encoding {
    #[default]
    Utf16,
    Utf8,
}

impl Encoding {
    pub(crate) fn offered(offered: Option<&[PositionEncodingKind]>) -> Self {
        match offered {
            Some(kinds) if kinds.contains(&PositionEncodingKind::UTF8) => Self::Utf8,
            _ => Self::Utf16,
        }
    }

    pub(crate) fn kind(self) -> PositionEncodingKind {
        match self {
            Self::Utf16 => PositionEncodingKind::UTF16,
            Self::Utf8 => PositionEncodingKind::UTF8,
        }
    }
}

pub(crate) fn range(index: &LineIndex, encoding: Encoding, span: Span) -> Range {
    Range::new(
        position(index, encoding, span.start),
        position(index, encoding, span.end),
    )
}

pub(crate) fn whole(index: &LineIndex, encoding: Encoding, text: &str) -> Range {
    range(index, encoding, Span::new(0, text.len()))
}

pub(crate) fn position(index: &LineIndex, encoding: Encoding, offset: usize) -> Position {
    let at = index.line_col(within(index, offset));
    match encoding {
        Encoding::Utf8 => Position::new(at.line, at.col),
        Encoding::Utf16 => index
            .to_wide(WideEncoding::Utf16, at)
            .map_or_else(|| Position::new(at.line, at.col), wide),
    }
}

pub(crate) fn offset(index: &LineIndex, encoding: Encoding, position: Position) -> Option<usize> {
    let at = match encoding {
        Encoding::Utf8 => LineCol {
            line: position.line,
            col: position.character,
        },
        Encoding::Utf16 => index.to_utf8(
            WideEncoding::Utf16,
            WideLineCol {
                line: position.line,
                col: position.character,
            },
        )?,
    };
    index.offset(at).map(usize::from)
}

pub(crate) fn point(text: &str, at: Location) -> usize {
    let mut start = 0;
    for (number, line) in text.split_inclusive('\n').enumerate() {
        if number + 1 == at.line {
            return start
                + line
                    .chars()
                    .take(at.column.saturating_sub(1))
                    .map(char::len_utf8)
                    .sum::<usize>();
        }
        start += line.len();
    }
    start
}

fn wide(at: WideLineCol) -> Position {
    Position::new(at.line, at.col)
}

fn within(index: &LineIndex, offset: usize) -> TextSize {
    let offset = u32::try_from(offset).map_or_else(|_| index.len(), TextSize::new);
    offset.min(index.len())
}
