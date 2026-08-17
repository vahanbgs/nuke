use std::fmt::{self, Write as _};

use nuke_syntax::{Atom, Float, MAX_DEPTH, Map, Tuple, Value};

use crate::error::{Path, Segment, article, form, too_deep};

pub type Error = crate::error::Error<ErrorKind>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    UnrepresentableRoot(&'static str),
    UnrepresentableKey(&'static str),
    TooDeep,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnrepresentableRoot(form) => write!(
                f,
                "{} {form} cannot be a KDL document; only a tuple, a map or a list opens one",
                article(form)
            ),
            Self::UnrepresentableKey(form) => write!(
                f,
                "{} {form} cannot name a KDL node; only a string or an atom names one",
                article(form)
            ),
            Self::TooDeep => too_deep(f),
        }
    }
}

pub fn to_string(value: &Value) -> Result<String, Error> {
    let mut writer = Writer::default();
    writer.document(value)?;
    Ok(writer.out)
}

#[derive(Default)]
struct Writer {
    out: String,
    path: Vec<Segment>,
}

impl Writer {
    fn document(&mut self, value: &Value) -> Result<(), Error> {
        match value {
            Value::Tuple(tuple) => self.tuple(tuple, 0, 0),
            Value::Map(map) => self.map(map, 0, 0),
            Value::List(items) => self.list(items, 0, 0),
            other => Err(self.error(ErrorKind::UnrepresentableRoot(form(other)))),
        }
    }

    fn tuple(&mut self, tuple: &Tuple, indent: usize, depth: usize) -> Result<(), Error> {
        for (name, value) in tuple.iter() {
            self.path.push(Segment::Field(name.as_str().to_owned()));
            self.line(indent);
            self.field(name.as_str());
            self.node(value, indent, depth + 1)?;
            self.path.pop();
        }
        Ok(())
    }

    fn map(&mut self, map: &Map, indent: usize, depth: usize) -> Result<(), Error> {
        for (position, (key, value)) in map.iter().enumerate() {
            self.path.push(Segment::Entry(position));
            self.line(indent);
            self.key(key)?;
            self.node(value, indent, depth + 1)?;
            self.path.pop();
        }
        Ok(())
    }

    fn list(&mut self, items: &[Value], indent: usize, depth: usize) -> Result<(), Error> {
        for (index, item) in items.iter().enumerate() {
            self.path.push(Segment::Index(index));
            self.line(indent);
            self.out.push_str(ITEM);
            self.node(item, indent, depth + 1)?;
            self.path.pop();
        }
        Ok(())
    }

    fn node(&mut self, value: &Value, indent: usize, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(self.error(ErrorKind::TooDeep));
        }
        match value {
            Value::Tuple(tuple) if !tuple.is_empty() => {
                self.open();
                self.tuple(tuple, indent + 2, depth)?;
                self.close(indent);
            }
            Value::Map(map) if !map.is_empty() => {
                self.open();
                self.map(map, indent + 2, depth)?;
                self.close(indent);
            }
            Value::List(items) if !items.is_empty() => {
                self.open();
                self.list(items, indent + 2, depth)?;
                self.close(indent);
            }
            Value::Tuple(_) | Value::Map(_) | Value::List(_) => {}
            Value::Atom(atom) => {
                self.out.push(' ');
                self.atom(atom);
            }
            Value::String(text) => {
                self.out.push(' ');
                self.string(text);
            }
            Value::Integer(integer) => {
                self.out.push(' ');
                self.out.push_str(integer.as_str());
            }
            Value::Float(number) => {
                self.out.push(' ');
                self.float(*number);
            }
        }
        Ok(())
    }

    fn key(&mut self, key: &Value) -> Result<(), Error> {
        match key {
            Value::Atom(atom) => {
                self.out.push_str(atom.as_str());
                Ok(())
            }
            Value::String(text) => {
                self.string(text);
                Ok(())
            }
            other => Err(self.error(ErrorKind::UnrepresentableKey(form(other)))),
        }
    }

    fn field(&mut self, name: &str) {
        if RESERVED.contains(&name) {
            self.string(name);
        } else {
            self.out.push_str(name);
        }
    }

    fn atom(&mut self, atom: &Atom) {
        match atom.as_str() {
            "True" => self.out.push_str("#true"),
            "False" => self.out.push_str("#false"),
            "Null" => self.out.push_str("#null"),
            spelling => self.out.push_str(spelling),
        }
    }

    fn string(&mut self, text: &str) {
        self.out.push('"');
        for character in text.chars() {
            match character {
                '"' => self.out.push_str("\\\""),
                '\\' => self.out.push_str("\\\\"),
                '\n' => self.out.push_str("\\n"),
                '\r' => self.out.push_str("\\r"),
                '\t' => self.out.push_str("\\t"),
                forbidden if is_forbidden(forbidden) => {
                    let _ = write!(self.out, "\\u{{{:X}}}", u32::from(forbidden));
                }
                other => self.out.push(other),
            }
        }
        self.out.push('"');
    }

    fn float(&mut self, number: Float) {
        let mut buffer = ryu::Buffer::new();
        self.out.push_str(buffer.format_finite(number.get()));
    }

    fn open(&mut self) {
        self.out.push_str(" {");
    }

    fn close(&mut self, indent: usize) {
        self.line(indent);
        self.out.push('}');
    }

    fn line(&mut self, indent: usize) {
        if !self.out.is_empty() {
            self.out.push('\n');
        }
        for _ in 0..indent {
            self.out.push(' ');
        }
    }

    fn error(&self, kind: ErrorKind) -> Error {
        Error::new(kind, Path::new(self.path.clone()))
    }
}

const ITEM: &str = "_item";
const RESERVED: [&str; 5] = ["true", "false", "null", "inf", "nan"];

fn is_forbidden(character: char) -> bool {
    matches!(
        character,
        '\u{0}'..='\u{1F}'
            | '\u{7F}'
            | '\u{85}'
            | '\u{200E}'..='\u{200F}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
    )
}
