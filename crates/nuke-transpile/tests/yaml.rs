use nuke_syntax::parse;
use nuke_transpile::Segment;
use nuke_transpile::yaml::{Error, ErrorKind, to_string};

fn yaml(source: &str) -> String {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect("the value should transpile")
}

fn refused(source: &str) -> Error {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect_err("the value should be refused")
}

#[test]
fn scalars_carry_their_yaml_spelling() {
    assert_eq!(yaml("True"), "true");
    assert_eq!(yaml("False"), "false");
    assert_eq!(yaml("Null"), "null");
    assert_eq!(yaml("0"), "0");
    assert_eq!(yaml("-7"), "-7");
    assert_eq!(yaml("-0"), "0");
    assert_eq!(yaml(r#""text""#), r#""text""#);
    assert_eq!(yaml("Relative"), "Relative");
}

#[test]
fn a_string_is_always_quoted_and_a_plain_atom_never_is() {
    assert_eq!(yaml("[Relative Atom99]"), "- Relative\n- Atom99");
    assert_eq!(
        yaml(r#"["" "text" "a b c"]"#),
        "- \"\"\n- \"text\"\n- \"a b c\""
    );
}

#[test]
fn a_word_that_a_yaml_1_1_loader_reads_as_a_boolean_is_quoted() {
    for spelling in ["Yes", "No", "On", "Off", "Y", "N", "TRUE", "NULL"] {
        assert_eq!(
            yaml(spelling),
            format!("\"{spelling}\""),
            "the atom {spelling} must not resolve as a boolean"
        );
    }
    assert_eq!(yaml("{on = 1 off = 2}"), "\"on\": 1\n\"off\": 2");
    assert_eq!(yaml("{y = 1 no = 2}"), "\"y\": 1\n\"no\": 2");
    assert_eq!(yaml("{one = 1 note = 2}"), "one: 1\nnote: 2");
}

#[test]
fn an_integer_keeps_every_digit_it_came_with() {
    let wide = "170141183460469231731687303715884105728";
    assert_eq!(yaml(wide), wide);
}

#[test]
fn a_float_carries_a_point_and_a_signed_exponent() {
    assert_eq!(yaml("0.0"), "0.0");
    assert_eq!(yaml("-0.0"), "-0.0");
    assert_eq!(yaml("1.5"), "1.5");
    assert_eq!(yaml("1e5"), "100000.0");
    assert_eq!(yaml("1e-5"), "0.00001");
    assert_eq!(yaml("1e-300"), "1.0e-300");
    assert_eq!(yaml("1e300"), "1.0e+300");
    assert_eq!(yaml("0.5e10"), "5000000000.0");
    for source in ["0.0", "-0.0", "1.5", "1e5", "1e-5", "-2.5e-3", "1e300"] {
        let written = yaml(source);
        assert!(
            written.contains('.'),
            "{source} lost its float shape as {written}"
        );
        assert!(
            !written.contains('e') || written.contains("e+") || written.contains("e-"),
            "{source} left its exponent unsigned as {written}"
        );
    }
}

#[test]
fn strings_take_the_yaml_escapes() {
    assert_eq!(yaml(r#""\"""#), r#""\"""#);
    assert_eq!(yaml(r#""\\""#), r#""\\""#);
    assert_eq!(yaml(r#""\n\r\t""#), r#""\n\r\t""#);
    assert_eq!(yaml(r#""é ✓ 😀""#), r#""é ✓ 😀""#);
}

#[test]
fn a_character_yaml_cannot_print_is_escaped_even_though_json_lets_it_through() {
    assert_eq!(yaml(r#""\u{0}""#), r#""\u0000""#);
    assert_eq!(yaml(r#""\u{1F}""#), r#""\u001f""#);
    assert_eq!(yaml(r#""\u{7F}""#), r#""\u007f""#);
    assert_eq!(yaml(r#""\u{85}""#), r#""\u0085""#);
    assert_eq!(yaml(r#""\u{9F}""#), r#""\u009f""#);
    assert_eq!(yaml(r#""\u{2028}""#), r#""\u2028""#);
    assert_eq!(yaml(r#""\u{FEFF}""#), r#""\ufeff""#);
    assert_eq!(yaml(r#""\u{FFFF}""#), r#""\uffff""#);
    assert_eq!(yaml(r#""\u{10FFFF}""#), "\"\u{10FFFF}\"");
}

#[test]
fn an_empty_collection_is_written_in_flow_on_the_line_it_belongs_to() {
    assert_eq!(yaml("{}"), "{}");
    assert_eq!(yaml("[]"), "[]");
    assert_eq!(yaml("{a = [] b = {}}"), "a: []\nb: {}");
    assert_eq!(yaml("[[] {}]"), "- []\n- {}");
}

#[test]
fn a_child_sits_two_columns_in_and_a_list_item_carries_its_own() {
    assert_eq!(
        yaml(r#"{a = 1 b = [True Null] c = {} d = {"k" => [1]}}"#),
        "a: 1\nb:\n  - true\n  - null\nc: {}\nd:\n  \"k\":\n    - 1"
    );
    assert_eq!(yaml("[[[1]]]"), "- - - 1");
    assert_eq!(yaml("[{a = 1 b = 2}]"), "- a: 1\n  b: 2");
}

#[test]
fn a_dot_file_lays_out_the_way_a_hand_written_yaml_file_would() {
    let source = include_str!("../../../fixtures/valid/dotfile.nuke");
    assert_eq!(
        yaml(source),
        "\
editor:
  theme: \"gruvbox-dark\"
  tab_width: 2
  line_numbers: Relative
shell:
  aliases:
    \"ll\": \"eza -l\"
    \"gs\": \"git status\"
  history_size: 10000
keybindings:
  - keys: \"ctrl+t\"
    action: OpenTerminal
  - keys: \"ctrl+w\"
    action: CloseWindow"
    );
}

#[test]
fn a_key_yaml_cannot_write_as_a_word_takes_an_explicit_entry() {
    let source = include_str!("../../../fixtures/valid/maps.nuke");
    assert_eq!(
        yaml(source),
        "\
\"string key\": 1
42: true
-1.5: \"float key\"
true: \"atom key\"
? [1, 2]
: \"list key\"
? {x: 1}
: \"tuple key\"
? {}
: \"empty key\"
? {\"nested\": 1}
: \"map key\""
    );
}

#[test]
fn an_atom_in_key_position_carries_its_value_rather_than_its_spelling() {
    assert_eq!(yaml("{True => 1}"), "true: 1");
    assert_eq!(yaml("{Null => 1}"), "null: 1");
    assert_eq!(yaml("{Relative => 1}"), "Relative: 1");
    assert_eq!(yaml("{Yes => 1}"), "\"Yes\": 1");
}

#[test]
fn two_keys_that_name_the_same_yaml_node_are_refused() {
    let error = refused(r#"{Relative => 1 "Relative" => 2}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::DuplicateKey("\"Relative\"".to_owned()),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "#2");

    let error = refused(r#"{shell = {{x = 1} => 1 {"x" => 1} => 2}}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::DuplicateKey("{\"x\": 1}".to_owned()),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "shell#2");
    assert_eq!(
        error.path().segments().last(),
        Some(&Segment::Entry(1)),
        "the entry, not its key, is what can be named"
    );
}

#[test]
fn a_key_that_is_a_number_and_a_string_that_spells_it_are_different_keys() {
    assert_eq!(yaml(r#"{42 => 1 "42" => 2}"#), "42: 1\n\"42\": 2");
    assert_eq!(yaml(r#"{1 => 1 1.0 => 2}"#), "1: 1\n1.0: 2");
}

#[test]
fn a_value_nested_past_the_parsers_own_limit_is_refused_rather_than_overflowing() {
    let depth = nuke_syntax::MAX_DEPTH + 2;
    let mut value = nuke_syntax::Value::List(Vec::new());
    for _ in 1..depth {
        value = nuke_syntax::Value::List(vec![value]);
    }
    let error = to_string(&value).expect_err("the writer should refuse it");
    assert_eq!(error.kind(), &ErrorKind::TooDeep);

    let mut map = nuke_syntax::Map::new();
    let one = nuke_syntax::Integer::parse("1").expect("1 is an integer");
    map.insert(value, nuke_syntax::Value::Integer(one));
    let error = to_string(&nuke_syntax::Value::Map(map))
        .expect_err("a key nests as deep as any other value");
    assert_eq!(error.kind(), &ErrorKind::TooDeep);
}
