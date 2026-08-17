use nuke_syntax::parse;
use nuke_transpile::kdl::{Error, ErrorKind, to_string};

fn kdl(source: &str) -> String {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect("the value should transpile")
}

fn scalar(source: &str) -> String {
    let written = kdl(&format!("{{v = {source}}}"));
    written
        .strip_prefix("v ")
        .expect("a scalar is written as the argument of one node")
        .to_owned()
}

fn refused(source: &str) -> Error {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect_err("the value should be refused")
}

#[test]
fn a_document_that_is_not_a_collection_is_refused() {
    for (source, form) in [
        ("True", "atom"),
        ("Null", "atom"),
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
        assert!(error.path().is_root(), "for {source}");
    }
    assert_eq!(kdl("{}"), "");
    assert_eq!(kdl("[]"), "");
}

#[test]
fn scalars_carry_their_kdl_spelling() {
    assert_eq!(scalar("0"), "0");
    assert_eq!(scalar("-0"), "0");
    assert_eq!(scalar("42"), "42");
    assert_eq!(scalar("-7"), "-7");
    assert_eq!(scalar(r#""text""#), r#""text""#);
    assert_eq!(scalar(r#""a b c""#), r#""a b c""#);
    assert_eq!(scalar(r#""""#), r#""""#);
}

#[test]
fn an_atom_kdl_has_a_keyword_for_takes_it_and_every_other_stays_a_bare_word() {
    assert_eq!(scalar("True"), "#true");
    assert_eq!(scalar("False"), "#false");
    assert_eq!(scalar("Null"), "#null");
    assert_eq!(scalar("Relative"), "Relative");
    assert_eq!(scalar("Atom99"), "Atom99");
    assert_eq!(
        kdl("[OpenTerminal CloseWindow]"),
        "_item OpenTerminal\n_item CloseWindow"
    );
    assert_ne!(scalar("Relative"), scalar(r#""Relative""#));
}

#[test]
fn a_field_name_kdl_reserves_is_quoted_and_every_other_is_bare() {
    assert_eq!(kdl("{true = 1}"), "\"true\" 1");
    assert_eq!(kdl("{false = 1}"), "\"false\" 1");
    assert_eq!(kdl("{null = 1}"), "\"null\" 1");
    assert_eq!(kdl("{inf = 1}"), "\"inf\" 1");
    assert_eq!(kdl("{nan = 1}"), "\"nan\" 1");
    assert_eq!(
        kdl("{truely = 1 t = 2 true_ = 3}"),
        "truely 1\nt 2\ntrue_ 3"
    );
    assert_eq!(kdl("{snake_case_1 = Null}"), "snake_case_1 #null");
}

#[test]
fn an_empty_collection_is_a_node_with_no_argument_and_no_children() {
    assert_eq!(kdl(r#"{a = {} b = [] c = ""}"#), "a\nb\nc \"\"");
    assert_eq!(kdl("{a = {} b = 1}"), "a\nb 1");
    assert_eq!(kdl("[[] {}]"), "_item\n_item");
    assert_eq!(kdl("{a = [[]]}"), "a {\n  _item\n}");
}

#[test]
fn a_scalar_is_an_argument_and_a_collection_is_a_block() {
    assert_eq!(kdl("{b = 1 a = 2}"), "b 1\na 2");
    assert_eq!(kdl("{a = {b = 1}}"), "a {\n  b 1\n}");
    assert_eq!(kdl("{a = {b = {c = 1}}}"), "a {\n  b {\n    c 1\n  }\n}");
}

#[test]
fn a_list_element_takes_the_item_name() {
    assert_eq!(kdl("[1]"), "_item 1");
    assert_eq!(kdl("{a = [1 2]}"), "a {\n  _item 1\n  _item 2\n}");
    assert_eq!(kdl("[[1]]"), "_item {\n  _item 1\n}");
    assert_ne!(kdl("{a = 1}"), kdl("{a = [1]}"));
}

#[test]
fn a_key_is_a_string_or_an_atom() {
    assert_eq!(kdl(r#"{"ll" => "eza -l"}"#), "\"ll\" \"eza -l\"");
    assert_eq!(kdl(r#"{"a b" => 1}"#), "\"a b\" 1");
    assert_eq!(kdl(r#"{"" => 1}"#), "\"\" 1");
    for (source, form) in [
        ("{42 => 1}", "integer"),
        ("{1.5 => 1}", "float"),
        ("{[1] => 1}", "list"),
        ("{{a = 1} => 1}", "tuple"),
        (r#"{{"n" => 1} => 1}"#, "map"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableKey(form),
            "{error}"
        );
        assert_eq!(error.path().to_string(), "#1");
    }
}

#[test]
fn a_key_keeps_its_atom_spelling_rather_than_its_value() {
    assert_eq!(kdl("{True => 1}"), "True 1");
    assert_eq!(kdl("{Relative => 1}"), "Relative 1");
}

#[test]
fn two_keys_of_one_spelling_write_two_nodes_rather_than_being_refused() {
    assert_eq!(
        kdl(r#"{Relative => 1 "Relative" => 2}"#),
        "Relative 1\n\"Relative\" 2"
    );
    assert_eq!(kdl("{a = 1}"), "a 1");
    assert_eq!(kdl(r#"{"a" => 1}"#), "\"a\" 1");
}

#[test]
fn strings_take_the_kdl_escapes_and_no_character_is_refused() {
    assert_eq!(scalar(r#""\"""#), r#""\"""#);
    assert_eq!(scalar(r#""\\""#), r#""\\""#);
    assert_eq!(scalar(r#""\n\r\t""#), r#""\n\r\t""#);
    assert_eq!(scalar(r#""é ✓ 😀""#), r#""é ✓ 😀""#);
    assert_eq!(scalar(r#""a # b""#), r#""a # b""#);
    assert_eq!(scalar(r#""\u{0}""#), r#""\u{0}""#);
    assert_eq!(scalar(r#""\u{1F}""#), r#""\u{1F}""#);
    assert_eq!(scalar(r#""\u{7F}""#), r#""\u{7F}""#);
    assert_eq!(scalar(r#""\u{85}""#), r#""\u{85}""#);
    assert_eq!(scalar(r#""\u{2028}""#), r#""\u{2028}""#);
    assert_eq!(scalar(r#""\u{FEFF}""#), r#""\u{FEFF}""#);
    assert_eq!(scalar(r#""\u{FFFF}""#), "\"\u{FFFF}\"");
    assert_eq!(scalar(r#""\u{10FFFF}""#), "\"\u{10FFFF}\"");
}

#[test]
fn a_float_keeps_its_shape_and_reads_back_as_the_same_double() {
    assert_eq!(scalar("0.0"), "0.0");
    assert_eq!(scalar("-0.0"), "-0.0");
    assert_eq!(scalar("1e5"), "100000.0");
    assert_eq!(scalar("1e300"), "1e300");
    for source in ["0.0", "-0.0", "1.5", "1e5", "1e-5", "-2.5e-3", "1e300"] {
        let written = scalar(source);
        assert!(
            written.contains('.') || written.contains('e'),
            "{written} reads back as an integer"
        );
        let read: f64 = written
            .parse()
            .unwrap_or_else(|_| panic!("{written} is a double"));
        let want: f64 = source.parse().expect("the source is a double");
        assert_eq!(read.to_bits(), want.to_bits(), "for {source}");
    }
}

#[test]
fn an_integer_keeps_every_digit_it_came_with() {
    let wide = "170141183460469231731687303715884105728";
    assert_eq!(scalar(wide), wide);
    assert_eq!(scalar("9223372036854775807"), "9223372036854775807");
}

#[test]
fn a_dot_file_lays_out_the_way_a_hand_written_kdl_file_would() {
    let source = include_str!("../../../fixtures/valid/dotfile.nuke");
    assert_eq!(
        kdl(source),
        r#"editor {
  theme "gruvbox-dark"
  tab_width 2
  line_numbers Relative
}
shell {
  aliases {
    "ll" "eza -l"
    "gs" "git status"
  }
  history_size 10000
}
keybindings {
  _item {
    keys "ctrl+t"
    action OpenTerminal
  }
  _item {
    keys "ctrl+w"
    action CloseWindow
  }
}"#
    );
}

#[test]
fn a_map_reads_as_the_run_of_named_nodes_a_kdl_config_is_made_of() {
    assert_eq!(
        kdl(r#"{"Ctrl g" => {action = SwitchToMode mode = "locked"} "Ctrl b" => {action = Quit}}"#),
        r#""Ctrl g" {
  action SwitchToMode
  mode "locked"
}
"Ctrl b" {
  action Quit
}"#
    );
}

#[test]
fn a_tuple_shaped_like_the_backends_own_vocabulary_still_writes_as_a_tuple() {
    assert_ne!(kdl("{a = {item = 1}}"), kdl("{a = [1]}"));
    assert_eq!(kdl("{a = {item = 1}}"), "a {\n  item 1\n}");
}

#[test]
fn every_layout_the_writer_chooses_is_kdl_a_real_parser_loads() {
    for source in [
        "{a = 1 b = {c = 2} d = {e = {f = 3}}}",
        "{k = [{a = 1} {a = 2}] z = {y = 1}}",
        r#"{"a.b" => {c = 1}}"#,
        r#"{"a b" => {"c d" => [{e = 1}]}}"#,
        "{a = {} b = [] c = [{}] d = [[1] [2]]}",
        r#"{a = "\u{0}\u{7F}\u{FEFF}\u{10FFFF}" b = ["x" Relative Null True]}"#,
        "{true = 1 inf = 2 nan = 3}",
        r#"{Relative => 1 "Relative" => 2}"#,
        "{a = {b = {c = {d = [{e = 1}]}}}}",
        "[1 -2 3.5 -0.0 True Null]",
    ] {
        let written = kdl(source);
        kdl::KdlDocument::parse_v2(&written).unwrap_or_else(|error| {
            panic!("{source} wrote KDL that does not parse: {error}\n{written}")
        });
    }
}

#[test]
fn a_value_nested_past_the_parsers_own_limit_is_refused_rather_than_overflowing() {
    let mut value = nuke_syntax::Value::List(Vec::new());
    for _ in 1..nuke_syntax::MAX_DEPTH + 2 {
        value = nuke_syntax::Value::List(vec![value]);
    }
    let error = to_string(&value).expect_err("the writer should refuse it");
    assert_eq!(error.kind(), &ErrorKind::TooDeep);
}
