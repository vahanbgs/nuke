use std::collections::HashSet;
use std::fmt::{self, Write as _};

use nuke_syntax::{Atom, Float, MAX_DEPTH, Map, Tuple, Value};

use crate::error::{Path, Segment, too_deep};

pub type Error = crate::error::Error<ErrorKind>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    DuplicateKey(String),
    TooDeep,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateKey(spelling) => {
                write!(f, "two keys of this map both name `{spelling}` in YAML")
            }
            Self::TooDeep => too_deep(f),
        }
    }
}

pub fn to_string(value: &Value) -> Result<String, Error> {
    let mut writer = Writer {
        out: String::new(),
        path: Vec::new(),
    };
    writer.node(value, 0, 0)?;
    Ok(writer.out)
}

struct Writer {
    out: String,
    path: Vec<Segment>,
}

impl Writer {
    fn node(&mut self, value: &Value, indent: usize, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(self.error(ErrorKind::TooDeep));
        }
        match value {
            Value::Tuple(tuple) if !tuple.is_empty() => self.tuple(tuple, indent, depth),
            Value::Map(map) if !map.is_empty() => self.map(map, indent, depth),
            Value::List(items) if !items.is_empty() => self.list(items, indent, depth),
            other => self.flow(other, depth),
        }
    }

    fn tuple(&mut self, tuple: &Tuple, indent: usize, depth: usize) -> Result<(), Error> {
        for (position, (name, value)) in tuple.iter().enumerate() {
            if position > 0 {
                self.newline(indent);
            }
            self.word(name.as_str());
            self.path.push(Segment::Field(name.as_str().to_owned()));
            self.entry(value, indent, depth + 1)?;
            self.path.pop();
        }
        Ok(())
    }

    fn map(&mut self, map: &Map, indent: usize, depth: usize) -> Result<(), Error> {
        let mut seen = HashSet::with_capacity(map.len());
        for (position, (key, value)) in map.iter().enumerate() {
            if position > 0 {
                self.newline(indent);
            }
            self.path.push(Segment::Entry(position));
            if self.key(key, &mut seen, depth + 1)? {
                self.newline(indent);
            }
            self.entry(value, indent, depth + 1)?;
            self.path.pop();
        }
        Ok(())
    }

    fn list(&mut self, items: &[Value], indent: usize, depth: usize) -> Result<(), Error> {
        for (index, item) in items.iter().enumerate() {
            if index > 0 {
                self.newline(indent);
            }
            self.out.push_str("- ");
            self.path.push(Segment::Index(index));
            self.node(item, indent + 2, depth + 1)?;
            self.path.pop();
        }
        Ok(())
    }

    fn entry(&mut self, value: &Value, indent: usize, depth: usize) -> Result<(), Error> {
        self.out.push(':');
        if is_block(value) {
            self.newline(indent + 2);
            self.node(value, indent + 2, depth)
        } else {
            self.out.push(' ');
            self.flow(value, depth)
        }
    }

    fn key(
        &mut self,
        key: &Value,
        seen: &mut HashSet<String>,
        depth: usize,
    ) -> Result<bool, Error> {
        let explicit = matches!(key, Value::Tuple(_) | Value::Map(_) | Value::List(_));
        if explicit {
            self.out.push_str("? ");
        }
        let start = self.out.len();
        self.flow(key, depth)?;
        let spelling = self.out[start..].to_owned();

        let mut identity = String::new();
        self.identity(key, &mut identity, depth)?;
        if seen.insert(identity) {
            Ok(explicit)
        } else {
            Err(self.error(ErrorKind::DuplicateKey(spelling)))
        }
    }

    fn flow(&mut self, value: &Value, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(self.error(ErrorKind::TooDeep));
        }
        match value {
            Value::Tuple(tuple) => {
                self.out.push('{');
                for (position, (name, item)) in tuple.iter().enumerate() {
                    if position > 0 {
                        self.out.push_str(", ");
                    }
                    self.word(name.as_str());
                    self.out.push_str(": ");
                    self.flow(item, depth + 1)?;
                }
                self.out.push('}');
            }
            Value::Map(map) => {
                let mut seen = HashSet::with_capacity(map.len());
                self.out.push('{');
                for (position, (key, item)) in map.iter().enumerate() {
                    if position > 0 {
                        self.out.push_str(", ");
                    }
                    self.key(key, &mut seen, depth + 1)?;
                    self.out.push_str(": ");
                    self.flow(item, depth + 1)?;
                }
                self.out.push('}');
            }
            Value::List(items) => {
                self.out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        self.out.push_str(", ");
                    }
                    self.flow(item, depth + 1)?;
                }
                self.out.push(']');
            }
            Value::Atom(atom) => self.atom(atom),
            Value::String(text) => self.string(text),
            Value::Integer(integer) => self.out.push_str(integer.as_str()),
            Value::Float(number) => self.out.push_str(&float(*number)),
        }
        Ok(())
    }

    fn identity(&self, key: &Value, into: &mut String, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(self.error(ErrorKind::TooDeep));
        }
        match key {
            Value::Tuple(tuple) => {
                into.push('{');
                for (name, value) in tuple.iter() {
                    text(into, name.as_str());
                    into.push(':');
                    self.identity(value, into, depth + 1)?;
                    into.push(',');
                }
                into.push('}');
            }
            Value::Map(map) => {
                into.push('{');
                for (nested, value) in map.iter() {
                    self.identity(nested, into, depth + 1)?;
                    into.push(':');
                    self.identity(value, into, depth + 1)?;
                    into.push(',');
                }
                into.push('}');
            }
            Value::List(items) => {
                into.push('[');
                for item in items {
                    self.identity(item, into, depth + 1)?;
                    into.push(',');
                }
                into.push(']');
            }
            Value::Atom(atom) => match atom.as_str() {
                "True" => into.push_str("bool:true"),
                "False" => into.push_str("bool:false"),
                "Null" => into.push_str("null"),
                spelling => text(into, spelling),
            },
            Value::String(string) => text(into, string),
            Value::Integer(integer) => {
                into.push_str("int:");
                into.push_str(integer.as_str());
            }
            Value::Float(number) => {
                into.push_str("float:");
                into.push_str(&float(*number));
            }
        }
        Ok(())
    }

    fn atom(&mut self, atom: &Atom) {
        match atom.as_str() {
            "True" => self.out.push_str("true"),
            "False" => self.out.push_str("false"),
            "Null" => self.out.push_str("null"),
            spelling => self.word(spelling),
        }
    }

    fn word(&mut self, text: &str) {
        if RESOLVED.iter().any(|word| text.eq_ignore_ascii_case(word)) {
            self.string(text);
        } else {
            self.out.push_str(text);
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
                other if is_unprintable(other) => {
                    let _ = write!(self.out, "\\u{:04x}", other as u32);
                }
                other => self.out.push(other),
            }
        }
        self.out.push('"');
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

const RESOLVED: [&str; 9] = ["y", "n", "yes", "no", "on", "off", "true", "false", "null"];

fn is_block(value: &Value) -> bool {
    match value {
        Value::Tuple(tuple) => !tuple.is_empty(),
        Value::Map(map) => !map.is_empty(),
        Value::List(items) => !items.is_empty(),
        _ => false,
    }
}

fn is_unprintable(character: char) -> bool {
    matches!(
        character,
        '\u{0}'..='\u{1F}'
            | '\u{7F}'..='\u{9F}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{FEFF}'
            | '\u{FFFE}'
            | '\u{FFFF}'
    )
}

fn float(number: Float) -> String {
    let mut buffer = ryu::Buffer::new();
    let written = buffer.format_finite(number.get());
    let Some((mantissa, exponent)) = written.split_once('e') else {
        return written.to_owned();
    };
    let mut shaped = String::with_capacity(written.len() + 3);
    shaped.push_str(mantissa);
    if !mantissa.contains('.') {
        shaped.push_str(".0");
    }
    shaped.push('e');
    if !exponent.starts_with('-') {
        shaped.push('+');
    }
    shaped.push_str(exponent);
    shaped
}

fn text(into: &mut String, string: &str) {
    let _ = write!(into, "s{}:{string}", string.len());
}
