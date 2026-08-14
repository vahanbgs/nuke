use std::collections::HashSet;
use std::fmt;

use nuke_syntax::{Atom, Float, MAX_DEPTH, Map, Tuple, Value};

use crate::error::{Path, Segment, article, form, too_deep};

pub type Error = crate::error::Error<ErrorKind>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    UnrepresentableKey(&'static str),
    DuplicateKey(String),
    TooDeep,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnrepresentableKey(form) => write!(
                f,
                "{} {form} cannot key a JSON object; only a string or an atom names one",
                article(form)
            ),
            Self::DuplicateKey(name) => {
                write!(f, "two keys of this map both name `{name}` in JSON")
            }
            Self::TooDeep => too_deep(f),
        }
    }
}

pub fn to_string(value: &Value) -> Result<String, Error> {
    write(value, Some(2))
}

pub fn to_string_compact(value: &Value) -> Result<String, Error> {
    write(value, None)
}

fn write(value: &Value, indent: Option<usize>) -> Result<String, Error> {
    let mut writer = Writer {
        out: String::new(),
        indent,
        path: Vec::new(),
    };
    writer.value(value, 0)?;
    Ok(writer.out)
}

struct Writer {
    out: String,
    indent: Option<usize>,
    path: Vec<Segment>,
}

impl Writer {
    fn value(&mut self, value: &Value, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(self.error(ErrorKind::TooDeep));
        }
        match value {
            Value::Tuple(tuple) => return self.tuple(tuple, depth),
            Value::Map(map) => return self.map(map, depth),
            Value::List(items) => return self.list(items, depth),
            Value::Atom(atom) => self.atom(atom),
            Value::String(text) => self.string(text),
            Value::Integer(integer) => self.out.push_str(integer.as_str()),
            Value::Float(float) => self.float(*float),
        }
        Ok(())
    }

    fn tuple(&mut self, tuple: &Tuple, depth: usize) -> Result<(), Error> {
        if tuple.is_empty() {
            self.out.push_str("{}");
            return Ok(());
        }
        self.out.push('{');
        for (position, (name, value)) in tuple.iter().enumerate() {
            self.separator(position, depth + 1);
            self.string(name.as_str());
            self.colon();
            self.path.push(Segment::Field(name.as_str().to_owned()));
            self.value(value, depth + 1)?;
            self.path.pop();
        }
        self.close('}', depth);
        Ok(())
    }

    fn map(&mut self, map: &Map, depth: usize) -> Result<(), Error> {
        if map.is_empty() {
            self.out.push_str("{}");
            return Ok(());
        }
        let mut seen = HashSet::with_capacity(map.len());
        self.out.push('{');
        for (position, (key, value)) in map.iter().enumerate() {
            self.path.push(Segment::Entry(position));
            let name = self.key(key)?;
            if !seen.insert(name) {
                return Err(self.error(ErrorKind::DuplicateKey(name.to_owned())));
            }
            self.separator(position, depth + 1);
            self.string(name);
            self.colon();
            self.value(value, depth + 1)?;
            self.path.pop();
        }
        self.close('}', depth);
        Ok(())
    }

    fn list(&mut self, items: &[Value], depth: usize) -> Result<(), Error> {
        if items.is_empty() {
            self.out.push_str("[]");
            return Ok(());
        }
        self.out.push('[');
        for (index, item) in items.iter().enumerate() {
            self.separator(index, depth + 1);
            self.path.push(Segment::Index(index));
            self.value(item, depth + 1)?;
            self.path.pop();
        }
        self.close(']', depth);
        Ok(())
    }

    fn key<'a>(&self, key: &'a Value) -> Result<&'a str, Error> {
        match key {
            Value::String(text) => Ok(text),
            Value::Atom(atom) => Ok(atom.as_str()),
            other => Err(self.error(ErrorKind::UnrepresentableKey(form(other)))),
        }
    }

    fn atom(&mut self, atom: &Atom) {
        match atom.as_str() {
            "True" => self.out.push_str("true"),
            "False" => self.out.push_str("false"),
            "Null" => self.out.push_str("null"),
            spelling => self.string(spelling),
        }
    }

    fn float(&mut self, float: Float) {
        let mut buffer = ryu::Buffer::new();
        self.out.push_str(buffer.format_finite(float.get()));
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
                control if control < ' ' => {
                    let code = control as u8;
                    self.out.push_str("\\u00");
                    self.out.push(hex(code >> 4));
                    self.out.push(hex(code & 0xF));
                }
                other => self.out.push(other),
            }
        }
        self.out.push('"');
    }

    fn separator(&mut self, position: usize, depth: usize) {
        if position > 0 {
            self.out.push(',');
        }
        self.newline(depth);
    }

    fn colon(&mut self) {
        self.out.push(':');
        if self.indent.is_some() {
            self.out.push(' ');
        }
    }

    fn close(&mut self, bracket: char, depth: usize) {
        self.newline(depth);
        self.out.push(bracket);
    }

    fn newline(&mut self, depth: usize) {
        if let Some(indent) = self.indent {
            self.out.push('\n');
            for _ in 0..indent * depth {
                self.out.push(' ');
            }
        }
    }

    fn error(&self, kind: ErrorKind) -> Error {
        Error::new(kind, Path::new(self.path.clone()))
    }
}

fn hex(nibble: u8) -> char {
    char::from(match nibble {
        0..=9 => b'0' + nibble,
        _ => b'a' + nibble - 10,
    })
}
