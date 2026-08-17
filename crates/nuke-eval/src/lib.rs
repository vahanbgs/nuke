pub mod error;

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use nuke_syntax::expr::{Binding, Document, Expr, ExprKind, Name};
use nuke_syntax::{Ident, MAX_DEPTH, Map, Span, Tuple, Value, surface};

pub use error::{Error, ErrorKind};

pub const MAX_VALUES: usize = 1 << 20;

pub const MAX_BYTES: usize = 1 << 20;

pub const MAX_IMPORTS: usize = 64;

pub fn eval(source: &str) -> Result<Value, Error> {
    reduce(&surface::parse(source)?)
}

pub fn eval_at(source: &str, path: &Path) -> Result<Value, Error> {
    reduce_at(&surface::parse(source)?, path)
}

pub fn reduce(document: &Document) -> Result<Value, Error> {
    Session::new().document(document, None)
}

pub fn reduce_at(document: &Document, path: &Path) -> Result<Value, Error> {
    let anchor = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let origin = anchor.parent().map(Path::to_path_buf);
    let mut session = Session::new();
    session.open.push(anchor);
    session.document(document, origin)
}

struct Session {
    budget: usize,
    open: Vec<PathBuf>,
    loaded: HashMap<PathBuf, Measured>,
}

impl Session {
    fn new() -> Self {
        Self {
            budget: MAX_VALUES,
            open: Vec::new(),
            loaded: HashMap::new(),
        }
    }

    fn document(&mut self, document: &Document, origin: Option<PathBuf>) -> Result<Value, Error> {
        let mut reducer = Reducer {
            session: self,
            scope: Vec::new(),
            origin,
        };
        reducer.bind(&document.bindings, 0)?;
        reducer.value(&document.value, 0)
    }

    fn load(&mut self, path: &Path, literal: &str, span: Span) -> Result<Measured, Error> {
        if self.open.len() >= MAX_IMPORTS {
            return Err(Error::new(ErrorKind::ImportsTooDeep, span));
        }
        let source = fs::read_to_string(path).map_err(|error| unreadable(literal, &error, span))?;
        let document = surface::parse(&source)
            .map_err(|error| imported(path, &source, Error::from(error), span))?;
        self.open.push(path.to_path_buf());
        let reduced = self.document(&document, path.parent().map(Path::to_path_buf));
        self.open.pop();
        Ok(Measured::new(
            reduced.map_err(|error| imported(path, &source, error, span))?,
        ))
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
    origin: Option<PathBuf>,
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
            ExprKind::Call { name, operand } => self.call(name, operand, expr.span, depth),
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

    fn call(
        &mut self,
        name: &Name,
        operand: &Expr,
        span: Span,
        depth: usize,
    ) -> Result<Value, Error> {
        match name.ident.as_str() {
            "import" => self.import(operand, span, depth),
            "concat" => self.concat(operand, span, depth),
            other => Err(Error::new(
                ErrorKind::NoSuchBuiltin(other.to_owned()),
                name.span,
            )),
        }
    }

    fn concat(&mut self, operand: &Expr, span: Span, depth: usize) -> Result<Value, Error> {
        let Value::List(parts) = self.value(operand, depth)? else {
            return Err(Error::new(ErrorKind::NotAList, operand.span));
        };
        let mut text = String::new();
        for part in parts {
            let Value::String(part) = part else {
                return Err(Error::new(ErrorKind::NotAString, operand.span));
            };
            if text.len() + part.len() > MAX_BYTES {
                return Err(Error::new(ErrorKind::TooLong, span));
            }
            text.push_str(&part);
        }
        self.spend(1, span)?;
        Ok(Value::String(text))
    }

    fn import(&mut self, operand: &Expr, span: Span, depth: usize) -> Result<Value, Error> {
        let ExprKind::String(literal) = &operand.kind else {
            return Err(Error::new(ErrorKind::ExpectedImportPath, operand.span));
        };
        let joined = match self.origin.as_deref() {
            Some(origin) => origin.join(literal),
            None => return Err(Error::new(ErrorKind::NoOrigin, span)),
        };
        let path = joined
            .canonicalize()
            .map_err(|error| unreadable(literal, &error, span))?;
        if self.session.open.contains(&path) {
            return Err(Error::new(ErrorKind::ImportCycle(path), span));
        }
        if !self.session.loaded.contains_key(&path) {
            let measured = self.session.load(&path, literal, span)?;
            self.session.loaded.insert(path.clone(), measured);
        }
        let values = self.session.loaded[&path].values;
        let reach = self.session.loaded[&path].depth;
        self.charge(values, reach, span, depth)?;
        Ok(self.session.loaded[&path].value.clone())
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

fn unreadable(path: &str, error: &io::Error, span: Span) -> Error {
    Error::new(
        ErrorKind::Unreadable {
            path: path.to_owned(),
            cause: error.kind(),
        },
        span,
    )
}

fn imported(path: &Path, source: &str, error: Error, span: Span) -> Error {
    let at = error.location(source);
    Error::new(
        ErrorKind::Import {
            path: path.to_path_buf(),
            at,
            cause: Box::new(error),
        },
        span,
    )
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
