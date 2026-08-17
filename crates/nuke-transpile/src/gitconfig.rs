use std::collections::HashSet;
use std::fmt;

use nuke_syntax::{Float, Map, Tuple, Value};

use crate::error::{Path, Segment, article, form};

pub type Error = crate::error::Error<ErrorKind>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    UnrepresentableRoot(&'static str),
    SectionlessKey(String),
    UnrepresentableValue(&'static str),
    EmptyList,
    UnrepresentableKey(&'static str),
    UnrepresentableCharacter(char),
    UnspellableName(String),
    DuplicateKey(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnrepresentableRoot(form) => write!(
                f,
                "{} {form} cannot be a git config file; only a tuple or a map opens one",
                article(form)
            ),
            Self::SectionlessKey(name) => write!(
                f,
                "`{name}` stands outside a section, and git can name no variable that is not in one"
            ),
            Self::UnrepresentableValue(form) => write!(
                f,
                "{} {form} has no git config spelling here; a section holds variables and subsections, and a variable holds scalars",
                article(form)
            ),
            Self::EmptyList => write!(
                f,
                "an empty list writes no line at all, so the variable holding it would be lost"
            ),
            Self::UnrepresentableKey(form) => write!(
                f,
                "{} {form} cannot name a git section, subsection or variable; only a string or an atom names one",
                article(form)
            ),
            Self::UnrepresentableCharacter(character) => write!(
                f,
                "U+{:04X} is a control, and a newline and a tab are the only ones with a spelling here",
                u32::from(*character)
            ),
            Self::UnspellableName(name) => write!(
                f,
                "`{name}` is not a name git can give a section or a variable"
            ),
            Self::DuplicateKey(name) => {
                write!(f, "two keys of this map both name `{name}` in git")
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

#[derive(Clone, Copy)]
enum Named<'a> {
    Field(&'a str),
    Key(&'a Value),
}

struct Entry<'a> {
    name: Named<'a>,
    value: &'a Value,
    segment: Segment,
    table: Option<Table<'a>>,
}

#[derive(PartialEq, Eq, Hash)]
enum Fold {
    Variable(String),
    Subsection(String),
}

#[derive(Default)]
struct Writer {
    out: String,
    path: Vec<Segment>,
}

impl Writer {
    fn document(&mut self, table: Table<'_>) -> Result<(), Error> {
        let mut seen = HashSet::new();
        for entry in entries(table) {
            self.path.push(entry.segment.clone());
            let name = self.named(entry.name)?;
            let Some(section) = entry.table else {
                return Err(self.error(ErrorKind::SectionlessKey(name.to_owned())));
            };
            if let Some(kind) = fault_in_section(name) {
                return Err(self.error(kind));
            }
            if !seen.insert(name.to_ascii_lowercase()) {
                return Err(self.error(ErrorKind::DuplicateKey(name.to_owned())));
            }
            self.section(name, section)?;
            self.path.pop();
        }
        Ok(())
    }

    fn section(&mut self, section: &str, table: Table<'_>) -> Result<(), Error> {
        let entries = entries(table);
        if entries.is_empty() {
            self.header(section, None);
            return Ok(());
        }
        let mut seen = HashSet::with_capacity(entries.len());
        let mut open = false;
        for entry in entries {
            self.path.push(entry.segment.clone());
            let name = self.named(entry.name)?;
            match entry.table {
                Some(subsection) => {
                    if let Some(kind) = fault_in_subsection(name) {
                        return Err(self.error(kind));
                    }
                    if !seen.insert(Fold::Subsection(name.to_owned())) {
                        return Err(self.error(ErrorKind::DuplicateKey(name.to_owned())));
                    }
                    self.header(section, Some(name));
                    open = false;
                    self.subsection(subsection)?;
                }
                None => {
                    if let Some(kind) = fault_in_variable(name) {
                        return Err(self.error(kind));
                    }
                    if !seen.insert(Fold::Variable(name.to_ascii_lowercase())) {
                        return Err(self.error(ErrorKind::DuplicateKey(name.to_owned())));
                    }
                    if !open {
                        self.header(section, None);
                        open = true;
                    }
                    self.variable(name, entry.value)?;
                }
            }
            self.path.pop();
        }
        Ok(())
    }

    fn subsection(&mut self, table: Table<'_>) -> Result<(), Error> {
        let entries = entries(table);
        let mut seen = HashSet::with_capacity(entries.len());
        for entry in entries {
            self.path.push(entry.segment.clone());
            let name = self.named(entry.name)?;
            if entry.table.is_some() {
                return Err(self.error(ErrorKind::UnrepresentableValue(form(entry.value))));
            }
            if let Some(kind) = fault_in_variable(name) {
                return Err(self.error(kind));
            }
            if !seen.insert(name.to_ascii_lowercase()) {
                return Err(self.error(ErrorKind::DuplicateKey(name.to_owned())));
            }
            self.variable(name, entry.value)?;
            self.path.pop();
        }
        Ok(())
    }

    fn variable(&mut self, name: &str, value: &Value) -> Result<(), Error> {
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
        self.out.push_str(INDENT);
        self.out.push_str(name);
        self.out.push_str(PAIR);
        self.scalar(value)
    }

    fn scalar(&mut self, value: &Value) -> Result<(), Error> {
        let text = match value {
            Value::Atom(atom) => atom.as_str(),
            Value::String(text) => text.as_str(),
            Value::Integer(integer) => integer.as_str(),
            Value::Float(number) => {
                self.float(*number);
                return Ok(());
            }
            other => return Err(self.error(ErrorKind::UnrepresentableValue(form(other)))),
        };
        self.text(text)
    }

    fn text(&mut self, text: &str) -> Result<(), Error> {
        if let Some(character) = text.chars().find(|character| is_unspellable(*character)) {
            return Err(self.error(ErrorKind::UnrepresentableCharacter(character)));
        }
        if !text.is_empty() && text.chars().all(is_bare) {
            self.out.push_str(text);
            return Ok(());
        }
        self.out.push('"');
        for character in text.chars() {
            match character {
                '"' => self.out.push_str("\\\""),
                '\\' => self.out.push_str("\\\\"),
                '\n' => self.out.push_str("\\n"),
                '\t' => self.out.push_str("\\t"),
                other => self.out.push(other),
            }
        }
        self.out.push('"');
        Ok(())
    }

    fn float(&mut self, number: Float) {
        let mut buffer = ryu::Buffer::new();
        self.out.push_str(buffer.format_finite(number.get()));
    }

    fn header(&mut self, section: &str, subsection: Option<&str>) {
        if !self.out.is_empty() {
            self.out.push_str("\n\n");
        }
        self.out.push('[');
        self.out.push_str(section);
        if let Some(name) = subsection {
            self.out.push_str(" \"");
            for character in name.chars() {
                if matches!(character, '"' | '\\') {
                    self.out.push('\\');
                }
                self.out.push(character);
            }
            self.out.push('"');
        }
        self.out.push(']');
    }

    fn named<'a>(&self, name: Named<'a>) -> Result<&'a str, Error> {
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
const INDENT: &str = "\n\t";

fn entries(table: Table<'_>) -> Vec<Entry<'_>> {
    match table {
        Table::Tuple(tuple) => tuple
            .iter()
            .map(|(name, value)| Entry {
                name: Named::Field(name.as_str()),
                value,
                segment: Segment::Field(name.as_str().to_owned()),
                table: Table::of(value),
            })
            .collect(),
        Table::Map(map) => map
            .iter()
            .enumerate()
            .map(|(position, (key, value))| Entry {
                name: Named::Key(key),
                value,
                segment: Segment::Entry(position),
                table: Table::of(value),
            })
            .collect(),
    }
}

fn fault_in_section(name: &str) -> Option<ErrorKind> {
    let spellable = !name.is_empty() && name.chars().all(is_name);
    (!spellable).then(|| ErrorKind::UnspellableName(name.to_owned()))
}

fn fault_in_variable(name: &str) -> Option<ErrorKind> {
    let spellable = name.starts_with(|character: char| character.is_ascii_alphabetic())
        && name.chars().all(is_name);
    (!spellable).then(|| ErrorKind::UnspellableName(name.to_owned()))
}

fn fault_in_subsection(name: &str) -> Option<ErrorKind> {
    name.chars()
        .find(|character| matches!(character, '\u{0}'..='\u{1F}'))
        .map(ErrorKind::UnrepresentableCharacter)
}

fn is_name(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '-'
}

fn is_bare(character: char) -> bool {
    !character.is_whitespace() && !matches!(character, '#' | ';' | '"' | '\\')
}

fn is_unspellable(character: char) -> bool {
    matches!(character, '\u{0}'..='\u{8}' | '\u{B}'..='\u{1F}')
}
