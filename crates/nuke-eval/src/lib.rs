pub mod error;

use nuke_syntax::expr::{Binding, Document, Expr, ExprKind};
use nuke_syntax::{Ident, MAX_DEPTH, Map, Span, Tuple, Value, surface};

pub use error::{Error, ErrorKind};

pub const MAX_VALUES: usize = 1 << 20;

pub fn eval(source: &str) -> Result<Value, Error> {
    reduce(&surface::parse(source)?)
}

pub fn reduce(document: &Document) -> Result<Value, Error> {
    let mut reducer = Reducer {
        scope: Vec::new(),
        budget: MAX_VALUES,
    };
    reducer.bind(&document.bindings, 0)?;
    reducer.value(&document.value, 0)
}

struct Bound {
    name: Ident,
    value: Value,
    values: usize,
    depth: usize,
}

struct Reducer {
    scope: Vec<Bound>,
    budget: usize,
}

impl Reducer {
    fn bind(&mut self, bindings: &[Binding], depth: usize) -> Result<usize, Error> {
        let frame = self.scope.len();
        for binding in bindings {
            let value = self.value(&binding.value, depth)?;
            let (values, reach) = measure(&value);
            self.scope.push(Bound {
                name: binding.name.ident.clone(),
                value,
                values,
                depth: reach,
            });
        }
        Ok(frame)
    }

    fn value(&mut self, expr: &Expr, depth: usize) -> Result<Value, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::new(ErrorKind::TooDeep, expr.span));
        }
        match &expr.kind {
            ExprKind::Reference(name) => self.reference(name, expr.span, depth),
            ExprKind::Tuple { bindings, fields } => {
                let frame = self.bind(bindings, depth)?;
                let mut tuple = Tuple::new();
                for field in fields {
                    let value = self.value(&field.value, depth + 1)?;
                    tuple.insert(field.name.ident.clone(), value);
                }
                self.scope.truncate(frame);
                self.spend(1, expr.span)?;
                Ok(Value::Tuple(tuple))
            }
            ExprKind::Map { bindings, entries } => {
                let frame = self.bind(bindings, depth)?;
                let mut map = Map::new();
                for entry in entries {
                    let key = self.value(&entry.key, depth + 1)?;
                    let value = self.value(&entry.value, depth + 1)?;
                    if map.insert(key, value).is_some() {
                        return Err(Error::new(ErrorKind::DuplicateKey, entry.key.span));
                    }
                }
                self.scope.truncate(frame);
                self.spend(1, expr.span)?;
                Ok(Value::Map(map))
            }
            ExprKind::Access { operand, field } => {
                let Value::Tuple(tuple) = self.value(operand, depth)? else {
                    return Err(Error::new(ErrorKind::NotATuple, operand.span));
                };
                tuple.take(field.ident.as_str()).ok_or_else(|| {
                    Error::new(
                        ErrorKind::NoSuchField(field.ident.as_str().to_owned()),
                        field.span,
                    )
                })
            }
            ExprKind::List(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    values.push(self.value(item, depth + 1)?);
                }
                self.spend(1, expr.span)?;
                Ok(Value::List(values))
            }
            ExprKind::Atom(atom) => {
                self.spend(1, expr.span)?;
                Ok(Value::Atom(atom.clone()))
            }
            ExprKind::String(text) => {
                self.spend(1, expr.span)?;
                Ok(Value::String(text.clone()))
            }
            ExprKind::Integer(integer) => {
                self.spend(1, expr.span)?;
                Ok(Value::Integer(integer.clone()))
            }
            ExprKind::Float(float) => {
                self.spend(1, expr.span)?;
                Ok(Value::Float(*float))
            }
        }
    }

    fn reference(&mut self, name: &Ident, span: Span, depth: usize) -> Result<Value, Error> {
        let Some(bound) = self.scope.iter().rev().find(|bound| bound.name == *name) else {
            return Err(Error::new(
                ErrorKind::Unbound(name.as_str().to_owned()),
                span,
            ));
        };
        if depth + bound.depth > MAX_DEPTH {
            return Err(Error::new(ErrorKind::TooDeep, span));
        }
        if bound.values > self.budget {
            return Err(Error::new(ErrorKind::TooLarge, span));
        }
        let value = bound.value.clone();
        self.budget -= bound.values;
        Ok(value)
    }

    fn spend(&mut self, values: usize, span: Span) -> Result<(), Error> {
        if values > self.budget {
            return Err(Error::new(ErrorKind::TooLarge, span));
        }
        self.budget -= values;
        Ok(())
    }
}

fn measure(value: &Value) -> (usize, usize) {
    let children: Box<dyn Iterator<Item = &Value>> = match value {
        Value::Tuple(fields) => Box::new(fields.iter().map(|(_, value)| value)),
        Value::Map(entries) => Box::new(entries.iter().flat_map(|(key, value)| [key, value])),
        Value::List(items) => Box::new(items.iter()),
        _ => return (1, 1),
    };
    let (values, depth) = children.fold((1, 0), |(values, depth), child| {
        let (child_values, child_depth) = measure(child);
        (values + child_values, depth.max(child_depth))
    });
    (values, depth + 1)
}
