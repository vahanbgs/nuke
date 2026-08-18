use nuke_syntax::{Map, Tuple, Value};

#[derive(Clone, Copy)]
pub(crate) enum Table<'a> {
    Tuple(&'a Tuple),
    Map(&'a Map),
}

impl<'a> Table<'a> {
    pub(crate) fn of(value: &'a Value) -> Option<Self> {
        match value {
            Value::Tuple(tuple) => Some(Self::Tuple(tuple)),
            Value::Map(map) => Some(Self::Map(map)),
            _ => None,
        }
    }
}
