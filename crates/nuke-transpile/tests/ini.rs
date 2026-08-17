use ini::{Ini, Properties};
use nuke_syntax::{Ident, Tuple, Value, parse};
use nuke_transpile::Segment;
use nuke_transpile::ini::{Error, ErrorKind, to_string};

fn ini(source: &str) -> String {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect("the value should transpile")
}

fn entry(source: &str) -> String {
    let written = ini(&format!("{{s = {{v = {source}}}}}"));
    written
        .strip_prefix("[s]\nv = ")
        .expect("a scalar is written as the value of one pair under one header")
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
        assert!(error.path().is_root(), "for {source}");
    }
    assert_eq!(ini("{}"), "");
}

#[test]
fn keys_lead_and_sections_follow_in_the_order_they_were_declared() {
    assert_eq!(ini("{a = 1 b = 2}"), "a = 1\nb = 2");
    assert_eq!(ini("{a = 1 b = {c = 2}}"), "a = 1\n\n[b]\nc = 2");
    assert_eq!(ini("{a = {b = 1} c = {d = 2}}"), "[a]\nb = 1\n\n[c]\nd = 2");
    assert_eq!(ini("{a = {}}"), "[a]");
    assert_eq!(ini("{a = {} b = {c = 1}}"), "[a]\n\n[b]\nc = 1");
}

#[test]
fn a_key_after_a_section_is_refused_rather_than_hoisted_above_it() {
    let error = refused("{a = {b = 1} c = 2}");
    assert_eq!(
        error.kind(),
        &ErrorKind::StrayKey("c".to_owned()),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "c");
}

#[test]
fn a_list_has_no_ini_spelling_in_any_position() {
    for (source, path) in [
        ("{a = [1]}", "a"),
        ("{s = {a = [1]}}", "s.a"),
        ("{a = {b = 1} c = [1]}", "c"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableValue("list"),
            "{error}"
        );
        assert_eq!(error.path().to_string(), path, "for {source}");
    }
}

#[test]
fn a_table_below_a_section_is_refused_because_ini_is_two_levels_deep() {
    let error = refused("{a = {b = {c = 1}}}");
    assert_eq!(
        error.kind(),
        &ErrorKind::UnrepresentableValue("tuple"),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "a.b");

    let error = refused(r#"{a = {b = {"c" => 1}}}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::UnrepresentableValue("map"),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "a.b");
}

#[test]
fn every_scalar_goes_out_as_its_own_text_and_no_two_stay_apart() {
    assert_eq!(entry("True"), "True");
    assert_eq!(entry("False"), "False");
    assert_eq!(entry("Null"), "Null");
    assert_eq!(entry("Relative"), "Relative");
    assert_eq!(entry(r#""text""#), "text");
    assert_eq!(entry("0"), "0");
    assert_eq!(entry("-0"), "0");
    assert_eq!(entry("-7"), "-7");
    assert_eq!(
        entry("170141183460469231731687303715884105728"),
        "170141183460469231731687303715884105728"
    );
    assert_eq!(entry("1.5"), "1.5");
    assert_eq!(entry("-0.0"), "-0.0");
    assert_eq!(entry("1e5"), "100000.0");
    assert_eq!(entry("1e300"), "1e300");

    assert_eq!(entry("42"), entry(r#""42""#));
    assert_eq!(entry("True"), entry(r#""True""#));
    assert_eq!(entry("1.0"), entry(r#""1.0""#));
}

#[test]
fn the_empty_string_writes_a_key_with_nothing_after_it() {
    assert_eq!(ini(r#"{s = {v = ""}}"#), "[s]\nv =");
}

#[test]
fn a_value_ini_has_no_escape_for_is_refused_by_the_character_itself() {
    for (source, character) in [
        (r#""\n""#, '\n'),
        (r#""\r""#, '\r'),
        (r#""\t""#, '\t'),
        (r#""\u{0}""#, '\u{0}'),
        (r#""\u{7F}""#, '\u{7F}'),
        (r#""\u{85}""#, '\u{85}'),
        (r#""\u{2028}""#, '\u{2028}'),
        (r#""\u{2029}""#, '\u{2029}'),
        (r#""\\""#, '\\'),
    ] {
        let error = refused(&format!("{{s = {{v = {source}}}}}"));
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableCharacter(character),
            "{error}"
        );
        assert_eq!(error.path().to_string(), "s.v", "for {source}");
    }
}

#[test]
fn a_value_a_reader_would_not_give_back_whole_is_refused_by_its_text() {
    for (source, text) in [
        (r#""\"x\"""#, "\"x\""),
        (r#""'x'""#, "'x'"),
        (r#"" x""#, " x"),
        (r#""x ""#, "x "),
        (r#""\u{A0}x""#, "\u{A0}x"),
        (r#""dark # comment""#, "dark # comment"),
        (r#""dark ; comment""#, "dark ; comment"),
    ] {
        let error = refused(&format!("{{s = {{v = {source}}}}}"));
        assert_eq!(
            error.kind(),
            &ErrorKind::UnspellableValue(text.to_owned()),
            "{error}"
        );
        assert_eq!(error.path().to_string(), "s.v", "for {source}");
    }
}

#[test]
fn the_characters_only_the_start_of_a_line_makes_syntax_stand_in_a_value() {
    assert_eq!(entry(r##""#ffffff""##), "#ffffff");
    assert_eq!(entry(r#"";x""#), ";x");
    assert_eq!(entry(r#""a#b""#), "a#b");
    assert_eq!(entry(r#""a[b]c=d:e""#), "a[b]c=d:e");
    assert_eq!(entry(r#""50%""#), "50%");
    assert_eq!(entry(r#""é ✓ 😀""#), "é ✓ 😀");
}

#[test]
fn a_name_is_narrower_than_a_string_and_narrower_than_a_value() {
    assert_eq!(ini(r#"{"ll" => {a = 1}}"#), "[ll]\na = 1");
    assert_eq!(ini(r#"{"a b" => {a = 1}}"#), "[a b]\na = 1");
    assert_eq!(ini(r#"{"42" => {a = 1}}"#), "[42]\na = 1");
    assert_eq!(ini("{Relative => {a = 1}}"), "[Relative]\na = 1");

    for name in [
        r#""""#,
        r#""a[b""#,
        r#""a]b""#,
        r#""a=b""#,
        r#""a:b""#,
        r##""#a""##,
        r#"";a""#,
        r#"" a""#,
        r#""a ""#,
        r#""DEFAULT""#,
        r#""a # b""#,
    ] {
        let error = refused(&format!("{{{name} => {{a = 1}}}}"));
        assert!(
            matches!(error.kind(), ErrorKind::UnspellableName(_)),
            "{name} should be unspellable, not {error}"
        );
        assert_eq!(error.path().to_string(), "#1", "for {name}");
    }

    let error = refused(r#"{"a\\b" => {a = 1}}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::UnrepresentableCharacter('\\'),
        "{error}"
    );
}

#[test]
fn a_key_ini_cannot_name_is_refused_and_two_names_it_folds_together_are_too() {
    let error = refused("{42 => {a = 1}}");
    assert_eq!(
        error.kind(),
        &ErrorKind::UnrepresentableKey("integer"),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "#1");

    let error = refused(r#"{Relative => {a = 1} "Relative" => {b = 2}}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::DuplicateKey("Relative".to_owned()),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "#2");
    assert_eq!(
        error.path().segments().last(),
        Some(&Segment::Entry(1)),
        "the entry, not its key, is what can be named"
    );

    let error = refused(r#"{a = {Theme => 1 "theme" => 2}}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::DuplicateKey("theme".to_owned()),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "a#2");
}

#[test]
fn a_dot_file_cannot_be_an_ini_file_because_one_of_its_sections_holds_a_table() {
    let source = include_str!("../../../fixtures/valid/dotfile.nuke");
    let error = refused(source);
    assert_eq!(
        error.kind(),
        &ErrorKind::UnrepresentableValue("map"),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "shell.aliases");
}

#[test]
fn a_flat_document_lays_out_the_way_a_hand_written_ini_file_would() {
    let source = r##"
{
  root = True
  depth = 1

  core = {
    editor = "hx"
    ratio = 0.5
    theme = "#ffffff"
  }

  user = {
    Name => "nuke"
    "email" => Null
  }
}
"##;
    assert_eq!(
        ini(source),
        "\
root = True
depth = 1

[core]
editor = hx
ratio = 0.5
theme = #ffffff

[user]
Name = nuke
email = Null"
    );
}

#[test]
fn what_the_backend_writes_is_the_document_a_real_ini_parser_reads_back() {
    for source in [
        "{}",
        "{a = 1 b = 2}",
        "{a = {b = 1}}",
        "{a = {} b = {c = 1}}",
        "{root = True depth = 1 core = {editor = \"hx\" ratio = 0.5 theme = \"#ffffff\"}}",
        r#"{"a b" => {"c d" => "e f"}}"#,
        r#"{Relative => {True => False Null => Atom99}}"#,
        r#"{s = {v = ""}}"#,
        r##"{s = {a = "#ffffff" b = "x;y" c = "p=q" d = "50%" e = "a[b]" f = "a#b"}}"##,
        "{s = {n = -0.0 m = 1e300 k = 9007199254740993 w = 170141183460469231731687303715884105728}}",
        r#"{s = {v = "é ✓ 😀"}}"#,
        r#"{"42" => {"0" => "1"}}"#,
    ] {
        let value = parse(source).expect("the source should parse");
        let written = to_string(&value).expect("the value should transpile");
        let loaded = Ini::load_from_str(&written).unwrap_or_else(|error| {
            panic!("{source} wrote INI that does not parse: {error}\n{written}")
        });
        reads(&value, &loaded, source);
    }
}

#[test]
fn a_value_nested_past_what_ini_holds_stops_at_the_first_level_rather_than_the_deepest() {
    let mut value = Value::List(Vec::new());
    for _ in 1..nuke_syntax::MAX_DEPTH + 2 {
        value = Value::List(vec![value]);
    }
    let mut tuple = Tuple::new();
    let name = Ident::parse("a").expect("a is an identifier");
    tuple.insert(name, value);
    let error = to_string(&Value::Tuple(tuple)).expect_err("the writer should refuse it");
    assert_eq!(error.kind(), &ErrorKind::UnrepresentableValue("list"));
    assert_eq!(
        error.path().to_string(),
        "a",
        "INI is two levels deep, so no depth limit can be reached"
    );
}

fn reads(value: &Value, loaded: &Ini, source: &str) {
    let entries = spread(value);
    let split = entries
        .iter()
        .position(|(_, item)| matches!(item, Value::Tuple(_) | Value::Map(_)))
        .unwrap_or(entries.len());

    let mut found = loaded.iter();
    let (name, properties) = found
        .next()
        .unwrap_or_else(|| panic!("{source} has no general section"));
    assert_eq!(name, None, "{source} should open with the general section");
    pairs(&entries[..split], properties, "the document", source);

    for (name, item) in &entries[split..] {
        let (read, properties) = found
            .next()
            .unwrap_or_else(|| panic!("{source} lost the section {name}"));
        assert_eq!(read, Some(name.as_str()), "{source} renamed a section");
        pairs(&spread(item), properties, name, source);
    }
    assert!(
        found.next().is_none(),
        "{source} read back a section it never wrote"
    );
}

fn pairs(expected: &[(String, &Value)], properties: &Properties, at: &str, source: &str) {
    assert_eq!(
        properties.iter().count(),
        expected.len(),
        "{source}: {at} lost a pair"
    );
    for ((name, item), (key, text)) in expected.iter().zip(properties.iter()) {
        assert_eq!(key, name, "{source}: {at} renamed a key");
        means(item, text, &format!("{source}: {at}.{name}"));
    }
}

fn spread(value: &Value) -> Vec<(String, &Value)> {
    match value {
        Value::Tuple(tuple) => tuple
            .iter()
            .map(|(name, item)| (name.as_str().to_owned(), item))
            .collect(),
        Value::Map(map) => map.iter().map(|(key, item)| (named(key), item)).collect(),
        other => panic!("{other:?} is not a table"),
    }
}

fn named(key: &Value) -> String {
    match key {
        Value::String(text) => text.clone(),
        Value::Atom(atom) => atom.as_str().to_owned(),
        other => panic!("INI cannot name {other:?}"),
    }
}

fn means(value: &Value, text: &str, at: &str) {
    match value {
        Value::Atom(atom) => assert_eq!(text, atom.as_str(), "at {at}"),
        Value::String(item) => assert_eq!(text, item, "at {at}"),
        Value::Integer(integer) => assert_eq!(text, integer.as_str(), "at {at}"),
        Value::Float(number) => assert_eq!(
            text.parse::<f64>().map(f64::to_bits),
            Ok(number.get().to_bits()),
            "at {at}"
        ),
        other => panic!("{at} is not a scalar: {other:?}"),
    }
}
