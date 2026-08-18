use nuke_syntax::parse;
use nuke_transpile::Segment;
use nuke_transpile::plist::{Error, ErrorKind, to_string};

const FIELD: &str = "<key>v</key>\n    ";

fn written(source: &str) -> String {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect("the value should transpile")
}

fn scalar(source: &str) -> String {
    let document = written(&format!("{{v = {source}}}"));
    let opened = document.find(FIELD).expect("the field should be written") + FIELD.len();
    let closed = document
        .rfind("\n  </dict>")
        .expect("the dict should close");
    document[opened..closed].to_owned()
}

fn refused(source: &str) -> Error {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect_err("the value should be refused")
}

fn reads(source: &str) -> plist::Value {
    let document = written(source);
    plist::Value::from_reader_xml(document.as_bytes()).unwrap_or_else(|error| {
        panic!("{source} wrote a plist that does not parse: {error}\n{document}")
    })
}

#[test]
fn the_prologue_names_the_file_a_property_list_rather_than_leaving_it_xml() {
    let document = written("{a = 1}");
    let mut lines = document.lines();
    assert_eq!(
        lines.next(),
        Some(r#"<?xml version="1.0" encoding="UTF-8"?>"#)
    );
    assert_eq!(
        lines.next(),
        Some(
            r#"<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">"#
        )
    );
    assert_eq!(lines.next(), Some(r#"<plist version="1.0">"#));
    assert!(
        document.ends_with("\n</plist>"),
        "the root element should close the file: {document}"
    );
}

#[test]
fn every_scalar_is_wrapped_in_the_element_that_names_its_type() {
    assert_eq!(scalar("True"), "<true/>");
    assert_eq!(scalar("False"), "<false/>");
    assert_eq!(scalar("Null"), "<string>Null</string>");
    assert_eq!(scalar("Relative"), "<string>Relative</string>");
    assert_eq!(scalar("0"), "<integer>0</integer>");
    assert_eq!(scalar("-0"), "<integer>0</integer>");
    assert_eq!(scalar("-7"), "<integer>-7</integer>");
    assert_eq!(scalar("1.5"), "<real>1.5</real>");
    assert_eq!(scalar(r#""""#), "<string></string>");
    assert_eq!(scalar(r#""text""#), "<string>text</string>");
    assert_eq!(scalar(r#""a b c""#), "<string>a b c</string>");
}

#[test]
fn an_atom_and_the_string_that_spells_it_are_two_documents_where_xml_wrote_one() {
    assert_ne!(written("[True]"), written(r#"["True"]"#));
    assert_ne!(written("[False]"), written(r#"["False"]"#));
    assert_ne!(written("[42]"), written(r#"["42"]"#));
    assert_ne!(written("[1.5]"), written(r#"["1.5"]"#));
}

#[test]
fn the_one_erasure_plist_cannot_undo_is_null_because_a_plist_has_no_word_for_it() {
    assert_eq!(written("[Null]"), written(r#"["Null"]"#));
    assert_eq!(scalar("Null"), "<string>Null</string>");
}

#[test]
fn the_three_empties_xml_wrote_alike_are_three_documents() {
    let string = written(r#"[""]"#);
    let tuple = written("[{}]");
    let list = written("[[]]");
    assert_ne!(string, tuple);
    assert_ne!(string, list);
    assert_ne!(tuple, list);
    assert!(string.contains("<string></string>"), "{string}");
    assert!(tuple.contains("<dict/>"), "{tuple}");
    assert!(list.contains("<array/>"), "{list}");
}

#[test]
fn a_tuple_and_a_map_are_both_a_dict_where_xml_kept_them_apart() {
    assert_eq!(written("{a = 1}"), written(r#"{"a" => 1}"#));
    assert_eq!(written("{a = 1 b = 2}"), written(r#"{"a" => 1 "b" => 2}"#));
    assert_ne!(written("{a = 1}"), written(r#"{"b" => 1}"#));
}

#[test]
fn a_map_key_carries_its_text_and_never_a_type() {
    let document = written(r#"{"a b" => 1 True => 2}"#);
    assert!(document.contains("<key>a b</key>"), "{document}");
    assert!(document.contains("<key>True</key>"), "{document}");
}

#[test]
fn a_key_that_is_not_a_name_is_refused_where_json_refuses_one() {
    for (source, form, path) in [
        ("{[1] => 1}", "list", "#1"),
        ("{{a = 1} => 1}", "tuple", "#1"),
        ("{1 => 1}", "integer", "#1"),
        ("{1.5 => 1}", "float", "#1"),
        ("{a = {2 => 1}}", "integer", "a#1"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableKey(form),
            "{error}"
        );
        assert_eq!(error.path().to_string(), path, "for {source}");
    }
}

#[test]
fn two_keys_that_name_one_key_element_are_refused() {
    let error = refused(r#"{Relative => 1 "Relative" => 2}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::DuplicateKey("Relative".to_owned())
    );
    assert_eq!(error.path().to_string(), "#2");
    assert!(matches!(error.path().segments(), [Segment::Entry(1)]));
}

#[test]
fn an_integer_wider_than_the_sixty_four_bits_a_plist_holds_is_refused() {
    for source in [
        "9223372036854775808",
        "-9223372036854775809",
        "170141183460469231731687303715884105728",
    ] {
        let error = refused(&format!("[{source}]"));
        assert_eq!(
            error.kind(),
            &ErrorKind::WideInteger(source.to_owned()),
            "for {source}"
        );
        assert_eq!(error.path().to_string(), "[0]", "for {source}");
    }
    assert_eq!(
        scalar("9223372036854775807"),
        "<integer>9223372036854775807</integer>"
    );
    assert_eq!(
        scalar("-9223372036854775808"),
        "<integer>-9223372036854775808</integer>"
    );
}

#[test]
fn a_float_reads_back_as_the_same_double() {
    for source in ["0.0", "-0.0", "1.5", "1e5", "1e-5", "-2.5e-3", "1e300"] {
        let text = scalar(source);
        let digits = text
            .strip_prefix("<real>")
            .and_then(|rest| rest.strip_suffix("</real>"))
            .unwrap_or_else(|| panic!("{source} should be a real, not {text}"));
        let read: f64 = digits
            .parse()
            .unwrap_or_else(|_| panic!("{digits} is a double"));
        let want: f64 = source.parse().expect("the source is a double");
        assert_eq!(
            read.to_bits(),
            want.to_bits(),
            "{source} read back as {text}"
        );
    }
}

#[test]
fn text_takes_the_xml_escapes_because_a_plist_is_an_xml_document() {
    assert_eq!(scalar(r#""a & b""#), "<string>a &amp; b</string>");
    assert_eq!(scalar(r#""a < b""#), "<string>a &lt; b</string>");
    assert_eq!(scalar(r#""a > b""#), "<string>a &gt; b</string>");
    assert_eq!(scalar(r#""\r""#), "<string>&#xD;</string>");
    assert_eq!(scalar(r#""\n""#), "<string>\n</string>");
    assert_eq!(scalar(r#""\t""#), "<string>\t</string>");
    let document = written(r#"{"a & b" => 1}"#);
    assert!(document.contains("<key>a &amp; b</key>"), "{document}");
}

#[test]
fn a_character_xml_cannot_carry_is_refused_where_it_stands() {
    for (source, character) in [
        (r#"["\u{0}"]"#, '\u{0}'),
        (r#"["\u{8}"]"#, '\u{8}'),
        (r#"["\u{B}"]"#, '\u{B}'),
        (r#"["\u{C}"]"#, '\u{C}'),
        (r#"["\u{1F}"]"#, '\u{1F}'),
        (r#"["\u{FFFE}"]"#, '\u{FFFE}'),
        (r#"["\u{FFFF}"]"#, '\u{FFFF}'),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableCharacter(character),
            "for {source}"
        );
        assert_eq!(error.path().to_string(), "[0]", "for {source}");
    }
    let error = refused(r#"{"\u{0}" => 1}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::UnrepresentableCharacter('\u{0}'),
        "a key is text too"
    );
}

#[test]
fn the_document_is_any_value_because_a_plist_root_wraps_anything() {
    for source in ["1", r#""text""#, "True", "[1 2]", "{a = 1}", "[]", "{}"] {
        let document = written(source);
        assert!(
            document.contains(r#"<plist version="1.0">"#),
            "for {source}"
        );
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

#[test]
fn a_real_plist_parser_reads_every_type_back_as_the_type_that_was_written() {
    let value =
        reads(r#"{flag = True off = False count = 7 ratio = 1.5 name = "n" atom = Relative}"#);
    let dictionary = value.as_dictionary().expect("the root is a dict");
    assert_eq!(dictionary.get("flag"), Some(&plist::Value::Boolean(true)));
    assert_eq!(dictionary.get("off"), Some(&plist::Value::Boolean(false)));
    assert_eq!(
        dictionary.get("count"),
        Some(&plist::Value::Integer(7.into()))
    );
    assert_eq!(dictionary.get("ratio"), Some(&plist::Value::Real(1.5)));
    assert_eq!(
        dictionary.get("name"),
        Some(&plist::Value::String("n".to_owned()))
    );
    assert_eq!(
        dictionary.get("atom"),
        Some(&plist::Value::String("Relative".to_owned()))
    );
}

#[test]
fn a_real_plist_parser_keeps_the_order_a_tuple_was_written_in() {
    let value = reads("{b = 1 a = 2 c = 3}");
    let dictionary = value.as_dictionary().expect("the root is a dict");
    let names: Vec<&str> = dictionary.keys().map(String::as_str).collect();
    assert_eq!(names, ["b", "a", "c"]);
}

#[test]
fn a_dot_file_lays_out_the_way_a_hand_written_plist_would() {
    let source = include_str!("../../../fixtures/valid/dotfile.nuke");
    let document = written(source);
    assert!(
        document.contains("    <key>editor</key>\n    <dict>\n      <key>theme</key>"),
        "{document}"
    );
    assert!(
        document.contains("      <key>tab_width</key>\n      <integer>2</integer>"),
        "{document}"
    );
    reads(source);
}
