use nuke_syntax::parse;
use nuke_transpile::nix::{Error, ErrorKind, to_string};

fn nix(source: &str) -> String {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect("the value should transpile")
}

fn scalar(source: &str) -> String {
    let written = nix(&format!("{{v = {source}}}"));
    written
        .strip_prefix("{\n  v = ")
        .and_then(|rest| rest.strip_suffix(";\n}"))
        .expect("a scalar is written as the one field of an attribute set")
        .to_owned()
}

fn refused(source: &str) -> Error {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect_err("the value should be refused")
}

fn reads(written: &str, source: &str) {
    let parsed = rnix::Root::parse(written);
    assert!(
        parsed.errors().is_empty(),
        "{source} wrote Nix that does not parse: {:?}\n{written}",
        parsed.errors()
    );
}

#[test]
fn a_document_is_one_expression_and_any_value_opens_it() {
    assert_eq!(nix("42"), "42");
    assert_eq!(nix("True"), "true");
    assert_eq!(nix("Null"), "null");
    assert_eq!(nix(r#""text""#), r#""text""#);
    assert_eq!(nix("-1.5"), "-1.5");
    assert_eq!(nix("{}"), "{ }");
    assert_eq!(nix("[]"), "[ ]");
    assert_eq!(nix("{a = 1}"), "{\n  a = 1;\n}");
}

#[test]
fn scalars_carry_their_nix_spelling() {
    assert_eq!(scalar("0"), "0");
    assert_eq!(scalar("-0"), "0");
    assert_eq!(scalar("42"), "42");
    assert_eq!(scalar("-7"), "-7");
    assert_eq!(scalar(r#""text""#), r#""text""#);
    assert_eq!(scalar(r#""a b c""#), r#""a b c""#);
    assert_eq!(scalar(r#""""#), r#""""#);
}

#[test]
fn an_atom_nix_has_a_word_for_takes_it_and_every_other_becomes_a_string() {
    assert_eq!(scalar("True"), "true");
    assert_eq!(scalar("False"), "false");
    assert_eq!(scalar("Null"), "null");
    assert_eq!(scalar("Relative"), r#""Relative""#);
    assert_eq!(scalar("Atom99"), r#""Atom99""#);
    assert_eq!(scalar("Relative"), scalar(r#""Relative""#));
    assert_eq!(
        nix("[OpenTerminal CloseWindow]"),
        "[\n  \"OpenTerminal\"\n  \"CloseWindow\"\n]"
    );
}

#[test]
fn a_negative_number_in_a_list_is_parenthesised_because_juxtaposition_is_application() {
    assert_eq!(nix("[-1]"), "[\n  (-1)\n]");
    assert_eq!(nix("[-1.5]"), "[\n  (-1.5)\n]");
    assert_eq!(nix("[-0.0]"), "[\n  (-0.0)\n]");
    assert_eq!(nix("[1 -2]"), "[\n  1\n  (-2)\n]");
    assert_eq!(nix("[-0]"), "[\n  0\n]");
    assert_eq!(nix("[[-1]]"), "[\n  [\n    (-1)\n  ]\n]");
    assert_eq!(scalar("-1"), "-1");
    assert_eq!(nix("{a = -1.5}"), "{\n  a = -1.5;\n}");
    reads(&nix("[-1 -1.5 -0.0 1]"), "[-1 -1.5 -0.0 1]");
}

#[test]
fn a_field_name_nix_reserves_is_quoted_and_every_other_is_bare() {
    for word in [
        "assert", "else", "if", "in", "inherit", "let", "rec", "then", "with",
    ] {
        assert_eq!(
            nix(&format!("{{{word} = 1}}")),
            format!("{{\n  \"{word}\" = 1;\n}}"),
            "for {word}"
        );
    }
    for word in ["or", "true", "false", "null"] {
        assert_eq!(
            nix(&format!("{{{word} = 1}}")),
            format!("{{\n  {word} = 1;\n}}"),
            "for {word}"
        );
    }
    assert_eq!(
        nix("{ending = 1 i = 2 if_ = 3}"),
        "{\n  ending = 1;\n  i = 2;\n  if_ = 3;\n}"
    );
    assert_eq!(nix("{snake_case_1 = Null}"), "{\n  snake_case_1 = null;\n}");
}

#[test]
fn an_empty_collection_of_every_kind_keeps_a_space_inside_its_brackets() {
    assert_eq!(
        nix(r#"{a = {} b = [] c = ""}"#),
        "{\n  a = { };\n  b = [ ];\n  c = \"\";\n}"
    );
    assert_eq!(nix("[[] {}]"), "[\n  [ ]\n  { }\n]");
    assert_eq!(nix("{a = [[]]}"), "{\n  a = [\n    [ ]\n  ];\n}");
}

#[test]
fn a_list_holds_its_elements_side_by_side_and_a_tuple_names_them() {
    assert_eq!(nix("[1 2]"), "[\n  1\n  2\n]");
    assert_eq!(nix("{b = 1 a = 2}"), "{\n  b = 1;\n  a = 2;\n}");
    assert_eq!(nix("{a = {b = 1}}"), "{\n  a = {\n    b = 1;\n  };\n}");
    assert_eq!(nix("[[1]]"), "[\n  [\n    1\n  ]\n]");
    assert_ne!(nix("{a = 1}"), nix("{a = [1]}"));
}

#[test]
fn a_key_is_a_string_or_an_atom_and_nothing_else_names_an_attribute() {
    assert_eq!(nix(r#"{"ll" => "eza -l"}"#), "{\n  \"ll\" = \"eza -l\";\n}");
    assert_eq!(nix(r#"{"a b" => 1}"#), "{\n  \"a b\" = 1;\n}");
    assert_eq!(nix(r#"{"" => 1}"#), "{\n  \"\" = 1;\n}");
    assert_eq!(nix(r#"{"a.b" => 1}"#), "{\n  \"a.b\" = 1;\n}");
    for (source, form) in [
        ("{42 => 1}", "integer"),
        ("{-1.5 => 1}", "float"),
        ("{[1] => 1}", "list"),
        ("{{a = 1} => 1}", "tuple"),
        (r#"{{"n" => 1} => 1}"#, "map"),
        ("{[] => 1}", "list"),
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
    assert_eq!(nix("{True => 1}"), "{\n  \"True\" = 1;\n}");
    assert_eq!(nix("{Null => 1}"), "{\n  \"Null\" = 1;\n}");
    assert_eq!(nix("{Relative => 1}"), "{\n  \"Relative\" = 1;\n}");
    assert_eq!(nix("{True => 1}"), nix(r#"{"True" => 1}"#));
    assert_ne!(nix("{True => 1}"), nix("{v = True}"));
}

#[test]
fn two_keys_that_name_one_attribute_are_refused_and_the_ones_nix_keeps_apart_are_not() {
    for (source, name) in [
        (r#"{Relative => 1 "Relative" => 2}"#, "Relative"),
        (r#"{True => 1 "True" => 2}"#, "True"),
        (r#"{Null => 1 "Null" => 2}"#, "Null"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::DuplicateKey(name.to_owned()),
            "{error}"
        );
        assert_eq!(error.path().to_string(), "#2");
    }
    assert_eq!(
        nix(r#"{"True" => 1 "true" => 2}"#),
        "{\n  \"True\" = 1;\n  \"true\" = 2;\n}"
    );
    assert_eq!(
        nix(r#"{"a" => 1}"#),
        nix("{a = 1}").replace("a =", "\"a\" =")
    );
}

#[test]
fn strings_take_the_five_escapes_nix_spells_the_same_way() {
    assert_eq!(scalar(r#""\"""#), r#""\"""#);
    assert_eq!(scalar(r#""\\""#), r#""\\""#);
    assert_eq!(scalar(r#""\n\r\t""#), r#""\n\r\t""#);
    assert_eq!(scalar(r#""a # b""#), r#""a # b""#);
    assert_eq!(scalar(r#""é ✓ 😀""#), r#""é ✓ 😀""#);
}

#[test]
fn an_antiquotation_a_string_would_open_is_escaped_and_a_bare_dollar_is_not() {
    assert_eq!(scalar(r#""${x}""#), r#""\${x}""#);
    assert_eq!(scalar(r#""$HOME""#), r#""$HOME""#);
    assert_eq!(scalar(r#""a $ b""#), r#""a $ b""#);
    assert_eq!(scalar(r#""$""#), r#""$""#);
    assert_eq!(scalar(r#""$$ {}""#), r#""$$ {}""#);
    assert_eq!(scalar(r#""\\${x}""#), r#""\\\${x}""#);
    assert_eq!(nix(r#"{"${x}" => 1}"#), "{\n  \"\\${x}\" = 1;\n}");
    reads(&nix(r#"["${x}" "$" "$$ {}"]"#), "antiquotation");
}

#[test]
fn a_character_nix_cannot_name_is_carried_as_itself_and_only_the_null_one_is_refused() {
    assert_eq!(scalar(r#""\u{1}""#), "\"\u{1}\"");
    assert_eq!(scalar(r#""\u{1F}\u{7F}""#), "\"\u{1F}\u{7F}\"");
    assert_eq!(
        scalar(r#""\u{85}\u{2028}\u{FEFF}\u{10FFFF}""#),
        "\"\u{85}\u{2028}\u{FEFF}\u{10FFFF}\""
    );
    assert!(
        !scalar(r#""\u{1}\u{7F}""#).contains('\\'),
        "a control character is escaped where Nix has no escape"
    );
    for source in [r#"{a = "\u{0}"}"#, r#"{a = "x\u{0}y"}"#] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableCharacter('\u{0}'),
            "{error}"
        );
        assert_eq!(error.path().to_string(), "a");
    }
    let error = refused(r#"{"\u{0}" => 1}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::UnrepresentableCharacter('\u{0}'),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "#1");
}

#[test]
fn an_integer_outside_the_range_a_minus_sign_leaves_writable_is_refused() {
    assert_eq!(scalar("9223372036854775807"), "9223372036854775807");
    assert_eq!(scalar("-9223372036854775807"), "-9223372036854775807");
    for digits in [
        "9223372036854775808",
        "-9223372036854775808",
        "170141183460469231731687303715884105728",
    ] {
        let error = refused(&format!("{{a = {digits}}}"));
        assert_eq!(
            error.kind(),
            &ErrorKind::WideInteger(digits.to_owned()),
            "{error}"
        );
        assert_eq!(error.path().to_string(), "a");
    }
}

#[test]
fn a_float_always_carries_a_point_in_its_mantissa_and_reads_back_as_the_same_double() {
    assert_eq!(scalar("0.0"), "0.0");
    assert_eq!(scalar("-0.0"), "-0.0");
    assert_eq!(scalar("1e5"), "100000.0");
    assert_eq!(scalar("1e300"), "1.0e300");
    assert_eq!(scalar("3e-308"), "3.0e-308");
    for source in [
        "0.0", "-0.0", "1.5", "1e5", "1e-5", "-2.5e-3", "1e300", "3e-308",
    ] {
        let written = scalar(source);
        let mantissa = written
            .split_once('e')
            .map_or(written.as_str(), |(mantissa, _)| mantissa);
        assert!(mantissa.contains('.'), "{written} reads back as an integer");
        let read: f64 = written
            .parse()
            .unwrap_or_else(|_| panic!("{written} is a double"));
        let want: f64 = source.parse().expect("the source is a double");
        assert_eq!(read.to_bits(), want.to_bits(), "for {source}");
    }
}

#[test]
fn a_float_too_small_for_nix_to_read_back_is_refused() {
    for source in ["5e-324", "1e-310", "1e-308"] {
        let error = refused(&format!("{{a = {source}}}"));
        let ErrorKind::SubnormalFloat(digits) = error.kind() else {
            panic!("{error}");
        };
        let read: f64 = digits.parse().expect("the digits are a double");
        let want: f64 = source.parse().expect("the source is a double");
        assert_eq!(read.to_bits(), want.to_bits(), "for {source}");
        assert_eq!(error.path().to_string(), "a");
    }
}

#[test]
fn a_dot_file_lays_out_the_way_a_hand_written_nix_file_would() {
    let source = include_str!("../../../fixtures/valid/dotfile.nuke");
    assert_eq!(
        nix(source),
        r#"{
  editor = {
    theme = "gruvbox-dark";
    tab_width = 2;
    line_numbers = "Relative";
  };
  shell = {
    aliases = {
      "ll" = "eza -l";
      "gs" = "git status";
    };
    history_size = 10000;
  };
  keybindings = [
    {
      keys = "ctrl+t";
      action = "OpenTerminal";
    }
    {
      keys = "ctrl+w";
      action = "CloseWindow";
    }
  ];
}"#
    );
}

#[test]
fn every_layout_the_writer_chooses_is_nix_a_real_parser_reads() {
    for source in [
        "{a = 1 b = {c = 2} d = {e = {f = 3}}}",
        "{k = [{a = 1} {a = 2}] z = {y = 1}}",
        r#"{"a.b" => {c = 1}}"#,
        r#"{"a b" => {"c d" => [{e = 1}]}}"#,
        "{a = {} b = [] c = [{}] d = [[1] [2]]}",
        r#"{a = "\u{1}\u{7F}\u{FEFF}\u{10FFFF}" b = ["x" Relative Null True]}"#,
        "{assert = 1 inherit = 2 rec = 3 or = 4 true = 5}",
        r#"{True => 1 "true" => 2 Relative => 3}"#,
        "{a = {b = {c = {d = [{e = 1}]}}}}",
        "[1 -2 3.5 -0.0 True Null]",
        "[1e300 -1e300 1e-5]",
        r#"["${x}" "a\"b" "a\\b" "a\nb"]"#,
        "42",
        "{}",
        "[]",
    ] {
        reads(&nix(source), source);
    }
}

#[test]
fn the_deepest_document_the_backend_writes_is_one_a_real_parser_still_reads() {
    let mut value = nuke_syntax::Value::List(Vec::new());
    for _ in 1..nuke_syntax::MAX_DEPTH {
        value = nuke_syntax::Value::List(vec![value]);
    }
    let written = to_string(&value).expect("the deepest document should transpile");
    reads(&written, "the deepest document");
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
