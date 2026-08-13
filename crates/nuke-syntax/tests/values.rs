use nuke_syntax::{Atom, ErrorKind, Ident, Integer, MAX_DEPTH, Value, parse};

fn parsed(source: &str) -> Value {
    parse(source).unwrap_or_else(|error| panic!("{source} should parse, but: {error}"))
}

fn fault(source: &str) -> ErrorKind {
    parse(source)
        .err()
        .unwrap_or_else(|| panic!("{source} should have been rejected"))
        .kind()
        .clone()
}

fn tuple(source: &str) -> nuke_syntax::Tuple {
    match parsed(source) {
        Value::Tuple(fields) => fields,
        other => panic!("{source} should be a tuple, not {other:?}"),
    }
}

fn map(source: &str) -> nuke_syntax::Map {
    match parsed(source) {
        Value::Map(entries) => entries,
        other => panic!("{source} should be a map, not {other:?}"),
    }
}

#[test]
fn a_brace_block_is_a_tuple_or_a_map_by_its_pair_operator() {
    assert!(tuple("{}").is_empty());
    assert_eq!(tuple("{x = 1}").len(), 1);
    assert_eq!(map("{\"a\" => 1}").len(), 1);
    assert_eq!(map("{{} => 1}").len(), 1);
}

#[test]
fn field_order_is_preserved() {
    let fields = tuple("{b = 1 a = 2 c = 3}");
    let names: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["b", "a", "c"]);
    assert_eq!(fields.get("a"), Some(&parsed("2")));
    assert_eq!(fields.get("z"), None);
}

#[test]
fn field_order_is_part_of_the_value_and_entry_order_is_not() {
    assert_ne!(parsed("{a = 1 b = 2}"), parsed("{b = 2 a = 1}"));
    assert_eq!(parsed("{1 => 2 3 => 4}"), parsed("{3 => 4 1 => 2}"));
    assert_eq!(
        parsed("{{1 => 2 3 => 4} => True}"),
        parsed("{{3 => 4 1 => 2} => True}")
    );
}

#[test]
fn a_repeated_field_or_key_is_an_error() {
    assert_eq!(
        fault("{a = 1 a = 2}"),
        ErrorKind::DuplicateField("a".into())
    );
    assert_eq!(fault("{[1 2] => 1 [1 2] => 2}"), ErrorKind::DuplicateKey);
    assert_eq!(
        fault("{{a = 1} => 1 {a = 1} => 2}"),
        ErrorKind::DuplicateKey
    );
}

#[test]
fn map_keys_are_compared_as_values_not_as_text() {
    assert_eq!(map("{1 => \"int\" 1.0 => \"float\"}").len(), 2);
    assert_eq!(
        fault("{1e5 => \"a\" 100000.0 => \"b\"}"),
        ErrorKind::DuplicateKey
    );
    assert_eq!(fault("{0 => \"a\" -0 => \"b\"}"), ErrorKind::DuplicateKey);
    assert_eq!(map("{0.0 => \"a\" -0.0 => \"b\"}").len(), 2);
}

#[test]
fn an_integer_keeps_its_width_and_narrows_on_request() {
    let huge = "1234567890123456789012345678901234567890";
    let Value::Integer(integer) = parsed(huge) else {
        panic!("{huge} should be an integer");
    };
    assert_eq!(integer.as_str(), huge);
    assert_eq!(integer.to_i64(), None);
    assert_eq!(integer.to_i128(), None);
    let Value::Integer(small) = parsed("-42") else {
        panic!("-42 should be an integer");
    };
    assert_eq!(small.to_i64(), Some(-42));
}

#[test]
fn a_number_is_a_float_exactly_when_it_carries_a_fraction_or_an_exponent() {
    assert!(matches!(parsed("1"), Value::Integer(_)));
    assert!(matches!(parsed("1.0"), Value::Float(_)));
    assert!(matches!(parsed("1e5"), Value::Float(_)));
    assert_ne!(parsed("1"), parsed("1.0"));
    assert_eq!(fault("[1e400]"), ErrorKind::FloatOutOfRange);
}

#[test]
fn escapes_decode_and_surrogates_do_not() {
    let Value::String(text) = parsed("\"\\\"\\\\\\n\\r\\t\\u{1F600}\\u{0}\"") else {
        panic!("that should be a string");
    };
    assert_eq!(text, "\"\\\n\r\t\u{1F600}\u{0}");
    assert_eq!(fault("\"\\u{D800}\""), ErrorKind::SurrogateEscape(0xD800));
    assert_eq!(fault("\"\\u{DFFF}\""), ErrorKind::SurrogateEscape(0xDFFF));
    assert_eq!(
        fault("\"\\u{110000}\""),
        ErrorKind::ScalarOutOfRange(0x0011_0000)
    );
    assert_eq!(fault("\"\\u{}\""), ErrorKind::MalformedUnicodeEscape);
    assert_eq!(fault("\"\\u{1234567}\""), ErrorKind::MalformedUnicodeEscape);
    assert_eq!(fault("\"\\u{FF\""), ErrorKind::MalformedUnicodeEscape);
}

#[test]
fn tokens_are_matched_greedily() {
    assert_eq!(parsed("[12]"), Value::List(vec![parsed("12")]));
    assert_eq!(parsed("[1 2]"), Value::List(vec![parsed("1"), parsed("2")]));
    assert_eq!(
        parsed("[TrueFalse]"),
        Value::List(vec![parsed("TrueFalse")])
    );
    assert_eq!(
        parsed("[True False]"),
        Value::List(vec![parsed("True"), parsed("False")])
    );
    let Value::List(items) = parsed("[{a=1}{\"b\"=>2}[3]4 5]") else {
        panic!("that should be a list");
    };
    assert_eq!(items.len(), 5);
}

#[test]
fn nesting_is_bounded_rather_than_left_to_the_stack() {
    let deep = format!("{}{}", "[".repeat(2000), "]".repeat(2000));
    assert_eq!(fault(&deep), ErrorKind::TooDeep);
    let shallow = format!("{}{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
    assert!(parse(&shallow).is_ok());
}

#[test]
fn an_error_carries_the_span_and_the_line_of_its_fault() {
    let source = "[\n  1\n  ,2\n]";
    let error = parse(source).expect_err("a comma is not a separator");
    assert_eq!(error.kind(), &ErrorKind::UnexpectedCharacter(','));
    assert_eq!(&source[error.span().start..error.span().end], ",");
    let location = error.location(source);
    assert_eq!((location.line, location.column), (3, 3));
}

#[test]
fn atoms_and_identifiers_admit_only_their_own_shapes() {
    assert_eq!(
        Atom::parse("True").map(|atom| atom.to_string()),
        Some("True".to_owned())
    );
    assert!(Atom::parse("Atom99").is_some());
    assert!(Atom::parse("true").is_none());
    assert!(Atom::parse("True_").is_none());
    assert!(Atom::parse("").is_none());
    assert!(Ident::parse("snake_case_1").is_some());
    assert!(Ident::parse("Snake").is_none());
    assert!(Ident::parse("snakeCase").is_none());
    assert!(Integer::parse("-0").is_some());
    assert!(Integer::parse("01").is_none());
    assert!(Integer::parse("1.0").is_none());
}
