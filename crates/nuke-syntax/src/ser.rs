use std::fmt;

use serde::Serialize;
use serde::ser::{
    self, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};

use crate::MAX_DEPTH;
use crate::value::{Atom, Float, Ident, Integer, Map, Tuple, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    NotFinite,
    Absent(&'static str),
    FieldName(String),
    DuplicateField(String),
    DuplicateKey,
    TooDeep,
    Message(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFinite => write!(
                f,
                "a NaN or an infinity has no canonical spelling; take them from atoms instead"
            ),
            Self::Absent(position) => write!(
                f,
                "no value stands for absence, so a `None` cannot be written {position}; \
                 only a tuple field can be left out"
            ),
            Self::FieldName(name) => write!(
                f,
                "`{name}` cannot name a tuple field; a field name is an identifier"
            ),
            Self::DuplicateField(name) => {
                write!(f, "the field `{name}` is already in this tuple")
            }
            Self::DuplicateKey => write!(f, "this key is already in this map"),
            Self::TooDeep => write!(f, "this value nests deeper than {MAX_DEPTH} levels"),
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
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::new(ErrorKind::Message(message.to_string()))
    }
}

pub fn to_value<T>(value: &T) -> Result<Value, Error>
where
    T: Serialize + ?Sized,
{
    match value.serialize(Serializer::new())? {
        Some(value) => Ok(value),
        None => Err(absent("at the top of a document")),
    }
}

fn absent(position: &'static str) -> Error {
    Error::new(ErrorKind::Absent(position))
}

fn present(value: Option<Value>, position: &'static str) -> Result<Value, Error> {
    value.ok_or_else(|| absent(position))
}

fn tag(variant: &str) -> Value {
    Atom::parse(variant).map_or_else(|| Value::String(variant.to_owned()), Value::Atom)
}

fn tagged(tag: Value, payload: Value) -> Value {
    let mut map = Map::new();
    map.insert(tag, payload);
    Value::Map(map)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Serializer {
    depth: usize,
}

impl Serializer {
    pub const fn new() -> Self {
        Self { depth: 0 }
    }

    fn descend(self) -> Result<Self, Error> {
        let depth = self.depth + 1;
        if depth > MAX_DEPTH {
            Err(Error::new(ErrorKind::TooDeep))
        } else {
            Ok(Self { depth })
        }
    }
}

macro_rules! integers {
    ($($method:ident as $target:ty),* $(,)?) => {
        $(
            fn $method(self, number: $target) -> Result<Option<Value>, Error> {
                Ok(Some(Value::Integer(Integer::new(&number.to_string()))))
            }
        )*
    };
}

impl serde::Serializer for Serializer {
    type Ok = Option<Value>;
    type Error = Error;
    type SerializeSeq = Elements;
    type SerializeTuple = Elements;
    type SerializeTupleStruct = Elements;
    type SerializeTupleVariant = TaggedElements;
    type SerializeMap = Entries;
    type SerializeStruct = Fields;
    type SerializeStructVariant = TaggedFields;

    fn serialize_bool(self, state: bool) -> Result<Option<Value>, Error> {
        let atom = if state { "True" } else { "False" };
        Ok(Some(Value::Atom(Atom::new(atom))))
    }

    integers! {
        serialize_i8 as i8,
        serialize_i16 as i16,
        serialize_i32 as i32,
        serialize_i64 as i64,
        serialize_i128 as i128,
        serialize_u8 as u8,
        serialize_u16 as u16,
        serialize_u32 as u32,
        serialize_u64 as u64,
        serialize_u128 as u128,
    }

    fn serialize_f32(self, number: f32) -> Result<Option<Value>, Error> {
        self.serialize_f64(f64::from(number))
    }

    fn serialize_f64(self, number: f64) -> Result<Option<Value>, Error> {
        Float::new(number)
            .map(|float| Some(Value::Float(float)))
            .ok_or_else(|| Error::new(ErrorKind::NotFinite))
    }

    fn serialize_char(self, character: char) -> Result<Option<Value>, Error> {
        Ok(Some(Value::String(character.to_string())))
    }

    fn serialize_str(self, text: &str) -> Result<Option<Value>, Error> {
        Ok(Some(Value::String(text.to_owned())))
    }

    fn serialize_bytes(self, bytes: &[u8]) -> Result<Option<Value>, Error> {
        self.descend()?;
        let items = bytes
            .iter()
            .map(|byte| Value::Integer(Integer::new(&byte.to_string())))
            .collect();
        Ok(Some(Value::List(items)))
    }

    fn serialize_none(self) -> Result<Option<Value>, Error> {
        Ok(None)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Option<Value>, Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Option<Value>, Error> {
        Ok(Some(Value::Tuple(Tuple::new())))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Option<Value>, Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<Option<Value>, Error> {
        Ok(Some(tag(variant)))
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Option<Value>, Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Option<Value>, Error>
    where
        T: Serialize + ?Sized,
    {
        let payload = present(
            value.serialize(self.descend()?)?,
            "as the payload of a variant",
        )?;
        Ok(Some(tagged(tag(variant), payload)))
    }

    fn serialize_seq(self, length: Option<usize>) -> Result<Elements, Error> {
        Ok(Elements::new(self.descend()?, length))
    }

    fn serialize_tuple(self, length: usize) -> Result<Elements, Error> {
        self.serialize_seq(Some(length))
    }

    fn serialize_tuple_struct(self, _name: &'static str, length: usize) -> Result<Elements, Error> {
        self.serialize_seq(Some(length))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        length: usize,
    ) -> Result<TaggedElements, Error> {
        Ok(TaggedElements {
            tag: tag(variant),
            elements: Elements::new(self.descend()?.descend()?, Some(length)),
        })
    }

    fn serialize_map(self, _length: Option<usize>) -> Result<Entries, Error> {
        Ok(Entries {
            entries: Map::new(),
            key: None,
            serializer: self.descend()?,
        })
    }

    fn serialize_struct(self, _name: &'static str, _length: usize) -> Result<Fields, Error> {
        Ok(Fields {
            fields: Tuple::new(),
            serializer: self.descend()?,
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _length: usize,
    ) -> Result<TaggedFields, Error> {
        Ok(TaggedFields {
            tag: tag(variant),
            fields: Fields {
                fields: Tuple::new(),
                serializer: self.descend()?.descend()?,
            },
        })
    }
}

pub struct Elements {
    items: Vec<Value>,
    serializer: Serializer,
}

impl Elements {
    fn new(serializer: Serializer, length: Option<usize>) -> Self {
        Self {
            items: Vec::with_capacity(length.unwrap_or_default()),
            serializer,
        }
    }

    fn push<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        let item = present(value.serialize(self.serializer)?, "in a list")?;
        self.items.push(item);
        Ok(())
    }
}

impl SerializeSeq for Elements {
    type Ok = Option<Value>;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.push(value)
    }

    fn end(self) -> Result<Option<Value>, Error> {
        Ok(Some(Value::List(self.items)))
    }
}

impl SerializeTuple for Elements {
    type Ok = Option<Value>;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.push(value)
    }

    fn end(self) -> Result<Option<Value>, Error> {
        SerializeSeq::end(self)
    }
}

impl SerializeTupleStruct for Elements {
    type Ok = Option<Value>;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.push(value)
    }

    fn end(self) -> Result<Option<Value>, Error> {
        SerializeSeq::end(self)
    }
}

pub struct TaggedElements {
    tag: Value,
    elements: Elements,
}

impl SerializeTupleVariant for TaggedElements {
    type Ok = Option<Value>;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.elements.push(value)
    }

    fn end(self) -> Result<Option<Value>, Error> {
        Ok(Some(tagged(self.tag, Value::List(self.elements.items))))
    }
}

pub struct Entries {
    entries: Map,
    key: Option<Value>,
    serializer: Serializer,
}

impl SerializeMap for Entries {
    type Ok = Option<Value>;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.key = Some(present(key.serialize(self.serializer)?, "as a map key")?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        let key = match self.key.take() {
            Some(key) => key,
            None => return Err(ser::Error::custom("a map value was written before its key")),
        };
        let value = present(value.serialize(self.serializer)?, "as a map value")?;
        if self.entries.insert(key, value).is_some() {
            return Err(Error::new(ErrorKind::DuplicateKey));
        }
        Ok(())
    }

    fn end(self) -> Result<Option<Value>, Error> {
        Ok(Some(if self.entries.is_empty() {
            Value::Tuple(Tuple::new())
        } else {
            Value::Map(self.entries)
        }))
    }
}

pub struct Fields {
    fields: Tuple,
    serializer: Serializer,
}

impl Fields {
    fn insert<T>(&mut self, name: &'static str, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        let Some(value) = value.serialize(self.serializer)? else {
            return Ok(());
        };
        let Some(field) = Ident::parse(name) else {
            return Err(Error::new(ErrorKind::FieldName(name.to_owned())));
        };
        if self.fields.insert(field, value).is_some() {
            return Err(Error::new(ErrorKind::DuplicateField(name.to_owned())));
        }
        Ok(())
    }
}

impl SerializeStruct for Fields {
    type Ok = Option<Value>;
    type Error = Error;

    fn serialize_field<T>(&mut self, name: &'static str, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.insert(name, value)
    }

    fn end(self) -> Result<Option<Value>, Error> {
        Ok(Some(Value::Tuple(self.fields)))
    }
}

pub struct TaggedFields {
    tag: Value,
    fields: Fields,
}

impl SerializeStructVariant for TaggedFields {
    type Ok = Option<Value>;
    type Error = Error;

    fn serialize_field<T>(&mut self, name: &'static str, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.fields.insert(name, value)
    }

    fn end(self) -> Result<Option<Value>, Error> {
        Ok(Some(tagged(self.tag, Value::Tuple(self.fields.fields))))
    }
}
