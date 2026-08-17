use std::fmt;

use nuke_syntax::{Float, MAX_DEPTH, Map, Tuple, Value};

use crate::error::{Path, Segment, too_deep};

pub type Error = crate::error::Error<ErrorKind>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    UnrepresentableName(String),
    UnrepresentableCharacter(char),
    TooDeep,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnrepresentableName(name) => {
                write!(f, "`{name}` is not a name an XML element can take")
            }
            Self::UnrepresentableCharacter(character) => {
                write!(
                    f,
                    "U+{:04X} is not a character XML can carry",
                    u32::from(*character)
                )
            }
            Self::TooDeep => too_deep(f),
        }
    }
}

pub fn to_string(value: &Value) -> Result<String, Error> {
    write(value, ROOT)
}

pub fn to_string_rooted(value: &Value, root: &str) -> Result<String, Error> {
    write(value, root)
}

fn write(value: &Value, root: &str) -> Result<String, Error> {
    if !is_name(root) {
        return Err(Error::new(
            ErrorKind::UnrepresentableName(root.to_owned()),
            Path::default(),
        ));
    }
    let mut writer = Writer::default();
    writer.element(root, value, 0, 0)?;
    Ok(writer.out)
}

#[derive(Default)]
struct Writer {
    out: String,
    path: Vec<Segment>,
}

impl Writer {
    fn element(
        &mut self,
        name: &str,
        value: &Value,
        indent: usize,
        depth: usize,
    ) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(self.error(ErrorKind::TooDeep));
        }
        self.open(name);
        match value {
            Value::Tuple(tuple) if !tuple.is_empty() => self.tuple(tuple, indent, depth)?,
            Value::Map(map) if !map.is_empty() => self.map(map, indent, depth)?,
            Value::List(items) if !items.is_empty() => self.list(items, indent, depth)?,
            other => self.text(other)?,
        }
        self.close(name);
        Ok(())
    }

    fn tuple(&mut self, tuple: &Tuple, indent: usize, depth: usize) -> Result<(), Error> {
        for (name, value) in tuple.iter() {
            self.newline(indent + 2);
            self.path.push(Segment::Field(name.as_str().to_owned()));
            self.element(name.as_str(), value, indent + 2, depth + 1)?;
            self.path.pop();
        }
        self.newline(indent);
        Ok(())
    }

    fn map(&mut self, map: &Map, indent: usize, depth: usize) -> Result<(), Error> {
        for (position, (key, value)) in map.iter().enumerate() {
            self.newline(indent + 2);
            self.path.push(Segment::Entry(position));
            self.entry(key, value, indent + 2, depth + 1)?;
            self.path.pop();
        }
        self.newline(indent);
        Ok(())
    }

    fn entry(
        &mut self,
        key: &Value,
        value: &Value,
        indent: usize,
        depth: usize,
    ) -> Result<(), Error> {
        self.open(ENTRY);
        self.newline(indent + 2);
        self.element(KEY, key, indent + 2, depth)?;
        self.newline(indent + 2);
        self.element(VALUE, value, indent + 2, depth)?;
        self.newline(indent);
        self.close(ENTRY);
        Ok(())
    }

    fn list(&mut self, items: &[Value], indent: usize, depth: usize) -> Result<(), Error> {
        for (index, item) in items.iter().enumerate() {
            self.newline(indent + 2);
            self.path.push(Segment::Index(index));
            self.element(ITEM, item, indent + 2, depth + 1)?;
            self.path.pop();
        }
        self.newline(indent);
        Ok(())
    }

    fn text(&mut self, value: &Value) -> Result<(), Error> {
        match value {
            Value::Atom(atom) => return self.escape(atom.as_str()),
            Value::String(text) => return self.escape(text),
            Value::Integer(integer) => self.out.push_str(integer.as_str()),
            Value::Float(number) => self.float(*number),
            Value::Tuple(_) | Value::Map(_) | Value::List(_) => {}
        }
        Ok(())
    }

    fn escape(&mut self, text: &str) -> Result<(), Error> {
        for character in text.chars() {
            match character {
                '&' => self.out.push_str("&amp;"),
                '<' => self.out.push_str("&lt;"),
                '>' => self.out.push_str("&gt;"),
                '\r' => self.out.push_str("&#xD;"),
                forbidden if is_forbidden(forbidden) => {
                    return Err(self.error(ErrorKind::UnrepresentableCharacter(forbidden)));
                }
                other => self.out.push(other),
            }
        }
        Ok(())
    }

    fn float(&mut self, number: Float) {
        let mut buffer = ryu::Buffer::new();
        self.out.push_str(buffer.format_finite(number.get()));
    }

    fn open(&mut self, name: &str) {
        self.out.push('<');
        self.out.push_str(name);
        self.out.push('>');
    }

    fn close(&mut self, name: &str) {
        self.out.push_str("</");
        self.out.push_str(name);
        self.out.push('>');
    }

    fn newline(&mut self, indent: usize) {
        self.out.push('\n');
        for _ in 0..indent {
            self.out.push(' ');
        }
    }

    fn error(&self, kind: ErrorKind) -> Error {
        Error::new(kind, Path::new(self.path.clone()))
    }
}

const ROOT: &str = "nuke";
const ENTRY: &str = "_entry";
const KEY: &str = "_key";
const VALUE: &str = "_value";
const ITEM: &str = "_item";

fn is_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters.next().is_some_and(is_name_start) && characters.all(is_name_char)
}

fn is_name_start(character: char) -> bool {
    character.is_ascii_alphabetic()
        || character == '_'
        || matches!(
            character,
            '\u{C0}'..='\u{D6}'
                | '\u{D8}'..='\u{F6}'
                | '\u{F8}'..='\u{2FF}'
                | '\u{370}'..='\u{37D}'
                | '\u{37F}'..='\u{1FFF}'
                | '\u{200C}'..='\u{200D}'
                | '\u{2070}'..='\u{218F}'
                | '\u{2C00}'..='\u{2FEF}'
                | '\u{3001}'..='\u{D7FF}'
                | '\u{F900}'..='\u{FDCF}'
                | '\u{FDF0}'..='\u{FFFD}'
                | '\u{10000}'..='\u{EFFFF}'
        )
}

fn is_name_char(character: char) -> bool {
    is_name_start(character)
        || character.is_ascii_digit()
        || matches!(
            character,
            '-' | '.' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}'
        )
}

fn is_forbidden(character: char) -> bool {
    matches!(
        character,
        '\u{0}'..='\u{8}' | '\u{B}' | '\u{C}' | '\u{E}'..='\u{1F}' | '\u{FFFE}' | '\u{FFFF}'
    )
}
