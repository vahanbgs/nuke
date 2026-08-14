use nuke_syntax::parse;
use nuke_transpile::Segment;
use nuke_transpile::toml::{Error, ErrorKind, to_string};

fn toml(source: &str) -> String {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect("the value should transpile")
}

fn scalar(source: &str) -> String {
    let written = toml(&format!("{{v = {source}}}"));
    written
        .strip_prefix("v = ")
        .expect("a scalar is written as the value of one pair")
        .to_owned()
}

fn refused(source: &str) -> Error {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect_err("the value should be refused")
}

#[test]
fn a_document_that_is_not_a_table_is_refused() {
    for (source, form) in [
        ("[1 2]", "list"),
        ("True", "atom"),
        (r#""text""#, "string"),
        ("42", "integer"),
        ("1.5", "float"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableRoot(form),
            "{error}"
        );
        assert_eq!(error.path().to_string(), "the document");
    }
    assert_eq!(toml("{}"), "");
}

#[test]
fn scalars_carry_their_toml_spelling() {
    assert_eq!(scalar("True"), "true");
    assert_eq!(scalar("False"), "false");
    assert_eq!(scalar("0"), "0");
    assert_eq!(scalar("-7"), "-7");
    assert_eq!(scalar("-0"), "0");
    assert_eq!(scalar(r#""text""#), r#""text""#);
}

#[test]
fn an_atom_toml_has_no_word_for_becomes_a_string_and_null_is_one_of_them() {
    assert_eq!(scalar("Null"), r#""Null""#);
    assert_eq!(scalar("Relative"), r#""Relative""#);
    assert_eq!(
        scalar("[OpenTerminal CloseWindow]"),
        r#"["OpenTerminal", "CloseWindow"]"#
    );
}

#[test]
fn an_integer_wider_than_the_64_bits_toml_gives_one_is_refused() {
    assert_eq!(scalar("9223372036854775807"), "9223372036854775807");
    assert_eq!(scalar("-9223372036854775808"), "-9223372036854775808");

    let wide = "170141183460469231731687303715884105728";
    let error = refused(&format!("{{shell = {{sizes = [{wide}]}}}}"));
    assert_eq!(
        error.kind(),
        &ErrorKind::WideInteger(wide.to_owned()),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "shell.sizes[0]");
}

#[test]
fn a_float_stays_a_float() {
    assert_eq!(scalar("0.0"), "0.0");
    assert_eq!(scalar("-0.0"), "-0.0");
    assert_eq!(scalar("1.5"), "1.5");
    assert_eq!(scalar("1e5"), "100000.0");
    assert_eq!(scalar("1e300"), "1e300");
    for source in ["0.0", "-0.0", "1.5", "1e5", "1e-5", "-2.5e-3", "1e300"] {
        let written = scalar(source);
        assert!(
            written.contains('.') || written.contains('e'),
            "{source} lost its float shape as {written}"
        );
    }
}

#[test]
fn strings_take_the_toml_escapes() {
    assert_eq!(scalar(r#""\"""#), r#""\"""#);
    assert_eq!(scalar(r#""\\""#), r#""\\""#);
    assert_eq!(scalar(r#""\n\r\t""#), r#""\n\r\t""#);
    assert_eq!(scalar(r#""é ✓ 😀""#), r#""é ✓ 😀""#);
    assert_eq!(scalar(r#""\u{0}""#), r#""\u0000""#);
    assert_eq!(scalar(r#""\u{1F}""#), r#""\u001F""#);
    assert_eq!(scalar(r#""\u{7F}""#), r#""\u007F""#);
    assert_eq!(scalar(r#""\u{FFFF}""#), "\"\u{FFFF}\"");
    assert_eq!(scalar(r#""\u{10FFFF}""#), "\"\u{10FFFF}\"");
}

#[test]
fn a_key_is_bare_where_toml_lets_it_be_and_quoted_where_it_cannot() {
    assert_eq!(toml(r#"{"ll" => 1}"#), "ll = 1");
    assert_eq!(toml(r#"{"tab-width" => 1}"#), "tab-width = 1");
    assert_eq!(toml(r#"{"42" => 1}"#), "42 = 1");
    assert_eq!(toml(r#"{"a b" => 1}"#), r#""a b" = 1"#);
    assert_eq!(toml(r#"{"" => 1}"#), r#""" = 1"#);
    assert_eq!(toml(r#"{"a.b" => {c = 1}}"#), "[\"a.b\"]\nc = 1");
}

#[test]
fn an_atom_in_key_position_carries_its_spelling_rather_than_its_value() {
    assert_eq!(toml("{True => 1}"), "True = 1");
    assert_eq!(toml("{Null => 1}"), "Null = 1");
    assert_eq!(toml("{Relative => 1}"), "Relative = 1");
}

#[test]
fn a_key_toml_cannot_name_is_refused_and_two_keys_naming_one_are_too() {
    let error = refused(r#"{"a" => 1 42 => 2}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::UnrepresentableKey("integer"),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "#2");

    let error = refused(r#"{shell = {Relative => 1 "Relative" => 2}}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::DuplicateKey("Relative".to_owned()),
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
fn an_empty_collection_is_written_inline_on_the_line_it_belongs_to() {
    assert_eq!(toml("{a = {} b = []}"), "a = {}\nb = []");
    assert_eq!(toml("{a = [{}] b = [[]]}"), "a = [{}]\nb = [[]]");
}

#[test]
fn a_table_is_a_section_where_a_section_fits_and_inline_where_it_does_not() {
    assert_eq!(toml("{a = 1 b = {c = 2}}"), "a = 1\n\n[b]\nc = 2");
    assert_eq!(toml("{b = {c = 2} a = 1}"), "b = {c = 2}\na = 1");
    assert_eq!(
        toml("{a = {b = 1} c = {d = 2}}"),
        "[a]\nb = 1\n\n[c]\nd = 2"
    );
    assert_eq!(toml("{a = {b = {c = 1}}}"), "[a]\n\n[a.b]\nc = 1");
    assert_eq!(toml("{a = {k = {b = 1} n = 2}}"), "[a]\nk = {b = 1}\nn = 2");
}

#[test]
fn a_list_of_tables_takes_a_header_of_its_own_and_any_other_list_goes_inline() {
    assert_eq!(
        toml("{k = [{a = 1} {a = 2}]}"),
        "[[k]]\na = 1\n\n[[k]]\na = 2"
    );
    assert_eq!(toml("{k = [{a = 1} 2]}"), "k = [{a = 1}, 2]");
    assert_eq!(toml("{k = [{} {a = 1}]}"), "k = [{}, {a = 1}]");
    assert_eq!(toml("{k = [1 2]}"), "k = [1, 2]");
    assert_eq!(toml("{a = {k = [{b = 1}]}}"), "[a]\n\n[[a.k]]\nb = 1");
}

#[test]
fn every_layout_the_writer_chooses_is_toml_a_real_parser_loads() {
    for source in [
        "{a = 1 b = {c = 2} d = {e = {f = 3}}}",
        "{b = {c = 2} a = 1}",
        "{k = [{a = 1} {a = 2}] z = {y = 1}}",
        r#"{"a.b" => {c = 1}}"#,
        r#"{"a b" => {"c d" => [{e = 1}]}}"#,
        "{a = {} b = [] c = [{}] d = [[1] [2]]}",
        r#"{a = "\n\u{7F}\u{0}" b = ["x" Relative Null True]}"#,
        "{a = {b = {c = {d = [{e = 1}]}}}}",
    ] {
        let written = toml(source);
        toml::from_str::<toml::Value>(&written).unwrap_or_else(|error| {
            panic!("{source} wrote TOML that does not parse: {error}\n{written}")
        });
    }
}

#[test]
fn a_dot_file_lays_out_the_way_a_hand_written_toml_file_would() {
    let source = include_str!("../../../fixtures/valid/dotfile.nuke");
    assert_eq!(
        toml(source),
        "\
[editor]
theme = \"gruvbox-dark\"
tab_width = 2
line_numbers = \"Relative\"

[shell]
aliases = {ll = \"eza -l\", gs = \"git status\"}
history_size = 10000

[[keybindings]]
keys = \"ctrl+t\"
action = \"OpenTerminal\"

[[keybindings]]
keys = \"ctrl+w\"
action = \"CloseWindow\""
    );
}

#[test]
fn a_value_nested_past_the_parsers_own_limit_is_refused_rather_than_overflowing() {
    let mut value = nuke_syntax::Value::List(Vec::new());
    for _ in 1..nuke_syntax::MAX_DEPTH + 2 {
        value = nuke_syntax::Value::List(vec![value]);
    }
    let mut tuple = nuke_syntax::Tuple::new();
    let name = nuke_syntax::Ident::parse("a").expect("a is an identifier");
    tuple.insert(name, value);
    let error =
        to_string(&nuke_syntax::Value::Tuple(tuple)).expect_err("the writer should refuse it");
    assert_eq!(error.kind(), &ErrorKind::TooDeep);
    assert!(
        error.path().to_string().starts_with('a'),
        "the path should name the field the deep value sits under"
    );
}
