use std::fmt;

use abnf_to_pest::escape_rulename;
use pest_vm::Vm;

pub const TOKENS_ABNF: &str = include_str!("../../../grammar/tokens.abnf");

pub const CANONICAL_ABNF: &str = include_str!("../../../grammar/canonical.abnf");

pub const SURFACE_ABNF: &str = include_str!("../../../grammar/surface.abnf");

pub const DOCUMENT: &str = "document";

pub const NUMBER: &str = "number";

pub const CANONICAL_NUMBER: &str = "canonical-number";

pub const SURFACE_NUMBER: &str = "surface-number";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layer {
    pub fragments: &'static [&'static str],
    pub start: &'static str,
    pub refined: &'static [(&'static str, &'static str)],
}

impl Layer {
    pub fn abnf(&self) -> String {
        self.fragments.join("\n")
    }
}

pub const CANONICAL: Layer = Layer {
    fragments: &[TOKENS_ABNF, CANONICAL_ABNF],
    start: DOCUMENT,
    refined: &[(NUMBER, CANONICAL_NUMBER)],
};

pub const SURFACE: Layer = Layer {
    fragments: &[TOKENS_ABNF, SURFACE_ABNF],
    start: DOCUMENT,
    refined: &[(NUMBER, SURFACE_NUMBER)],
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    Syntax(String),
    Number(String),
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(message) => write!(f, "{message}"),
            Self::Number(token) => {
                write!(f, "`{token}` is not how this grammar spells a number")
            }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Abnf(String),
    Pest(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Abnf(message) => write!(f, "the grammar is not valid ABNF: {message}"),
            Self::Pest(message) => write!(f, "the grammar does not translate to pest: {message}"),
        }
    }
}

impl std::error::Error for Error {}

pub fn to_pest(abnf: &str) -> Result<String, Error> {
    let rules = abnf_to_pest::parse_abnf(abnf).map_err(|error| Error::Abnf(error.to_string()))?;
    Ok(abnf_to_pest::render_rules_to_pest(rules)
        .pretty(100)
        .to_string())
}

pub struct Grammar {
    vm: Vm,
    layer: Layer,
    start: String,
}

impl Grammar {
    pub fn canonical() -> Result<Self, Error> {
        Self::new(CANONICAL)
    }

    pub fn surface() -> Result<Self, Error> {
        Self::new(SURFACE)
    }

    pub fn new(layer: Layer) -> Result<Self, Error> {
        let pest = to_pest(&layer.abnf())?;
        let (_, rules) = pest_meta::parse_and_optimize(&pest).map_err(|errors| {
            Error::Pest(
                errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        })?;
        Ok(Self {
            vm: Vm::new(rules),
            layer,
            start: escape_rulename(layer.start),
        })
    }

    pub fn parse<'a>(&'a self, input: &'a str) -> Result<Parse<'a>, Rejection> {
        let pairs = self
            .vm
            .parse(&self.start, input)
            .map_err(|error| Rejection::Syntax(error.to_string()))?;
        let consumed = pairs
            .clone()
            .next()
            .map_or(0, |pair| pair.as_span().end_pos().pos());
        if consumed != input.len() {
            return Err(Rejection::Syntax(format!(
                "trailing input at byte {consumed} of {}",
                input.len()
            )));
        }
        for (token, refining) in self.layer.refined {
            let token = escape_rulename(token);
            for pair in pairs
                .clone()
                .flatten()
                .filter(|pair| pair.as_rule() == token.as_str())
            {
                let spelling = pair.as_str();
                if !self.matches(refining, spelling) {
                    return Err(Rejection::Number(spelling.to_owned()));
                }
            }
        }
        Ok(Parse { pairs })
    }

    pub fn matches(&self, rule: &str, input: &str) -> bool {
        self.vm
            .parse(&escape_rulename(rule), input)
            .is_ok_and(|pairs| {
                pairs
                    .clone()
                    .next()
                    .map(|pair| pair.as_span().end_pos().pos())
                    == Some(input.len())
            })
    }

    pub fn accepts(&self, input: &str) -> bool {
        self.parse(input).is_ok()
    }
}

pub struct Parse<'a> {
    pairs: pest::iterators::Pairs<'a, &'a str>,
}

impl<'a> Parse<'a> {
    pub fn shape(&self) -> Shape {
        let Some(document) = self.pairs.clone().next() else {
            return Shape::Empty;
        };
        if let Some(fields) = document
            .clone()
            .into_inner()
            .find(|child| child.as_rule() == "fields")
        {
            return Shape::Tuple(fields_of(&fields));
        }
        descend(document, &["value"]).map_or(Shape::Empty, shape_of)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shape {
    Empty,
    Scalar(String),
    List(Vec<Shape>),
    Tuple(Vec<(String, Shape)>),
    Map(Vec<(Shape, Shape)>),
}

fn descend<'a>(
    pair: pest::iterators::Pair<'a, &'a str>,
    path: &[&str],
) -> Option<pest::iterators::Pair<'a, &'a str>> {
    let Some((head, rest)) = path.split_first() else {
        return Some(pair);
    };
    pair.into_inner()
        .find(|child| child.as_rule() == *head)
        .and_then(|child| descend(child, rest))
}

fn children<'a>(
    pair: &pest::iterators::Pair<'a, &'a str>,
    rule: &str,
) -> Vec<pest::iterators::Pair<'a, &'a str>> {
    pair.clone()
        .into_inner()
        .filter(|child| child.as_rule() == rule)
        .collect()
}

fn fields_of(pair: &pest::iterators::Pair<'_, &str>) -> Vec<(String, Shape)> {
    children(pair, "field")
        .into_iter()
        .filter_map(|field| {
            let name = descend(field.clone(), &["ident"])?.as_str().to_owned();
            let value = descend(field, &["value"])?;
            Some((name, shape_of(value)))
        })
        .collect()
}

fn shape_of(value: pest::iterators::Pair<'_, &str>) -> Shape {
    let mut inner = value;
    loop {
        let below = match inner.as_rule() {
            "value" if !children(&inner, "apply").is_empty() => "apply",
            "apply" if children(&inner, "apply").is_empty() => "pipe",
            "pipe" if children(&inner, "projection").len() == 1 => "projection",
            "projection" if children(&inner, "access").is_empty() => "operand",
            "apply" | "pipe" => return Shape::Scalar(inner.as_rule().to_owned()),
            "projection" => return Shape::Scalar("access".to_owned()),
            "value" | "operand" => {
                let Some(taken) = inner.into_inner().next() else {
                    return Shape::Empty;
                };
                inner = taken;
                continue;
            }
            _ => break,
        };
        let Some(next) = children(&inner, below).into_iter().next() else {
            return Shape::Empty;
        };
        inner = next;
    }
    match inner.as_rule() {
        "list" => Shape::List(
            children(&inner, "value")
                .into_iter()
                .map(shape_of)
                .collect(),
        ),
        "group" => children(&inner, "value")
            .into_iter()
            .next()
            .map_or(Shape::Empty, shape_of),
        "tuple" => Shape::Tuple(fields_of(&inner)),
        "map" => Shape::Map(
            children(&inner, "entry")
                .into_iter()
                .filter_map(|entry| {
                    let mut values = children(&entry, "value").into_iter();
                    let key = values.next()?;
                    let value = values.next()?;
                    Some((shape_of(key), shape_of(value)))
                })
                .collect(),
        ),
        other => Shape::Scalar(other.to_owned()),
    }
}
