use std::borrow::Cow;

use nuke_syntax::{Align, Float, Integer, Notation, Spec, Value};

use crate::ErrorKind;

pub(crate) fn of(value: &Value, spec: Option<Spec>) -> Result<Cow<'_, str>, ErrorKind> {
    let Some(spec) = spec else {
        return match value {
            Value::String(text) => Ok(Cow::Borrowed(text.as_str())),
            Value::Integer(number) => Ok(Cow::Borrowed(number.as_str())),
            Value::Float(_) => Err(ErrorKind::NeedsPrecision),
            _ => Err(ErrorKind::NoText),
        };
    };
    if spec.prefixed && spec.notation.is_none() {
        return Err(ErrorKind::UnfitSpec);
    }
    if spec.zeroed && (spec.width.is_none() || spec.align.is_some() || spec.fill.is_some()) {
        return Err(ErrorKind::UnfitSpec);
    }
    let (body, numeric) = match value {
        Value::String(text) => (of_string(text, &spec)?, false),
        Value::Integer(number) => (of_integer(number, &spec)?, true),
        Value::Float(number) => (of_float(*number, &spec)?, true),
        _ => return Err(ErrorKind::NoText),
    };
    Ok(Cow::Owned(pad(body, &spec, numeric)))
}

fn of_string(text: &str, spec: &Spec) -> Result<String, ErrorKind> {
    if spec.precision.is_some() || spec.notation.is_some() || spec.sign {
        return Err(ErrorKind::UnfitSpec);
    }
    Ok(text.to_owned())
}

fn of_integer(number: &Integer, spec: &Spec) -> Result<String, ErrorKind> {
    if spec.precision.is_some() {
        return Err(ErrorKind::UnfitSpec);
    }
    let Some(notation) = spec.notation else {
        return Ok(signed(number.as_str().to_owned(), spec.sign));
    };
    let Some(value) = number.to_i128().filter(|value| *value >= 0) else {
        return Err(if number.as_str().starts_with('-') {
            ErrorKind::UnfitSpec
        } else {
            ErrorKind::TooWide
        });
    };
    let body = match (notation, spec.prefixed) {
        (Notation::Binary, false) => format!("{value:b}"),
        (Notation::Binary, true) => format!("{value:#b}"),
        (Notation::Octal, false) => format!("{value:o}"),
        (Notation::Octal, true) => format!("{value:#o}"),
        (Notation::LowerHex, false) => format!("{value:x}"),
        (Notation::LowerHex, true) => format!("{value:#x}"),
        (Notation::UpperHex, false) => format!("{value:X}"),
        (Notation::UpperHex, true) => format!("{value:#X}"),
        (Notation::Exponent, _) => return Err(ErrorKind::UnfitSpec),
    };
    Ok(signed(body, spec.sign))
}

fn of_float(number: Float, spec: &Spec) -> Result<String, ErrorKind> {
    let Some(precision) = spec.precision else {
        return Err(ErrorKind::NeedsPrecision);
    };
    let value = number.get();
    let body = match spec.notation {
        None => format!("{value:.precision$}"),
        Some(Notation::Exponent) => format!("{value:.precision$e}"),
        Some(_) => return Err(ErrorKind::UnfitSpec),
    };
    Ok(signed(body, spec.sign))
}

fn signed(body: String, wanted: bool) -> String {
    if wanted && !body.starts_with('-') {
        return format!("+{body}");
    }
    body
}

fn pad(body: String, spec: &Spec, numeric: bool) -> String {
    let Some(width) = spec.width else {
        return body;
    };
    let Some(missing) = width.checked_sub(body.chars().count()) else {
        return body;
    };
    if spec.zeroed {
        let at = digits(&body, spec.prefixed);
        return format!("{}{}{}", &body[..at], run('0', missing), &body[at..]);
    }
    let fill = spec.fill.unwrap_or(' ');
    let align = spec
        .align
        .unwrap_or(if numeric { Align::Right } else { Align::Left });
    match align {
        Align::Left => format!("{body}{}", run(fill, missing)),
        Align::Right => format!("{}{body}", run(fill, missing)),
        Align::Centre => {
            let before = missing / 2;
            format!("{}{body}{}", run(fill, before), run(fill, missing - before))
        }
    }
}

fn digits(body: &str, prefixed: bool) -> usize {
    let sign = usize::from(body.starts_with(['+', '-']));
    sign + if prefixed { "0x".len() } else { 0 }
}

fn run(character: char, count: usize) -> String {
    (0..count).map(|_| character).collect()
}
