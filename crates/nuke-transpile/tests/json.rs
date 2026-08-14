use nuke_syntax::parse;
use nuke_transpile::Segment;
use nuke_transpile::json::{Error, ErrorKind, to_string, to_string_compact};

fn pretty(source: &str) -> String {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect("the value should transpile")
}

fn compact(source: &str) -> String {
    let value = parse(source).expect("the source should parse");
    to_string_compact(&value).expect("the value should transpile")
}

fn refused(source: &str) -> Error {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect_err("the value should be refused")
}

#[test]
fn scalars_carry_their_json_spelling() {
    assert_eq!(compact("True"), "true");
    assert_eq!(compact("False"), "false");
    assert_eq!(compact("Null"), "null");
    assert_eq!(compact("0"), "0");
    assert_eq!(compact("-7"), "-7");
    assert_eq!(compact("-0"), "0");
    assert_eq!(compact(r#""text""#), r#""text""#);
}

#[test]
fn an_atom_that_is_not_a_boolean_or_null_becomes_a_string() {
    assert_eq!(compact("Relative"), r#""Relative""#);
    assert_eq!(
        compact("[OpenTerminal CloseWindow]"),
        r#"["OpenTerminal","CloseWindow"]"#
    );
}

#[test]
fn an_integer_keeps_every_digit_it_came_with() {
    let wide = "170141183460469231731687303715884105728";
    assert_eq!(compact(wide), wide);
}

#[test]
fn a_float_stays_a_float() {
    assert_eq!(compact("0.0"), "0.0");
    assert_eq!(compact("-0.0"), "-0.0");
    assert_eq!(compact("1.5"), "1.5");
    assert_eq!(compact("1e5"), "100000.0");
    assert_eq!(compact("1e300"), "1e300");
    for source in ["0.0", "-0.0", "1.5", "1e5", "1e-5", "-2.5e-3", "1e300"] {
        let json = compact(source);
        assert!(
            json.contains('.') || json.contains('e'),
            "{source} lost its float shape as {json}"
        );
    }
}

#[test]
fn strings_take_the_json_escapes() {
    assert_eq!(compact(r#""\"""#), r#""\"""#);
    assert_eq!(compact(r#""\\""#), r#""\\""#);
    assert_eq!(compact(r#""\n\r\t""#), r#""\n\r\t""#);
    assert_eq!(compact(r#""\u{0}""#), "\"\\u0000\"");
    assert_eq!(compact(r#""\u{1F}""#), "\"\\u001f\"");
    assert_eq!(compact(r#""\u{7F}""#), "\"\u{7F}\"");
    assert_eq!(compact(r#""é ✓ 😀""#), r#""é ✓ 😀""#);
}

#[test]
fn a_tuple_becomes_an_object_in_declaration_order() {
    assert_eq!(compact("{b = 1 a = 2}"), r#"{"b":1,"a":2}"#);
    assert_eq!(compact("{}"), "{}");
    assert_eq!(compact("[]"), "[]");
}

#[test]
fn a_map_with_word_shaped_keys_becomes_an_object() {
    assert_eq!(compact(r#"{"ll" => "eza -l"}"#), r#"{"ll":"eza -l"}"#);
    assert_eq!(compact("{Relative => 1}"), r#"{"Relative":1}"#);
}

#[test]
fn an_atom_in_key_position_keeps_its_spelling_rather_than_its_value() {
    assert_eq!(compact("{True => 1}"), r#"{"True":1}"#);
    assert_eq!(compact("{Null => 1}"), r#"{"Null":1}"#);
}

#[test]
fn the_pretty_layout_indents_by_two_and_ends_without_a_newline() {
    assert_eq!(
        pretty(r#"{a = 1 b = [True Null] c = {} d = {"k" => [1]}}"#),
        "{\n  \"a\": 1,\n  \"b\": [\n    true,\n    null\n  ],\n  \"c\": {},\n  \"d\": {\n    \"k\": [\n      1\n    ]\n  }\n}"
    );
}

#[test]
fn a_key_that_is_not_a_string_or_an_atom_is_refused_where_it_stands() {
    let error = refused("{shell = {aliases = {[1 2] => 1}}}");
    assert_eq!(
        error.kind(),
        &ErrorKind::UnrepresentableKey("list"),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "shell.aliases#1");
    assert_eq!(
        error.path().segments().last(),
        Some(&Segment::Entry(0)),
        "the entry, not its key, is what can be named"
    );

    for (source, form) in [
        ("{42 => True}", "integer"),
        ("{-1.5 => True}", "float"),
        ("{[] => []}", "list"),
        (r#"{{x = 1} => True}"#, "tuple"),
        (r#"{{"n" => 1} => True}"#, "map"),
    ] {
        assert_eq!(
            refused(source).kind(),
            &ErrorKind::UnrepresentableKey(form),
            "for {source}"
        );
    }
}

#[test]
fn two_keys_that_name_the_same_json_key_are_refused() {
    let error = refused(r#"{Relative => 1 "Relative" => 2}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::DuplicateKey("Relative".to_owned()),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "#2");
}

#[test]
fn a_value_nested_past_the_parsers_own_limit_is_refused_rather_than_overflowing() {
    let depth = nuke_syntax::MAX_DEPTH + 2;
    let source = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
    assert!(parse(&source).is_err(), "the parser refuses the same shape");

    let mut value = nuke_syntax::Value::List(Vec::new());
    for _ in 1..depth {
        value = nuke_syntax::Value::List(vec![value]);
    }
    let error = to_string(&value).expect_err("the writer should refuse it");
    assert_eq!(error.kind(), &ErrorKind::TooDeep);
}

#[test]
fn a_path_names_the_root_when_nothing_encloses_the_value() {
    let error = refused("{[1] => 1}");
    assert_eq!(error.path().to_string(), "#1");
    assert!(!error.path().is_root());
    assert_eq!(nuke_transpile::Path::default().to_string(), "the document");
}
