use nuke_syntax::parse;
use nuke_transpile::lua::{Error, ErrorKind, to_string};

fn lua(source: &str) -> String {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect("the value should transpile")
}

fn scalar(source: &str) -> String {
    let written = lua(&format!("{{v = {source}}}"));
    written
        .strip_prefix("return {\n  v = ")
        .and_then(|rest| rest.strip_suffix(",\n}"))
        .expect("a scalar is written as the one field of a table")
        .to_owned()
}

fn refused(source: &str) -> Error {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect_err("the value should be refused")
}

#[test]
fn a_document_is_a_chunk_returning_any_value_at_all() {
    assert_eq!(lua("42"), "return 42");
    assert_eq!(lua("True"), "return true");
    assert_eq!(lua("Null"), "return \"Null\"");
    assert_eq!(lua(r#""text""#), "return \"text\"");
    assert_eq!(lua("-1.5"), "return -1.5");
    assert_eq!(lua("{}"), "return {}");
    assert_eq!(lua("[]"), "return {}");
    assert_eq!(lua("{a = 1}"), "return {\n  a = 1,\n}");
}

#[test]
fn scalars_carry_their_lua_spelling() {
    assert_eq!(scalar("0"), "0");
    assert_eq!(scalar("-0"), "0");
    assert_eq!(scalar("42"), "42");
    assert_eq!(scalar("-7"), "-7");
    assert_eq!(scalar(r#""text""#), r#""text""#);
    assert_eq!(scalar(r#""a b c""#), r#""a b c""#);
    assert_eq!(scalar(r#""""#), r#""""#);
}

#[test]
fn an_atom_lua_has_a_word_for_takes_it_and_every_other_becomes_a_string() {
    assert_eq!(scalar("True"), "true");
    assert_eq!(scalar("False"), "false");
    assert_eq!(scalar("Null"), r#""Null""#);
    assert_eq!(scalar("Relative"), r#""Relative""#);
    assert_eq!(scalar("Atom99"), r#""Atom99""#);
    assert_eq!(scalar("Relative"), scalar(r#""Relative""#));
    assert_eq!(
        lua("[OpenTerminal CloseWindow]"),
        "return {\n  \"OpenTerminal\",\n  \"CloseWindow\",\n}"
    );
}

#[test]
fn a_field_name_lua_reserves_is_bracketed_and_every_other_is_bare() {
    for word in [
        "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if",
        "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
    ] {
        assert_eq!(
            lua(&format!("{{{word} = 1}}")),
            format!("return {{\n  [\"{word}\"] = 1,\n}}"),
            "for {word}"
        );
    }
    assert_eq!(
        lua("{ending = 1 e = 2 end_ = 3}"),
        "return {\n  ending = 1,\n  e = 2,\n  end_ = 3,\n}"
    );
    assert_eq!(
        lua("{snake_case_1 = Null}"),
        "return {\n  snake_case_1 = \"Null\",\n}"
    );
}

#[test]
fn an_empty_collection_of_every_kind_is_an_empty_table() {
    assert_eq!(
        lua(r#"{a = {} b = [] c = ""}"#),
        "return {\n  a = {},\n  b = {},\n  c = \"\",\n}"
    );
    assert_eq!(lua("[[] {}]"), "return {\n  {},\n  {},\n}");
    assert_eq!(lua("{a = [[]]}"), "return {\n  a = {\n    {},\n  },\n}");
}

#[test]
fn a_list_writes_positional_fields_and_a_tuple_writes_named_ones() {
    assert_eq!(lua("[1 2]"), "return {\n  1,\n  2,\n}");
    assert_eq!(lua("{b = 1 a = 2}"), "return {\n  b = 1,\n  a = 2,\n}");
    assert_eq!(
        lua("{a = {b = 1}}"),
        "return {\n  a = {\n    b = 1,\n  },\n}"
    );
    assert_eq!(lua("[[1]]"), "return {\n  {\n    1,\n  },\n}");
    assert_ne!(lua("{a = 1}"), lua("{a = [1]}"));
}

#[test]
fn a_key_is_any_scalar_and_a_collection_keys_by_identity_so_it_is_refused() {
    assert_eq!(
        lua(r#"{"ll" => "eza -l"}"#),
        "return {\n  [\"ll\"] = \"eza -l\",\n}"
    );
    assert_eq!(lua(r#"{"a b" => 1}"#), "return {\n  [\"a b\"] = 1,\n}");
    assert_eq!(lua(r#"{"" => 1}"#), "return {\n  [\"\"] = 1,\n}");
    assert_eq!(lua("{42 => 1}"), "return {\n  [42] = 1,\n}");
    assert_eq!(lua("{-1.5 => 1}"), "return {\n  [-1.5] = 1,\n}");
    for (source, form) in [
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
fn a_key_carries_its_value_rather_than_its_spelling() {
    assert_eq!(lua("{True => 1}"), "return {\n  [true] = 1,\n}");
    assert_eq!(lua("{False => 1}"), "return {\n  [false] = 1,\n}");
    assert_eq!(lua("{Relative => 1}"), "return {\n  [\"Relative\"] = 1,\n}");
    assert_ne!(lua("{True => 1}"), lua(r#"{"True" => 1}"#));
}

#[test]
fn two_keys_lua_folds_into_one_are_refused_and_the_ones_it_keeps_apart_are_not() {
    for (source, spelling) in [
        (r#"{Relative => 1 "Relative" => 2}"#, "[\"Relative\"]"),
        (r#"{Null => 1 "Null" => 2}"#, "[\"Null\"]"),
        ("{1 => 1 1.0 => 2}", "[1.0]"),
        ("{0 => 1 -0.0 => 2}", "[-0.0]"),
        ("{0.0 => 1 -0.0 => 2}", "[-0.0]"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::DuplicateKey(spelling.to_owned()),
            "{error}"
        );
        assert_eq!(error.path().to_string(), "#2");
    }
    assert_eq!(
        lua(r#"{True => 1 "True" => 2}"#),
        "return {\n  [true] = 1,\n  [\"True\"] = 2,\n}"
    );
    assert_eq!(
        lua(r#"{1 => 1 "1" => 2}"#),
        "return {\n  [1] = 1,\n  [\"1\"] = 2,\n}"
    );
    assert_eq!(
        lua(r#"{a = 1}"#),
        lua(r#"{"a" => 1}"#).replace("[\"a\"]", "a")
    );
}

#[test]
fn strings_take_the_escapes_every_lua_from_five_one_reads() {
    assert_eq!(scalar(r#""\"""#), r#""\"""#);
    assert_eq!(scalar(r#""\\""#), r#""\\""#);
    assert_eq!(scalar(r#""\n\r\t""#), r#""\n\r\t""#);
    assert_eq!(scalar(r#""\u{0}""#), r#""\000""#);
    assert_eq!(scalar(r#""\u{1}\u{1F}""#), r#""\001\031""#);
    assert_eq!(scalar(r#""\u{7}\u{8}\u{B}\u{C}""#), r#""\007\008\011\012""#);
    assert_eq!(scalar(r#""\u{7F}""#), r#""\127""#);
    assert_eq!(scalar(r#""\u{0}1""#), r#""\0001""#);
    assert_eq!(scalar(r#""a # b""#), r#""a # b""#);
    assert_eq!(scalar(r#""é ✓ 😀""#), r#""é ✓ 😀""#);
    for source in [
        r#""\u{85}""#,
        r#""\u{2028}""#,
        r#""\u{FEFF}""#,
        r#""\u{10FFFF}""#,
    ] {
        let written = scalar(source);
        assert!(
            !written.contains("\\u"),
            "{written} spells a character the way only Lua 5.3 reads it"
        );
    }
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
fn an_integer_past_the_width_every_lua_agrees_on_is_refused() {
    assert_eq!(scalar("9007199254740992"), "9007199254740992");
    assert_eq!(scalar("-9007199254740992"), "-9007199254740992");
    for digits in [
        "9007199254740993",
        "-9007199254740993",
        "9223372036854775807",
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
    let error = refused("{9007199254740993 => 1}");
    assert_eq!(error.path().to_string(), "#1");
}

#[test]
fn a_dot_file_lays_out_the_way_a_hand_written_lua_file_would() {
    let source = include_str!("../../../fixtures/valid/dotfile.nuke");
    assert_eq!(
        lua(source),
        r#"return {
  editor = {
    theme = "gruvbox-dark",
    tab_width = 2,
    line_numbers = "Relative",
  },
  shell = {
    aliases = {
      ["ll"] = "eza -l",
      ["gs"] = "git status",
    },
    history_size = 10000,
  },
  keybindings = {
    {
      keys = "ctrl+t",
      action = "OpenTerminal",
    },
    {
      keys = "ctrl+w",
      action = "CloseWindow",
    },
  },
}"#
    );
}

#[test]
fn every_layout_the_writer_chooses_is_lua_a_real_interpreter_loads() {
    let state = mlua::Lua::new();
    for source in [
        "{a = 1 b = {c = 2} d = {e = {f = 3}}}",
        "{k = [{a = 1} {a = 2}] z = {y = 1}}",
        r#"{"a.b" => {c = 1}}"#,
        r#"{"a b" => {"c d" => [{e = 1}]}}"#,
        "{a = {} b = [] c = [{}] d = [[1] [2]]}",
        r#"{a = "\u{0}\u{7F}\u{FEFF}\u{10FFFF}" b = ["x" Relative Null True]}"#,
        "{end = 1 goto = 2 nil = 3 function = 4}",
        r#"{True => 1 "True" => 2 42 => 3 -1.5 => 4}"#,
        "{a = {b = {c = {d = [{e = 1}]}}}}",
        "[1 -2 3.5 -0.0 True Null]",
        "42",
        "{}",
    ] {
        let written = lua(source);
        state
            .load(written.as_str())
            .eval::<mlua::Value>()
            .unwrap_or_else(|error| {
                panic!("{source} wrote Lua that does not load: {error}\n{written}")
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
