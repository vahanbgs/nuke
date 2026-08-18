use std::fmt;

use nuke_syntax::{Float, Value};

use crate::error::{Path, Segment, article, form};
use crate::table::Table;

pub type Error = crate::error::Error<ErrorKind>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    UnrepresentableRoot(&'static str),
    UnrepresentableValue(&'static str),
    UnrepresentableKey(&'static str),
    UnspellableName(String),
    EmptyString,
    EmptyList,
    UnrepresentableCharacter(char),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnrepresentableRoot(form) => write!(
                f,
                "{} {form} cannot be a Ghostty config; only a tuple or a map opens one",
                article(form)
            ),
            Self::UnrepresentableValue(form) => write!(
                f,
                "{} {form} has no Ghostty spelling; the file has no section, so an option holds a scalar or a list of them",
                article(form)
            ),
            Self::UnrepresentableKey(form) => write!(
                f,
                "{} {form} cannot name a Ghostty option; only a string or an atom names one",
                article(form)
            ),
            Self::UnspellableName(name) => write!(
                f,
                "`{name}` is not a name Ghostty could give an option; one holds lowercase letters, digits and `-`, and begins with a letter"
            ),
            Self::EmptyString => write!(
                f,
                "the empty string has no Ghostty spelling: an empty value resets its option instead of setting it"
            ),
            Self::EmptyList => write!(
                f,
                "an empty list writes no line at all, so the option holding it would be lost"
            ),
            Self::UnrepresentableCharacter(character) => write!(
                f,
                "U+{:04X} is a control, and Ghostty has no escape to write one with",
                u32::from(*character)
            ),
        }
    }
}

pub fn to_string(value: &Value) -> Result<String, Error> {
    let Some(table) = Table::of(value) else {
        return Err(Error::new(
            ErrorKind::UnrepresentableRoot(form(value)),
            Path::default(),
        ));
    };
    let mut writer = Writer::default();
    writer.document(table)?;
    Ok(writer.out)
}

enum Named<'a> {
    Field(&'a str),
    Key(&'a Value),
}

struct Entry<'a> {
    name: Named<'a>,
    value: &'a Value,
    segment: Segment,
}

#[derive(Default)]
struct Writer {
    out: String,
    path: Vec<Segment>,
}

impl Writer {
    fn document(&mut self, table: Table<'_>) -> Result<(), Error> {
        for entry in entries(table) {
            self.path.push(entry.segment.clone());
            let name = self.named(&entry.name)?;
            if !is_name(name) {
                return Err(self.error(ErrorKind::UnspellableName(name.to_owned())));
            }
            self.option(name, entry.value)?;
            self.path.pop();
        }
        Ok(())
    }

    fn option(&mut self, name: &str, value: &Value) -> Result<(), Error> {
        let Value::List(items) = value else {
            return self.pair(name, value);
        };
        if items.is_empty() {
            return Err(self.error(ErrorKind::EmptyList));
        }
        for (index, item) in items.iter().enumerate() {
            self.path.push(Segment::Index(index));
            self.pair(name, item)?;
            self.path.pop();
        }
        Ok(())
    }

    fn pair(&mut self, name: &str, value: &Value) -> Result<(), Error> {
        let spelling = self.scalar(value)?;
        if !self.out.is_empty() {
            self.out.push('\n');
        }
        self.out.push_str(name);
        self.out.push_str(PAIR);
        self.out.push_str(&spelling);
        Ok(())
    }

    fn scalar(&mut self, value: &Value) -> Result<String, Error> {
        let text = match value {
            Value::Atom(atom) => match atom.as_str() {
                "True" => TRUE,
                "False" => FALSE,
                other => other,
            },
            Value::String(text) => text.as_str(),
            Value::Integer(integer) => integer.as_str(),
            Value::Float(number) => return Ok(self.float(*number)),
            other => return Err(self.error(ErrorKind::UnrepresentableValue(form(other)))),
        };
        self.text(text)
    }

    fn text(&self, text: &str) -> Result<String, Error> {
        if text.is_empty() {
            return Err(self.error(ErrorKind::EmptyString));
        }
        if let Some(character) = text.chars().find(|character| is_control(*character)) {
            return Err(self.error(ErrorKind::UnrepresentableCharacter(character)));
        }
        if needs_fencing(text) {
            return Ok(format!("\"{text}\""));
        }
        Ok(text.to_owned())
    }

    fn float(&self, number: Float) -> String {
        let mut buffer = ryu::Buffer::new();
        buffer.format_finite(number.get()).to_owned()
    }

    fn named<'a>(&self, name: &Named<'a>) -> Result<&'a str, Error> {
        let key = match name {
            Named::Field(field) => return Ok(field),
            Named::Key(key) => key,
        };
        match key {
            Value::String(text) => Ok(text.as_str()),
            Value::Atom(atom) => Ok(atom.as_str()),
            other => Err(self.error(ErrorKind::UnrepresentableKey(form(other)))),
        }
    }

    fn error(&self, kind: ErrorKind) -> Error {
        Error::new(kind, Path::new(self.path.clone()))
    }
}

const PAIR: &str = " = ";
const TRUE: &str = "true";
const FALSE: &str = "false";

fn entries(table: Table<'_>) -> Vec<Entry<'_>> {
    match table {
        Table::Tuple(tuple) => tuple
            .iter()
            .map(|(name, value)| Entry {
                name: Named::Field(name.as_str()),
                value,
                segment: Segment::Field(name.as_str().to_owned()),
            })
            .collect(),
        Table::Map(map) => map
            .iter()
            .enumerate()
            .map(|(position, (key, value))| Entry {
                name: Named::Key(key),
                value,
                segment: Segment::Entry(position),
            })
            .collect(),
    }
}

fn is_name(name: &str) -> bool {
    name.starts_with(|character: char| character.is_ascii_lowercase())
        && name.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

fn is_control(character: char) -> bool {
    matches!(character, '\u{0}'..='\u{1F}')
}

fn needs_fencing(text: &str) -> bool {
    let fenced = text.len() > 1 && text.starts_with('"') && text.ends_with('"');
    fenced || text.starts_with(char::is_whitespace) || text.ends_with(char::is_whitespace)
}
