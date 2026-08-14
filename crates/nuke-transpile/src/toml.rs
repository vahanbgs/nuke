use std::collections::HashSet;
use std::fmt::{self, Write as _};

use nuke_syntax::{Atom, Float, MAX_DEPTH, Map, Tuple, Value};

use crate::error::{Path, Segment, article, form, too_deep};

pub type Error = crate::error::Error<ErrorKind>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    UnrepresentableRoot(&'static str),
    UnrepresentableKey(&'static str),
    DuplicateKey(String),
    WideInteger(String),
    TooDeep,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnrepresentableRoot(form) => write!(
                f,
                "{} {form} cannot be a TOML document; only a tuple or a map opens one",
                article(form)
            ),
            Self::UnrepresentableKey(form) => write!(
                f,
                "{} {form} cannot key a TOML table; only a string or an atom names one",
                article(form)
            ),
            Self::DuplicateKey(name) => {
                write!(f, "two keys of this map both name `{name}` in TOML")
            }
            Self::WideInteger(digits) => {
                write!(
                    f,
                    "`{digits}` does not fit the 64 bits TOML gives an integer"
                )
            }
            Self::TooDeep => too_deep(f),
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
    writer.table(table, 0)?;
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

    fn filled(value: &'a Value) -> Option<Self> {
        Self::of(value).filter(|table| !table.is_empty())
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::Tuple(tuple) => tuple.is_empty(),
            Self::Map(map) => map.is_empty(),
        }
    }
}

enum Section<'a> {
    Table(Table<'a>),
    Array(Vec<Table<'a>>),
}

impl<'a> Section<'a> {
    fn of(value: &'a Value) -> Option<Self> {
        match value {
            Value::List(items) if !items.is_empty() => items
                .iter()
                .map(Table::filled)
                .collect::<Option<_>>()
                .map(Self::Array),
            other => Table::filled(other).map(Self::Table),
        }
    }
}

struct Entry<'a> {
    key: &'a str,
    value: &'a Value,
    segment: Segment,
    section: Option<Section<'a>>,
}

#[derive(Default)]
struct Writer {
    out: String,
    path: Vec<Segment>,
    header: Vec<String>,
}

impl Writer {
    fn table(&mut self, table: Table<'_>, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(self.error(ErrorKind::TooDeep));
        }
        let entries = self.entries(table)?;
        let split = entries
            .iter()
            .rposition(|entry| entry.section.is_none())
            .map_or(0, |last| last + 1);

        for entry in &entries[..split] {
            self.line();
            self.pair(entry, depth + 1)?;
        }
        for entry in &entries[split..] {
            let Some(section) = &entry.section else {
                continue;
            };
            self.path.push(entry.segment.clone());
            self.header.push(key(entry.key));
            match section {
                Section::Table(nested) => {
                    self.open(false);
                    self.table(*nested, depth + 1)?;
                }
                Section::Array(tables) => {
                    for (index, nested) in tables.iter().enumerate() {
                        self.path.push(Segment::Index(index));
                        self.open(true);
                        self.table(*nested, depth + 2)?;
                        self.path.pop();
                    }
                }
            }
            self.header.pop();
            self.path.pop();
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
                    section: Section::of(value),
                })
                .collect()),
            Table::Map(map) => {
                let mut seen = HashSet::with_capacity(map.len());
                let mut entries = Vec::with_capacity(map.len());
                for (position, (key, value)) in map.iter().enumerate() {
                    self.path.push(Segment::Entry(position));
                    let name = self.key(key)?;
                    if !seen.insert(name) {
                        return Err(self.error(ErrorKind::DuplicateKey(name.to_owned())));
                    }
                    self.path.pop();
                    entries.push(Entry {
                        key: name,
                        value,
                        segment: Segment::Entry(position),
                        section: Section::of(value),
                    });
                }
                Ok(entries)
            }
        }
    }

    fn pair(&mut self, entry: &Entry<'_>, depth: usize) -> Result<(), Error> {
        self.out.push_str(&key(entry.key));
        self.out.push_str(" = ");
        self.path.push(entry.segment.clone());
        self.inline(entry.value, depth)?;
        self.path.pop();
        Ok(())
    }

    fn inline(&mut self, value: &Value, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(self.error(ErrorKind::TooDeep));
        }
        match value {
            Value::Tuple(tuple) => return self.inline_table(Table::Tuple(tuple), depth),
            Value::Map(map) => return self.inline_table(Table::Map(map), depth),
            Value::List(items) => return self.inline_list(items, depth),
            Value::Atom(atom) => self.atom(atom),
            Value::String(text) => escape(&mut self.out, text),
            Value::Integer(integer) => {
                if integer.to_i64().is_none() {
                    return Err(self.error(ErrorKind::WideInteger(integer.as_str().to_owned())));
                }
                self.out.push_str(integer.as_str());
            }
            Value::Float(number) => self.float(*number),
        }
        Ok(())
    }

    fn inline_table(&mut self, table: Table<'_>, depth: usize) -> Result<(), Error> {
        let entries = self.entries(table)?;
        self.out.push('{');
        for (position, entry) in entries.iter().enumerate() {
            if position > 0 {
                self.out.push_str(", ");
            }
            self.pair(entry, depth + 1)?;
        }
        self.out.push('}');
        Ok(())
    }

    fn inline_list(&mut self, items: &[Value], depth: usize) -> Result<(), Error> {
        self.out.push('[');
        for (index, item) in items.iter().enumerate() {
            if index > 0 {
                self.out.push_str(", ");
            }
            self.path.push(Segment::Index(index));
            self.inline(item, depth + 1)?;
            self.path.pop();
        }
        self.out.push(']');
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
            spelling => escape(&mut self.out, spelling),
        }
    }

    fn float(&mut self, number: Float) {
        let mut buffer = ryu::Buffer::new();
        self.out.push_str(buffer.format_finite(number.get()));
    }

    fn open(&mut self, array: bool) {
        if !self.out.is_empty() {
            self.out.push_str("\n\n");
        }
        self.out.push('[');
        if array {
            self.out.push('[');
        }
        for (position, part) in self.header.iter().enumerate() {
            if position > 0 {
                self.out.push('.');
            }
            self.out.push_str(part);
        }
        if array {
            self.out.push(']');
        }
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

fn key(name: &str) -> String {
    if !name.is_empty()
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
    {
        return name.to_owned();
    }
    let mut quoted = String::with_capacity(name.len() + 2);
    escape(&mut quoted, name);
    quoted
}

fn escape(into: &mut String, text: &str) {
    into.push('"');
    for character in text.chars() {
        match character {
            '"' => into.push_str("\\\""),
            '\\' => into.push_str("\\\\"),
            '\n' => into.push_str("\\n"),
            '\r' => into.push_str("\\r"),
            '\t' => into.push_str("\\t"),
            control if control < ' ' || control == '\u{7F}' => {
                let _ = write!(into, "\\u{:04X}", control as u32);
            }
            other => into.push(other),
        }
    }
    into.push('"');
}
