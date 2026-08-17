use std::collections::HashSet;
use std::fmt::{self, Write as _};

use nuke_syntax::{Atom, Float, Integer, MAX_DEPTH, Map, Tuple, Value};

use crate::error::{Path, Segment, article, form, too_deep};

pub type Error = crate::error::Error<ErrorKind>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    UnrepresentableKey(&'static str),
    DuplicateKey(String),
    WideInteger(String),
    TooDeep,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnrepresentableKey(form) => write!(
                f,
                "{} {form} keys a Lua table by identity rather than by value; only a scalar keys one",
                article(form)
            ),
            Self::DuplicateKey(spelling) => {
                write!(f, "two keys of this map both index `{spelling}` in Lua")
            }
            Self::WideInteger(digits) => write!(
                f,
                "`{digits}` is past the integers every Lua holds exactly, which stop at 2^53"
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
        self.out.push_str(RETURN);
        self.value(value, 0, 0)
    }

    fn value(&mut self, value: &Value, indent: usize, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(self.error(ErrorKind::TooDeep));
        }
        match value {
            Value::Tuple(tuple) => return self.tuple(tuple, indent, depth),
            Value::Map(map) => return self.map(map, indent, depth),
            Value::List(items) => return self.list(items, indent, depth),
            Value::Atom(atom) => self.atom(atom),
            Value::String(text) => self.string(text),
            Value::Integer(integer) => {
                self.integer(integer)?;
            }
            Value::Float(number) => self.float(*number),
        }
        Ok(())
    }

    fn tuple(&mut self, tuple: &Tuple, indent: usize, depth: usize) -> Result<(), Error> {
        if tuple.is_empty() {
            self.out.push_str(EMPTY);
            return Ok(());
        }
        self.out.push('{');
        for (name, value) in tuple.iter() {
            self.path.push(Segment::Field(name.as_str().to_owned()));
            self.line(indent + 2);
            self.field(name.as_str());
            self.out.push_str(" = ");
            self.value(value, indent + 2, depth + 1)?;
            self.out.push(',');
            self.path.pop();
        }
        self.close(indent);
        Ok(())
    }

    fn map(&mut self, map: &Map, indent: usize, depth: usize) -> Result<(), Error> {
        if map.is_empty() {
            self.out.push_str(EMPTY);
            return Ok(());
        }
        let mut seen = HashSet::with_capacity(map.len());
        self.out.push('{');
        for (position, (key, value)) in map.iter().enumerate() {
            self.path.push(Segment::Entry(position));
            self.line(indent + 2);
            let start = self.out.len();
            let index = self.key(key)?;
            if !seen.insert(index) {
                return Err(self.error(ErrorKind::DuplicateKey(self.out[start..].to_owned())));
            }
            self.out.push_str(" = ");
            self.value(value, indent + 2, depth + 1)?;
            self.out.push(',');
            self.path.pop();
        }
        self.close(indent);
        Ok(())
    }

    fn list(&mut self, items: &[Value], indent: usize, depth: usize) -> Result<(), Error> {
        if items.is_empty() {
            self.out.push_str(EMPTY);
            return Ok(());
        }
        self.out.push('{');
        for (index, item) in items.iter().enumerate() {
            self.path.push(Segment::Index(index));
            self.line(indent + 2);
            self.value(item, indent + 2, depth + 1)?;
            self.out.push(',');
            self.path.pop();
        }
        self.close(indent);
        Ok(())
    }

    fn key(&mut self, key: &Value) -> Result<Index, Error> {
        self.out.push('[');
        let index = match key {
            Value::Atom(atom) => {
                self.atom(atom);
                Index::of_atom(atom)
            }
            Value::String(text) => {
                self.string(text);
                Index::Text(text.clone())
            }
            Value::Integer(integer) => Index::Integer(self.integer(integer)?),
            Value::Float(number) => {
                self.float(*number);
                Index::of_float(*number)
            }
            other => return Err(self.error(ErrorKind::UnrepresentableKey(form(other)))),
        };
        self.out.push(']');
        Ok(index)
    }

    fn field(&mut self, name: &str) {
        if RESERVED.contains(&name) {
            self.out.push('[');
            self.string(name);
            self.out.push(']');
        } else {
            self.out.push_str(name);
        }
    }

    fn atom(&mut self, atom: &Atom) {
        match atom.as_str() {
            "True" => self.out.push_str("true"),
            "False" => self.out.push_str("false"),
            spelling => self.string(spelling),
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
                control if control.is_ascii_control() => {
                    let _ = write!(self.out, "\\{:03}", u32::from(control));
                }
                other => self.out.push(other),
            }
        }
        self.out.push('"');
    }

    fn integer(&mut self, integer: &Integer) -> Result<i64, Error> {
        let exact = integer
            .to_i64()
            .filter(|value| value.unsigned_abs() <= EXACT);
        let Some(value) = exact else {
            return Err(self.error(ErrorKind::WideInteger(integer.as_str().to_owned())));
        };
        self.out.push_str(integer.as_str());
        Ok(value)
    }

    fn float(&mut self, number: Float) {
        let mut buffer = ryu::Buffer::new();
        self.out.push_str(buffer.format_finite(number.get()));
    }

    fn close(&mut self, indent: usize) {
        self.line(indent);
        self.out.push('}');
    }

    fn line(&mut self, indent: usize) {
        self.out.push('\n');
        for _ in 0..indent {
            self.out.push(' ');
        }
    }

    fn error(&self, kind: ErrorKind) -> Error {
        Error::new(kind, Path::new(self.path.clone()))
    }
}

#[derive(PartialEq, Eq, Hash)]
enum Index {
    Boolean(bool),
    Integer(i64),
    Double(u64),
    Text(String),
}

impl Index {
    fn of_atom(atom: &Atom) -> Self {
        match atom.as_str() {
            "True" => Self::Boolean(true),
            "False" => Self::Boolean(false),
            spelling => Self::Text(spelling.to_owned()),
        }
    }

    fn of_float(number: Float) -> Self {
        let value = number.get();
        if value.fract() == 0.0 && value.abs() <= EXACT as f64 {
            Self::Integer(value as i64)
        } else {
            Self::Double(value.to_bits())
        }
    }
}

const RETURN: &str = "return ";
const EMPTY: &str = "{}";
const EXACT: u64 = 1 << 53;
const RESERVED: [&str; 22] = [
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];
