use std::collections::HashSet;
use std::fmt;

use nuke_syntax::{Atom, Float, Integer, MAX_DEPTH, Map, Tuple, Value};

use crate::error::{Path, Segment, article, form, too_deep};
use crate::xml_text;

pub type Error = crate::error::Error<ErrorKind>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    UnrepresentableKey(&'static str),
    DuplicateKey(String),
    WideInteger(String),
    UnrepresentableCharacter(char),
    TooDeep,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnrepresentableKey(form) => write!(
                f,
                "{} {form} cannot key a property list dictionary; only a string or an atom names one",
                article(form)
            ),
            Self::DuplicateKey(name) => write!(
                f,
                "two keys of this map both name `{name}` in a property list"
            ),
            Self::WideInteger(digits) => write!(
                f,
                "`{digits}` is wider than the 64 bits a property list integer holds"
            ),
            Self::UnrepresentableCharacter(character) => write!(
                f,
                "U+{:04X} is not a character XML can carry",
                u32::from(*character)
            ),
            Self::TooDeep => too_deep(f),
        }
    }
}

pub fn to_string(value: &Value) -> Result<String, Error> {
    let mut writer = Writer::default();
    writer.out.push_str(PROLOGUE);
    writer.newline(2);
    writer.value(value, 2, 0)?;
    writer.newline(0);
    writer.out.push_str(CLOSE);
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
            Value::Float(number) => {
                self.real(*number);
                Ok(())
            }
        }
    }

    fn tuple(&mut self, tuple: &Tuple, indent: usize, depth: usize) -> Result<(), Error> {
        if tuple.is_empty() {
            self.out.push_str(EMPTY_DICT);
            return Ok(());
        }
        self.out.push_str(OPEN_DICT);
        for (name, value) in tuple.iter() {
            self.path.push(Segment::Field(name.as_str().to_owned()));
            self.newline(indent + 2);
            self.key(name.as_str())?;
            self.newline(indent + 2);
            self.value(value, indent + 2, depth + 1)?;
            self.path.pop();
        }
        self.newline(indent);
        self.out.push_str(CLOSE_DICT);
        Ok(())
    }

    fn map(&mut self, map: &Map, indent: usize, depth: usize) -> Result<(), Error> {
        if map.is_empty() {
            self.out.push_str(EMPTY_DICT);
            return Ok(());
        }
        let mut seen = HashSet::with_capacity(map.len());
        self.out.push_str(OPEN_DICT);
        for (position, (key, value)) in map.iter().enumerate() {
            self.path.push(Segment::Entry(position));
            let name = self.name(key)?;
            if !seen.insert(name) {
                return Err(self.error(ErrorKind::DuplicateKey(name.to_owned())));
            }
            self.newline(indent + 2);
            self.key(name)?;
            self.newline(indent + 2);
            self.value(value, indent + 2, depth + 1)?;
            self.path.pop();
        }
        self.newline(indent);
        self.out.push_str(CLOSE_DICT);
        Ok(())
    }

    fn list(&mut self, items: &[Value], indent: usize, depth: usize) -> Result<(), Error> {
        if items.is_empty() {
            self.out.push_str(EMPTY_ARRAY);
            return Ok(());
        }
        self.out.push_str(OPEN_ARRAY);
        for (index, item) in items.iter().enumerate() {
            self.path.push(Segment::Index(index));
            self.newline(indent + 2);
            self.value(item, indent + 2, depth + 1)?;
            self.path.pop();
        }
        self.newline(indent);
        self.out.push_str(CLOSE_ARRAY);
        Ok(())
    }

    fn name<'a>(&self, key: &'a Value) -> Result<&'a str, Error> {
        match key {
            Value::String(text) => Ok(text),
            Value::Atom(atom) => Ok(atom.as_str()),
            other => Err(self.error(ErrorKind::UnrepresentableKey(form(other)))),
        }
    }

    fn key(&mut self, name: &str) -> Result<(), Error> {
        self.out.push_str(OPEN_KEY);
        self.escape(name)?;
        self.out.push_str(CLOSE_KEY);
        Ok(())
    }

    fn atom(&mut self, atom: &Atom) -> Result<(), Error> {
        match atom.as_str() {
            "True" => self.out.push_str(TRUE),
            "False" => self.out.push_str(FALSE),
            spelling => return self.string(spelling),
        }
        Ok(())
    }

    fn string(&mut self, text: &str) -> Result<(), Error> {
        self.out.push_str(OPEN_STRING);
        self.escape(text)?;
        self.out.push_str(CLOSE_STRING);
        Ok(())
    }

    fn integer(&mut self, integer: &Integer) -> Result<(), Error> {
        if integer.to_i64().is_none() {
            return Err(self.error(ErrorKind::WideInteger(integer.as_str().to_owned())));
        }
        self.out.push_str(OPEN_INTEGER);
        self.out.push_str(integer.as_str());
        self.out.push_str(CLOSE_INTEGER);
        Ok(())
    }

    fn real(&mut self, number: Float) {
        let mut buffer = ryu::Buffer::new();
        self.out.push_str(OPEN_REAL);
        self.out.push_str(buffer.format_finite(number.get()));
        self.out.push_str(CLOSE_REAL);
    }

    fn escape(&mut self, text: &str) -> Result<(), Error> {
        xml_text::escape(&mut self.out, text)
            .map_err(|forbidden| self.error(ErrorKind::UnrepresentableCharacter(forbidden)))
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

const PROLOGUE: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" ",
    "\"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n",
    "<plist version=\"1.0\">"
);
const CLOSE: &str = "</plist>";
const OPEN_DICT: &str = "<dict>";
const CLOSE_DICT: &str = "</dict>";
const EMPTY_DICT: &str = "<dict/>";
const OPEN_ARRAY: &str = "<array>";
const CLOSE_ARRAY: &str = "</array>";
const EMPTY_ARRAY: &str = "<array/>";
const OPEN_KEY: &str = "<key>";
const CLOSE_KEY: &str = "</key>";
const OPEN_STRING: &str = "<string>";
const CLOSE_STRING: &str = "</string>";
const OPEN_INTEGER: &str = "<integer>";
const CLOSE_INTEGER: &str = "</integer>";
const OPEN_REAL: &str = "<real>";
const CLOSE_REAL: &str = "</real>";
const TRUE: &str = "<true/>";
const FALSE: &str = "<false/>";
