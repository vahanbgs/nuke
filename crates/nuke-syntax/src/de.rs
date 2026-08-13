use std::fmt;
use std::slice;

use serde::Deserialize;
use serde::de::{
    self, DeserializeOwned, DeserializeSeed, EnumAccess, Error as _, IntoDeserializer, MapAccess,
    SeqAccess, VariantAccess, Visitor,
};

use crate::error::Span;
use crate::value::{Ident, Integer, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Syntax(crate::Error),
    Mismatch { found: String, expected: String },
    IntegerOutOfRange { text: String, wanted: &'static str },
    Message(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(error) => write!(f, "{error}"),
            Self::Mismatch { found, expected } => write!(f, "expected {expected}, found {found}"),
            Self::IntegerOutOfRange { text, wanted } => {
                write!(f, "`{text}` is past what a {wanted} holds")
            }
            Self::Message(message) => f.write_str(message),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn span(&self) -> Option<Span> {
        match &self.kind {
            ErrorKind::Syntax(error) => Some(error.span()),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for Error {}

impl From<crate::Error> for Error {
    fn from(error: crate::Error) -> Self {
        Self::new(ErrorKind::Syntax(error))
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::new(ErrorKind::Message(message.to_string()))
    }
}

pub fn from_value<'de, T>(value: &'de Value) -> Result<T, Error>
where
    T: Deserialize<'de>,
{
    T::deserialize(Deserializer::new(value))
}

pub fn from_str<T>(source: &str) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let value = crate::parse(source)?;
    from_value(&value)
}

fn describe(value: &Value) -> String {
    match value {
        Value::Tuple(tuple) if tuple.is_empty() => "an empty block".to_owned(),
        Value::Tuple(_) => "a tuple".to_owned(),
        Value::Map(_) => "a map".to_owned(),
        Value::List(_) => "a list".to_owned(),
        Value::Atom(atom) => format!("the atom `{atom}`"),
        Value::String(text) => format!("the string `{text}`"),
        Value::Integer(integer) => format!("the integer `{integer}`"),
        Value::Float(float) => format!("the float `{float}`"),
    }
}

fn mismatch(found: &Value, expected: &dyn de::Expected) -> Error {
    Error::new(ErrorKind::Mismatch {
        found: describe(found),
        expected: expected.to_string(),
    })
}

fn out_of_range(integer: &Integer, wanted: &'static str) -> Error {
    Error::new(ErrorKind::IntegerOutOfRange {
        text: integer.as_str().to_owned(),
        wanted,
    })
}

fn widest<'de, V>(integer: &Integer, visitor: V) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let text = integer.as_str();
    if let Ok(number) = text.parse::<i64>() {
        visitor.visit_i64(number)
    } else if let Ok(number) = text.parse::<u64>() {
        visitor.visit_u64(number)
    } else if let Ok(number) = text.parse::<i128>() {
        visitor.visit_i128(number)
    } else if let Ok(number) = text.parse::<u128>() {
        visitor.visit_u128(number)
    } else {
        Err(out_of_range(integer, "u128"))
    }
}

pub struct Deserializer<'de> {
    value: &'de Value,
}

impl<'de> Deserializer<'de> {
    pub const fn new(value: &'de Value) -> Self {
        Self { value }
    }
}

impl<'de> IntoDeserializer<'de, Error> for &'de Value {
    type Deserializer = Deserializer<'de>;

    fn into_deserializer(self) -> Self::Deserializer {
        Deserializer::new(self)
    }
}

macro_rules! integers {
    ($($method:ident => $visit:ident as $target:ty),* $(,)?) => {
        $(
            fn $method<V>(self, visitor: V) -> Result<V::Value, Error>
            where
                V: Visitor<'de>,
            {
                match self.value {
                    Value::Integer(integer) => integer
                        .as_str()
                        .parse::<$target>()
                        .map_err(|_| out_of_range(integer, stringify!($target)))
                        .and_then(|number| visitor.$visit(number)),
                    other => Err(mismatch(other, &visitor)),
                }
            }
        )*
    };
}

impl<'de> serde::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Atom(atom) => match atom.as_str() {
                "True" => visitor.visit_bool(true),
                "False" => visitor.visit_bool(false),
                text => visitor.visit_borrowed_str(text),
            },
            Value::String(text) => visitor.visit_borrowed_str(text),
            Value::Integer(integer) => widest(integer, visitor),
            Value::Float(float) => visitor.visit_f64(float.get()),
            Value::List(items) => visitor.visit_seq(Elements::new(items)),
            Value::Tuple(tuple) => visitor.visit_map(Fields::new(tuple.iter())),
            Value::Map(map) => visitor.visit_map(Entries::new(map.iter())),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Atom(atom) => match atom.as_str() {
                "True" => visitor.visit_bool(true),
                "False" => visitor.visit_bool(false),
                _ => Err(mismatch(self.value, &visitor)),
            },
            other => Err(mismatch(other, &visitor)),
        }
    }

    integers! {
        deserialize_i8 => visit_i8 as i8,
        deserialize_i16 => visit_i16 as i16,
        deserialize_i32 => visit_i32 as i32,
        deserialize_i64 => visit_i64 as i64,
        deserialize_i128 => visit_i128 as i128,
        deserialize_u8 => visit_u8 as u8,
        deserialize_u16 => visit_u16 as u16,
        deserialize_u32 => visit_u32 as u32,
        deserialize_u64 => visit_u64 as u64,
        deserialize_u128 => visit_u128 as u128,
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Float(float) => visitor.visit_f64(float.get()),
            other => Err(mismatch(other, &visitor)),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(text) => {
                let mut characters = text.chars();
                match (characters.next(), characters.next()) {
                    (Some(character), None) => visitor.visit_char(character),
                    _ => Err(mismatch(self.value, &visitor)),
                }
            }
            other => Err(mismatch(other, &visitor)),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(text) => visitor.visit_borrowed_str(text),
            other => Err(mismatch(other, &visitor)),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::List(items) => {
                let bytes = items
                    .iter()
                    .map(|item| match item {
                        Value::Integer(integer) => integer
                            .as_str()
                            .parse::<u8>()
                            .map_err(|_| out_of_range(integer, "u8")),
                        other => Err(mismatch(other, &"a byte")),
                    })
                    .collect::<Result<Vec<u8>, Error>>()?;
                visitor.visit_byte_buf(bytes)
            }
            other => Err(mismatch(other, &visitor)),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Tuple(tuple) if tuple.is_empty() => visitor.visit_unit(),
            Value::Map(map) if map.is_empty() => visitor.visit_unit(),
            other => Err(mismatch(other, &visitor)),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::List(items) => visitor.visit_seq(Elements::new(items)),
            other => Err(mismatch(other, &visitor)),
        }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::List(items) if items.len() == len => visitor.visit_seq(Elements::new(items)),
            Value::List(items) => Err(Error::invalid_length(items.len(), &visitor)),
            other => Err(mismatch(other, &visitor)),
        }
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Map(map) => visitor.visit_map(Entries::new(map.iter())),
            Value::Tuple(tuple) => visitor.visit_map(Fields::new(tuple.iter())),
            other => Err(mismatch(other, &visitor)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Tuple(tuple) => visitor.visit_map(Fields::new(tuple.iter())),
            other => Err(mismatch(other, &visitor)),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Atom(atom) => visitor.visit_enum(atom.as_str().into_deserializer()),
            Value::String(text) => visitor.visit_enum(text.as_str().into_deserializer()),
            Value::Map(map) => {
                let mut entries = map.iter();
                match (entries.next(), entries.next()) {
                    (Some((tag, payload)), None) => visitor.visit_enum(Variant { tag, payload }),
                    _ => Err(mismatch(self.value, &visitor)),
                }
            }
            other => Err(mismatch(other, &visitor)),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Atom(atom) => visitor.visit_borrowed_str(atom.as_str()),
            Value::String(text) => visitor.visit_borrowed_str(text),
            other => Err(mismatch(other, &visitor)),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

struct Elements<'de> {
    items: slice::Iter<'de, Value>,
}

impl<'de> Elements<'de> {
    fn new(items: &'de [Value]) -> Self {
        Self {
            items: items.iter(),
        }
    }
}

impl<'de> SeqAccess<'de> for Elements<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.items
            .next()
            .map(|item| seed.deserialize(Deserializer::new(item)))
            .transpose()
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.items.len())
    }
}

struct Fields<'de, I> {
    fields: I,
    value: Option<&'de Value>,
}

impl<'de, I> Fields<'de, I> {
    const fn new(fields: I) -> Self {
        Self {
            fields,
            value: None,
        }
    }
}

impl<'de, I> MapAccess<'de> for Fields<'de, I>
where
    I: Iterator<Item = (&'de Ident, &'de Value)>,
{
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.fields.next() {
            Some((name, value)) => {
                self.value = Some(value);
                seed.deserialize(name.as_str().into_deserializer())
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(Deserializer::new(value)),
            None => Err(Error::custom("a field value was asked for before its name")),
        }
    }
}

struct Entries<'de, I> {
    entries: I,
    value: Option<&'de Value>,
}

impl<'de, I> Entries<'de, I> {
    const fn new(entries: I) -> Self {
        Self {
            entries,
            value: None,
        }
    }
}

impl<'de, I> MapAccess<'de> for Entries<'de, I>
where
    I: Iterator<Item = (&'de Value, &'de Value)>,
{
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.entries.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(Deserializer::new(key)).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(Deserializer::new(value)),
            None => Err(Error::custom("an entry value was asked for before its key")),
        }
    }
}

struct Variant<'de> {
    tag: &'de Value,
    payload: &'de Value,
}

impl<'de> EnumAccess<'de> for Variant<'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self), Error>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(Deserializer::new(self.tag))
            .map(|tag| (tag, self))
    }
}

impl<'de> VariantAccess<'de> for Variant<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        Err(Error::custom(
            "a unit variant is written as a bare atom, not as a single entry map",
        ))
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(Deserializer::new(self.payload))
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        serde::Deserializer::deserialize_tuple(Deserializer::new(self.payload), len, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        serde::Deserializer::deserialize_struct(
            Deserializer::new(self.payload),
            "",
            fields,
            visitor,
        )
    }
}
