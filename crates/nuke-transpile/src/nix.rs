use std::collections::HashSet;
use std::fmt;

use nuke_syntax::{Atom, Float, Integer, MAX_DEPTH, Map, Tuple, Value};

use crate::error::{Path, Segment, article, form, too_deep};

pub type Error = crate::error::Error<ErrorKind>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    UnrepresentableKey(&'static str),
    DuplicateKey(String),
    UnrepresentableCharacter(char),
    WideInteger(String),
    SubnormalFloat(String),
    TooDeep,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnrepresentableKey(form) => write!(
                f,
                "{} {form} cannot name a Nix attribute; only a string or an atom names one",
                article(form)
            ),
            Self::DuplicateKey(name) => {
                write!(f, "two keys of this map both name `{name}` in Nix")
            }
            Self::UnrepresentableCharacter(character) => write!(
                f,
                "U+{:04X} is not a character a Nix string can hold",
                u32::from(*character)
            ),
            Self::WideInteger(digits) => write!(
                f,
                "`{digits}` is outside the integers Nix writes, which run from {LEAST} to {GREATEST}"
            ),
            Self::SubnormalFloat(digits) => write!(
                f,
                "`{digits}` is subnormal, and Nix reads no float below {SMALLEST}"
            ),
            Self::TooDeep => too_deep(f),
        }
    }
}

pub fn to_string(value: &Value) -> Result<String, Error> {
    let mut writer = Writer::default();
    writer.value(value, 0, 0)?;
    Ok(writer.out)
}

#[derive(Default)]
struct Writer {
    out: String,
    path: Vec<Segment>,
}

impl Writer {
    fn value(&mut self, value: &Value, indent: usize, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(self.error(ErrorKind::TooDeep));
        }
        match value {
            Value::Tuple(tuple) => self.tuple(tuple, indent, depth),
            Value::Map(map) => self.map(map, indent, depth),
            Value::List(items) => self.list(items, indent, depth),
            Value::Atom(atom) => self.atom(atom),
            Value::String(text) => self.string(text),
            Value::Integer(integer) => self.integer(integer),
            Value::Float(number) => self.float(*number),
        }
    }

    fn tuple(&mut self, tuple: &Tuple, indent: usize, depth: usize) -> Result<(), Error> {
        if tuple.is_empty() {
            self.out.push_str(EMPTY_SET);
            return Ok(());
        }
        self.out.push('{');
        for (name, value) in tuple.iter() {
            self.path.push(Segment::Field(name.as_str().to_owned()));
            self.line(indent + 2);
            self.field(name.as_str())?;
            self.out.push_str(" = ");
            self.value(value, indent + 2, depth + 1)?;
            self.out.push(';');
            self.path.pop();
        }
        self.close('}', indent);
        Ok(())
    }

    fn map(&mut self, map: &Map, indent: usize, depth: usize) -> Result<(), Error> {
        if map.is_empty() {
            self.out.push_str(EMPTY_SET);
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
            self.line(indent + 2);
            self.string(name)?;
            self.out.push_str(" = ");
            self.value(value, indent + 2, depth + 1)?;
            self.out.push(';');
            self.path.pop();
        }
        self.close('}', indent);
        Ok(())
    }

    fn list(&mut self, items: &[Value], indent: usize, depth: usize) -> Result<(), Error> {
        if items.is_empty() {
            self.out.push_str(EMPTY_LIST);
            return Ok(());
        }
        self.out.push('[');
        for (index, item) in items.iter().enumerate() {
            self.path.push(Segment::Index(index));
            self.line(indent + 2);
            self.element(item, indent + 2, depth + 1)?;
            self.path.pop();
        }
        self.close(']', indent);
        Ok(())
    }

    fn element(&mut self, item: &Value, indent: usize, depth: usize) -> Result<(), Error> {
        if !is_negative(item) {
            return self.value(item, indent, depth);
        }
        self.out.push('(');
        self.value(item, indent, depth)?;
        self.out.push(')');
        Ok(())
    }

    fn key<'a>(&self, key: &'a Value) -> Result<&'a str, Error> {
        match key {
            Value::String(text) => Ok(text),
            Value::Atom(atom) => Ok(atom.as_str()),
            other => Err(self.error(ErrorKind::UnrepresentableKey(form(other)))),
        }
    }

    fn field(&mut self, name: &str) -> Result<(), Error> {
        if RESERVED.contains(&name) {
            return self.string(name);
        }
        self.out.push_str(name);
        Ok(())
    }

    fn atom(&mut self, atom: &Atom) -> Result<(), Error> {
        match atom.as_str() {
            "True" => self.out.push_str("true"),
            "False" => self.out.push_str("false"),
            "Null" => self.out.push_str("null"),
            spelling => return self.string(spelling),
        }
        Ok(())
    }

    fn string(&mut self, text: &str) -> Result<(), Error> {
        self.out.push('"');
        let mut characters = text.chars().peekable();
        while let Some(character) = characters.next() {
            match character {
                '\0' => return Err(self.error(ErrorKind::UnrepresentableCharacter(character))),
                '"' => self.out.push_str("\\\""),
                '\\' => self.out.push_str("\\\\"),
                '\n' => self.out.push_str("\\n"),
                '\r' => self.out.push_str("\\r"),
                '\t' => self.out.push_str("\\t"),
                '$' if characters.peek() == Some(&'{') => self.out.push_str("\\$"),
                other => self.out.push(other),
            }
        }
        self.out.push('"');
        Ok(())
    }

    fn integer(&mut self, integer: &Integer) -> Result<(), Error> {
        if integer.to_i64().is_none_or(|value| value == i64::MIN) {
            return Err(self.error(ErrorKind::WideInteger(integer.as_str().to_owned())));
        }
        self.out.push_str(integer.as_str());
        Ok(())
    }

    fn float(&mut self, number: Float) -> Result<(), Error> {
        let mut buffer = ryu::Buffer::new();
        let written = buffer.format_finite(number.get());
        if number.get().is_subnormal() {
            return Err(self.error(ErrorKind::SubnormalFloat(written.to_owned())));
        }
        let Some((mantissa, exponent)) = written.split_once('e') else {
            self.out.push_str(written);
            return Ok(());
        };
        self.out.push_str(mantissa);
        if !mantissa.contains('.') {
            self.out.push_str(".0");
        }
        self.out.push('e');
        self.out.push_str(exponent);
        Ok(())
    }

    fn close(&mut self, bracket: char, indent: usize) {
        self.line(indent);
        self.out.push(bracket);
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

fn is_negative(value: &Value) -> bool {
    match value {
        Value::Integer(integer) => integer.as_str().starts_with('-'),
        Value::Float(number) => number.get().is_sign_negative(),
        _ => false,
    }
}

const EMPTY_SET: &str = "{ }";
const EMPTY_LIST: &str = "[ ]";
const LEAST: i64 = -i64::MAX;
const GREATEST: i64 = i64::MAX;
const SMALLEST: f64 = f64::MIN_POSITIVE;
const RESERVED: [&str; 9] = [
    "assert", "else", "if", "in", "inherit", "let", "rec", "then", "with",
];
