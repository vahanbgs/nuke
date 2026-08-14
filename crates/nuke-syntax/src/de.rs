use std::fmt;
use std::slice;

use serde::Deserialize;
use serde::de::{
    self, DeserializeOwned, DeserializeSeed, EnumAccess, Error as _, IntoDeserializer, MapAccess,
    SeqAccess, VariantAccess, Visitor,
};

use crate::error::Span;
use crate::hint;
use crate::value::{Atom, Float, Ident, Integer, Map, Tuple, Value};

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
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match name {
            hint::VALUE => match self.value {
                Value::Atom(_) => visitor.visit_map(Hint::new(hint::ATOM, self)),
                Value::Tuple(_) => visitor.visit_map(Hint::new(hint::TUPLE, self)),
                Value::Integer(integer) if wider_than_serde(integer) => {
                    visitor.visit_map(Hint::new(hint::INTEGER, self))
                }
                _ => self.deserialize_any(visitor),
            },
            hint::ATOM => match self.value {
                Value::Atom(atom) => visitor.visit_borrowed_str(atom.as_str()),
                other => Err(mismatch(other, &visitor)),
            },
            hint::INTEGER => match self.value {
                Value::Integer(integer) => visitor.visit_borrowed_str(integer.as_str()),
                other => Err(mismatch(other, &visitor)),
            },
            hint::TUPLE => match self.value {
                Value::Tuple(tuple) => visitor.visit_map(Fields::new(tuple.iter())),
                other => Err(mismatch(other, &visitor)),
            },
            _ => visitor.visit_newtype_struct(self),
        }
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

fn wider_than_serde(integer: &Integer) -> bool {
    integer.to_i128().is_none() && integer.as_str().parse::<u128>().is_err()
}

struct Hint<'de> {
    key: &'static str,
    payload: Option<Deserializer<'de>>,
}

impl<'de> Hint<'de> {
    const fn new(key: &'static str, payload: Deserializer<'de>) -> Self {
        Self {
            key,
            payload: Some(payload),
        }
    }
}

impl<'de> MapAccess<'de> for Hint<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.payload.is_none() {
            return Ok(None);
        }
        seed.deserialize(self.key.into_deserializer()).map(Some)
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.payload.take() {
            Some(payload) => seed.deserialize(payload),
            None => Err(Error::custom("a hinted value was asked for twice")),
        }
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a value")
    }

    fn visit_bool<E: de::Error>(self, state: bool) -> Result<Value, E> {
        Ok(Value::Atom(Atom::new(if state { "True" } else { "False" })))
    }

    fn visit_i64<E: de::Error>(self, number: i64) -> Result<Value, E> {
        Ok(Value::Integer(Integer::new(&number.to_string())))
    }

    fn visit_i128<E: de::Error>(self, number: i128) -> Result<Value, E> {
        Ok(Value::Integer(Integer::new(&number.to_string())))
    }

    fn visit_u64<E: de::Error>(self, number: u64) -> Result<Value, E> {
        Ok(Value::Integer(Integer::new(&number.to_string())))
    }

    fn visit_u128<E: de::Error>(self, number: u128) -> Result<Value, E> {
        Ok(Value::Integer(Integer::new(&number.to_string())))
    }

    fn visit_f64<E: de::Error>(self, number: f64) -> Result<Value, E> {
        Float::new(number).map(Value::Float).ok_or_else(|| {
            E::custom("the canonical form has no infinity and no NaN; take them from atoms")
        })
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Value, E> {
        Ok(Value::String(text.to_owned()))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Atom(Atom::new("Null")))
    }

    fn visit_none<E: de::Error>(self) -> Result<Value, E> {
        self.visit_unit()
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    fn visit_seq<A>(self, mut access: A) -> Result<Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut items = Vec::with_capacity(access.size_hint().unwrap_or_default());
        while let Some(item) = access.next_element()? {
            items.push(item);
        }
        Ok(Value::List(items))
    }

    fn visit_map<A>(self, mut access: A) -> Result<Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let Some(first) = access.next_key::<Value>()? else {
            return Ok(Value::Map(Map::new()));
        };
        if let Value::String(name) = &first {
            match name.as_str() {
                hint::ATOM => return Ok(Value::Atom(access.next_value()?)),
                hint::INTEGER => return Ok(Value::Integer(access.next_value()?)),
                hint::TUPLE => return Ok(Value::Tuple(access.next_value()?)),
                _ => {}
            }
        }
        let mut entries = Map::new();
        let value = access.next_value()?;
        entries.insert(first, value);
        while let Some((key, value)) = access.next_entry()? {
            if entries.insert(key, value).is_some() {
                return Err(de::Error::custom("this key is already in this map"));
            }
        }
        Ok(Value::Map(entries))
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(hint::VALUE, ValueVisitor)
    }
}

struct AtomVisitor;

impl<'de> Visitor<'de> for AtomVisitor {
    type Value = Atom;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an atom")
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Atom, E> {
        Atom::parse(text).ok_or_else(|| E::custom(format!("`{text}` is not the shape of an atom")))
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Atom, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(self)
    }
}

impl<'de> Deserialize<'de> for Atom {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(hint::ATOM, AtomVisitor)
    }
}

struct IntegerVisitor;

impl<'de> Visitor<'de> for IntegerVisitor {
    type Value = Integer;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an integer")
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Integer, E> {
        Integer::parse(text).ok_or_else(|| {
            E::custom(format!(
                "`{text}` is not how the canonical form spells a number"
            ))
        })
    }

    fn visit_i64<E: de::Error>(self, number: i64) -> Result<Integer, E> {
        Ok(Integer::new(&number.to_string()))
    }

    fn visit_i128<E: de::Error>(self, number: i128) -> Result<Integer, E> {
        Ok(Integer::new(&number.to_string()))
    }

    fn visit_u64<E: de::Error>(self, number: u64) -> Result<Integer, E> {
        Ok(Integer::new(&number.to_string()))
    }

    fn visit_u128<E: de::Error>(self, number: u128) -> Result<Integer, E> {
        Ok(Integer::new(&number.to_string()))
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Integer, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Deserialize<'de> for Integer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(hint::INTEGER, IntegerVisitor)
    }
}

struct FloatVisitor;

impl<'de> Visitor<'de> for FloatVisitor {
    type Value = Float;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a float")
    }

    fn visit_f64<E: de::Error>(self, number: f64) -> Result<Float, E> {
        Float::new(number).ok_or_else(|| {
            E::custom("the canonical form has no infinity and no NaN; take them from atoms")
        })
    }
}

impl<'de> Deserialize<'de> for Float {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_f64(FloatVisitor)
    }
}

struct TupleVisitor;

impl<'de> Visitor<'de> for TupleVisitor {
    type Value = Tuple;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a tuple")
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Tuple, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }

    fn visit_map<A>(self, mut access: A) -> Result<Tuple, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut fields = Tuple::new();
        while let Some((name, value)) = access.next_entry::<String, Value>()? {
            let Some(field) = Ident::parse(&name) else {
                return Err(de::Error::custom(format!(
                    "`{name}` cannot name a tuple field; a field name is an identifier"
                )));
            };
            if fields.insert(field, value).is_some() {
                return Err(de::Error::custom(format!(
                    "the field `{name}` is already in this tuple"
                )));
            }
        }
        Ok(fields)
    }
}

impl<'de> Deserialize<'de> for Tuple {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(hint::TUPLE, TupleVisitor)
    }
}

struct MapVisitor;

impl<'de> Visitor<'de> for MapVisitor {
    type Value = Map;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a map")
    }

    fn visit_map<A>(self, mut access: A) -> Result<Map, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entries = Map::new();
        while let Some((key, value)) = access.next_entry::<Value, Value>()? {
            if entries.insert(key, value).is_some() {
                return Err(de::Error::custom("this key is already in this map"));
            }
        }
        Ok(entries)
    }
}

impl<'de> Deserialize<'de> for Map {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(MapVisitor)
    }
}
