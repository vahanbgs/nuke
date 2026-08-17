use std::collections::HashSet;
use std::fmt;

use nuke_syntax::{Float, Map, Tuple, Value};

use crate::error::{Path, Segment, article, form};

pub type Error = crate::error::Error<ErrorKind>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    UnrepresentableRoot(&'static str),
    UnrepresentableValue(&'static str),
    UnrepresentableKey(&'static str),
    UnrepresentableCharacter(char),
    UnspellableName(String),
    UnspellableValue(String),
    StrayKey(String),
    DuplicateKey(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnrepresentableRoot(form) => write!(
                f,
                "{} {form} cannot be an INI document; only a tuple or a map opens one",
                article(form)
            ),
            Self::UnrepresentableValue(form) => write!(
                f,
                "{} {form} has no INI spelling; a section holds scalars and nothing holds a list",
                article(form)
            ),
            Self::UnrepresentableKey(form) => write!(
                f,
                "{} {form} cannot name an INI section or key; only a string or an atom names one",
                article(form)
            ),
            Self::UnrepresentableCharacter(character) => write!(
                f,
                "U+{:04X} is a character INI has no escape for",
                u32::from(*character)
            ),
            Self::UnspellableName(name) => {
                write!(f, "`{name}` is not a name INI can give a section or a key")
            }
            Self::UnspellableValue(text) => {
                write!(f, "`{text}` is not a value INI can write literally")
            }
            Self::StrayKey(name) => write!(
                f,
                "`{name}` stands after a section, where a reader takes it for one of its keys"
            ),
            Self::DuplicateKey(name) => {
                write!(f, "two keys of this map both name `{name}` in INI")
            }
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

#[derive(Clone, Copy)]
enum Table<'a> {
    Tuple(&'a Tuple),
    Map(&'a Map),
}

impl<'a> Table<'a> {
    fn of(value: &'a Value) -> Option<Self> {
        match value {
            Value::Tuple(tuple) => Some(Self::Tuple(tuple)),
            Value::Map(map) => Some(Self::Map(map)),
            _ => None,
        }
    }
}

struct Entry<'a> {
    key: &'a str,
    value: &'a Value,
    segment: Segment,
    section: Option<Table<'a>>,
}

#[derive(Default)]
struct Writer {
    out: String,
    path: Vec<Segment>,
}

impl Writer {
    fn document(&mut self, table: Table<'_>) -> Result<(), Error> {
        let entries = self.entries(table)?;
        let split = entries
            .iter()
            .position(|entry| entry.section.is_some())
            .unwrap_or(entries.len());

        for entry in &entries[..split] {
            self.line();
            self.pair(entry)?;
        }
        for entry in &entries[split..] {
            let Some(section) = entry.section else {
                self.path.push(entry.segment.clone());
                return Err(match entry.value {
                    Value::List(_) => {
                        self.error(ErrorKind::UnrepresentableValue(form(entry.value)))
                    }
                    _ => self.error(ErrorKind::StrayKey(entry.key.to_owned())),
                });
            };
            self.header(entry.key);
            self.path.push(entry.segment.clone());
            self.section(section)?;
            self.path.pop();
        }
        Ok(())
    }

    fn section(&mut self, table: Table<'_>) -> Result<(), Error> {
        let entries = self.entries(table)?;
        for entry in &entries {
            self.line();
            self.pair(entry)?;
        }
        Ok(())
    }

    fn entries<'a>(&mut self, table: Table<'a>) -> Result<Vec<Entry<'a>>, Error> {
        match table {
            Table::Tuple(tuple) => Ok(tuple
                .iter()
                .map(|(name, value)| Entry {
                    key: name.as_str(),
                    value,
                    segment: Segment::Field(name.as_str().to_owned()),
                    section: Table::of(value),
                })
                .collect()),
            Table::Map(map) => {
                let mut seen = HashSet::with_capacity(map.len());
                let mut entries = Vec::with_capacity(map.len());
                for (position, (key, value)) in map.iter().enumerate() {
                    self.path.push(Segment::Entry(position));
                    let name = self.name(key)?;
                    if !seen.insert(name.to_ascii_lowercase()) {
                        return Err(self.error(ErrorKind::DuplicateKey(name.to_owned())));
                    }
                    self.path.pop();
                    entries.push(Entry {
                        key: name,
                        value,
                        segment: Segment::Entry(position),
                        section: Table::of(value),
                    });
                }
                Ok(entries)
            }
        }
    }

    fn name<'a>(&self, key: &'a Value) -> Result<&'a str, Error> {
        let name = match key {
            Value::String(text) => text.as_str(),
            Value::Atom(atom) => atom.as_str(),
            other => return Err(self.error(ErrorKind::UnrepresentableKey(form(other)))),
        };
        match fault_in_name(name) {
            Some(kind) => Err(self.error(kind)),
            None => Ok(name),
        }
    }

    fn pair(&mut self, entry: &Entry<'_>) -> Result<(), Error> {
        self.path.push(entry.segment.clone());
        self.out.push_str(entry.key);
        self.out.push_str(PAIR);
        let start = self.out.len();
        self.scalar(entry.value)?;
        if self.out.len() == start {
            self.out.pop();
        }
        self.path.pop();
        Ok(())
    }

    fn scalar(&mut self, value: &Value) -> Result<(), Error> {
        match value {
            Value::Atom(atom) => self.out.push_str(atom.as_str()),
            Value::String(text) => {
                if let Some(kind) = fault_in_value(text) {
                    return Err(self.error(kind));
                }
                self.out.push_str(text);
            }
            Value::Integer(integer) => self.out.push_str(integer.as_str()),
            Value::Float(number) => self.float(*number),
            other => return Err(self.error(ErrorKind::UnrepresentableValue(form(other)))),
        }
        Ok(())
    }

    fn float(&mut self, number: Float) {
        let mut buffer = ryu::Buffer::new();
        self.out.push_str(buffer.format_finite(number.get()));
    }

    fn header(&mut self, name: &str) {
        if !self.out.is_empty() {
            self.out.push_str("\n\n");
        }
        self.out.push('[');
        self.out.push_str(name);
        self.out.push(']');
    }

    fn line(&mut self) {
        if !self.out.is_empty() {
            self.out.push('\n');
        }
    }

    fn error(&self, kind: ErrorKind) -> Error {
        Error::new(kind, Path::new(self.path.clone()))
    }
}

const PAIR: &str = " = ";
const INHERITED: &str = "DEFAULT";

fn fault_in_value(text: &str) -> Option<ErrorKind> {
    if let Some(character) = text.chars().find(|character| is_forbidden(*character)) {
        return Some(ErrorKind::UnrepresentableCharacter(character));
    }
    let trimmed = text.starts_with(char::is_whitespace) || text.ends_with(char::is_whitespace);
    let quoted = text.starts_with(['"', '\'']);
    let commented = text.contains(" #") || text.contains(" ;");
    (trimmed || quoted || commented).then(|| ErrorKind::UnspellableValue(text.to_owned()))
}

fn fault_in_name(name: &str) -> Option<ErrorKind> {
    match fault_in_value(name) {
        Some(ErrorKind::UnspellableValue(text)) => return Some(ErrorKind::UnspellableName(text)),
        Some(other) => return Some(other),
        None => {}
    }
    let barred = name.is_empty()
        || name.contains(['[', ']', '=', ':'])
        || name.starts_with(['#', ';'])
        || name == INHERITED;
    barred.then(|| ErrorKind::UnspellableName(name.to_owned()))
}

fn is_forbidden(character: char) -> bool {
    matches!(
        character,
        '\u{0}'..='\u{1F}' | '\u{7F}' | '\u{85}' | '\u{2028}' | '\u{2029}' | '\\'
    )
}
