use crate::error::{Error, ErrorKind, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Spec {
    pub fill: Option<char>,
    pub align: Option<Align>,
    pub sign: bool,
    pub prefixed: bool,
    pub zeroed: bool,
    pub width: Option<usize>,
    pub precision: Option<usize>,
    pub notation: Option<Notation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Centre,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notation {
    Binary,
    Octal,
    LowerHex,
    UpperHex,
    Exponent,
}

impl Spec {
    pub(crate) fn parse(text: &str, span: Span) -> Result<Self, Error> {
        let malformed = || Error::new(ErrorKind::MalformedSpec, span);
        let characters: Vec<char> = text.chars().collect();
        let mut at = 0;
        let mut spec = Self::default();
        if characters.is_empty() {
            return Err(malformed());
        }
        if let Some(align) = characters.get(1).copied().and_then(Align::of) {
            spec.fill = characters.first().copied();
            spec.align = Some(align);
            at = 2;
        } else if let Some(align) = characters.first().copied().and_then(Align::of) {
            spec.align = Some(align);
            at = 1;
        }
        spec.sign = take(&characters, &mut at, '+');
        spec.prefixed = take(&characters, &mut at, '#');
        spec.zeroed = take(&characters, &mut at, '0');
        spec.width = uint(&characters, &mut at, span)?;
        if take(&characters, &mut at, '.') {
            spec.precision = Some(uint(&characters, &mut at, span)?.ok_or_else(malformed)?);
        }
        spec.notation = characters.get(at).copied().and_then(Notation::of);
        if spec.notation.is_some() {
            at += 1;
        }
        if at != characters.len() {
            return Err(malformed());
        }
        Ok(spec)
    }
}

impl Align {
    const fn of(character: char) -> Option<Self> {
        match character {
            '<' => Some(Self::Left),
            '^' => Some(Self::Centre),
            '>' => Some(Self::Right),
            _ => None,
        }
    }
}

impl Notation {
    const fn of(character: char) -> Option<Self> {
        match character {
            'b' => Some(Self::Binary),
            'o' => Some(Self::Octal),
            'x' => Some(Self::LowerHex),
            'X' => Some(Self::UpperHex),
            'e' => Some(Self::Exponent),
            _ => None,
        }
    }
}

fn take(characters: &[char], at: &mut usize, wanted: char) -> bool {
    let found = characters.get(*at) == Some(&wanted);
    if found {
        *at += 1;
    }
    found
}

fn uint(characters: &[char], at: &mut usize, span: Span) -> Result<Option<usize>, Error> {
    let start = *at;
    if characters.get(start) == Some(&'0') {
        *at += 1;
        return Ok(Some(0));
    }
    while characters.get(*at).is_some_and(char::is_ascii_digit) {
        *at += 1;
    }
    if *at == start {
        return Ok(None);
    }
    characters[start..*at]
        .iter()
        .collect::<String>()
        .parse()
        .map(Some)
        .map_err(|_| Error::new(ErrorKind::MalformedSpec, span))
}
