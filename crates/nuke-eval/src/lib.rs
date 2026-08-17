pub mod error;

use nuke_syntax::expr::{Binding, Document, Expr, ExprKind};
use nuke_syntax::{Ident, MAX_DEPTH, Map, Span, Tuple, Value, surface};

pub use error::{Error, ErrorKind};

pub const MAX_VALUES: usize = 1 << 20;

pub fn eval(source: &str) -> Result<Value, Error> {
    reduce(&surface::parse(source)?)
}

pub fn reduce(document: &Document) -> Result<Value, Error> {
    let mut session = Session::new();
    session.document(document)
}

struct Session {
    budget: usize,
}

impl Session {
    const fn new() -> Self {
        Self { budget: MAX_VALUES }
    }

    fn document(&mut self, document: &Document) -> Result<Value, Error> {
        let mut reducer = Reducer {
            session: self,
            scope: Vec::new(),
        };
        reducer.bind(&document.bindings, 0)?;
        reducer.value(&document.value, 0)
    }
}

struct Measured {
    value: Value,
    values: usize,
    depth: usize,
}

impl Measured {
    fn new(value: Value) -> Self {
        let (values, depth) = measure(&value);
        Self {
            value,
            values,
            depth,
        }
    }
}

struct Bound {
    name: Ident,
    measured: Measured,
}

struct Reducer<'a> {
    session: &'a mut Session,
    scope: Vec<Bound>,
}

impl Reducer<'_> {
    fn bind(&mut self, bindings: &[Binding], depth: usize) -> Result<usize, Error> {
        let frame = self.scope.len();
        for binding in bindings {
            let value = self.value(&binding.value, depth)?;
            self.scope.push(Bound {
                name: binding.name.ident.clone(),
                measured: Measured::new(value),
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
        let Some(at) = self.scope.iter().rposition(|bound| bound.name == *name) else {
            return Err(Error::new(
                ErrorKind::Unbound(name.as_str().to_owned()),
                span,
            ));
        };
        let values = self.scope[at].measured.values;
        let reach = self.scope[at].measured.depth;
        self.charge(values, reach, span, depth)?;
        Ok(self.scope[at].measured.value.clone())
    }

    fn charge(
        &mut self,
        values: usize,
        reach: usize,
        span: Span,
        depth: usize,
    ) -> Result<(), Error> {
        if depth + reach > MAX_DEPTH {
            return Err(Error::new(ErrorKind::TooDeep, span));
        }
        self.spend(values, span)
    }

    fn spend(&mut self, values: usize, span: Span) -> Result<(), Error> {
        if values > self.session.budget {
            return Err(Error::new(ErrorKind::TooLarge, span));
        }
        self.session.budget -= values;
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
