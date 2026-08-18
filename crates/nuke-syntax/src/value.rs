use std::borrow::Borrow;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use indexmap::IndexMap;

use crate::error::{Error, ErrorKind, Span};
use crate::lexer::{NumberKind, canonical_number};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Value {
    Tuple(Tuple),
    Map(Map),
    List(Vec<Value>),
    Atom(Atom),
    String(String),
    Integer(Integer),
    Float(Float),
}

impl FromStr for Value {
    type Err = Error;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        crate::parse(source)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Atom(Box<str>);

impl Atom {
    pub(crate) fn new(text: &str) -> Self {
        Self(text.into())
    }

    pub fn parse(text: &str) -> Option<Self> {
        let mut characters = text.chars();
        let leads = characters
            .next()
            .is_some_and(|first| first.is_ascii_uppercase());
        let follows = characters.all(|rest| rest.is_ascii_alphanumeric());
        (leads && follows).then(|| Self::new(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ident(Box<str>);

impl Ident {
    pub(crate) fn new(text: &str) -> Self {
        Self(text.into())
    }

    pub fn parse(text: &str) -> Option<Self> {
        let mut characters = text.chars();
        let leads = characters
            .next()
            .is_some_and(|first| first.is_ascii_lowercase());
        let follows = characters
            .all(|rest| rest.is_ascii_lowercase() || rest.is_ascii_digit() || rest == '_');
        (leads && follows).then(|| Self::new(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for Ident {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Integer(Box<str>);

impl Integer {
    pub(crate) fn new(text: &str) -> Self {
        Self(if text == "-0" {
            "0".into()
        } else {
            text.into()
        })
    }

    pub fn parse(text: &str) -> Option<Self> {
        matches!(canonical_number(text), Some(NumberKind::Integer)).then(|| Self::new(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_i64(&self) -> Option<i64> {
        self.0.parse().ok()
    }

    pub fn to_i128(&self) -> Option<i128> {
        self.0.parse().ok()
    }
}

impl fmt::Display for Integer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Float(f64);

impl Float {
    pub(crate) fn parse(text: &str, span: Span) -> Result<Self, Error> {
        match text.parse::<f64>() {
            Ok(value) if value.is_finite() => Ok(Self(value)),
            _ => Err(Error::new(ErrorKind::FloatOutOfRange, span)),
        }
    }

    pub fn new(value: f64) -> Option<Self> {
        value.is_finite().then_some(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for Float {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for Float {}

impl Hash for Float {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.to_bits());
    }
}

impl Ord for Float {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl PartialOrd for Float {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Float {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Tuple(IndexMap<Ident, Value>);

impl Tuple {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: Ident, value: Value) -> Option<Value> {
        self.0.insert(name, value)
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.0.get(name)
    }

    pub fn take(mut self, name: &str) -> Option<Value> {
        self.0.shift_remove(name)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Ident, &Value)> {
        self.0.iter()
    }
}

impl PartialEq for Tuple {
    fn eq(&self, other: &Self) -> bool {
        self.0.iter().eq(other.0.iter())
    }
}

impl Eq for Tuple {}

impl Hash for Tuple {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0.len());
        for field in &self.0 {
            field.hash(state);
        }
    }
}

impl FromIterator<(Ident, Value)> for Tuple {
    fn from_iter<I: IntoIterator<Item = (Ident, Value)>>(fields: I) -> Self {
        Self(fields.into_iter().collect())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Map(IndexMap<Value, Value>);

impl Map {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: Value, value: Value) -> Option<Value> {
        self.0.insert(key, value)
    }

    pub fn get(&self, key: &Value) -> Option<&Value> {
        self.0.get(key)
    }

    pub fn take(mut self, key: &Value) -> Option<Value> {
        self.0.shift_remove(key)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Value, &Value)> {
        self.0.iter()
    }
}

impl PartialEq for Map {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Map {}

impl Hash for Map {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut combined = 0u64;
        for entry in &self.0 {
            let mut hasher = DefaultHasher::new();
            entry.hash(&mut hasher);
            combined ^= hasher.finish();
        }
        state.write_usize(self.0.len());
        state.write_u64(combined);
    }
}

impl FromIterator<(Value, Value)> for Map {
    fn from_iter<I: IntoIterator<Item = (Value, Value)>>(entries: I) -> Self {
        Self(entries.into_iter().collect())
    }
}
