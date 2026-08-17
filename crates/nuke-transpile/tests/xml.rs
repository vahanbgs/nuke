use nuke_syntax::parse;
use nuke_transpile::Segment;
use nuke_transpile::xml::{Error, ErrorKind, to_string, to_string_rooted};

fn xml(source: &str) -> String {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect("the value should transpile")
}

fn scalar(source: &str) -> String {
    let written = xml(&format!("{{v = {source}}}"));
    let opened = written.find("<v>").expect("the field should be written") + "<v>".len();
    let closed = written.rfind("</v>").expect("the field should be closed");
    written[opened..closed].to_owned()
}

fn refused(source: &str) -> Error {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect_err("the value should be refused")
}

#[test]
fn scalars_carry_their_xml_spelling_and_no_atom_is_translated() {
    assert_eq!(scalar("True"), "True");
    assert_eq!(scalar("False"), "False");
    assert_eq!(scalar("Null"), "Null");
    assert_eq!(scalar("Relative"), "Relative");
    assert_eq!(scalar("0"), "0");
    assert_eq!(scalar("-0"), "0");
    assert_eq!(scalar("-7"), "-7");
    assert_eq!(scalar(r#""text""#), "text");
    assert_eq!(scalar(r#""a b c""#), "a b c");
}

#[test]
fn an_atom_and_the_string_that_spells_it_write_the_same_document() {
    assert_eq!(xml("[True]"), xml(r#"["True"]"#));
    assert_eq!(xml("[42]"), xml(r#"["42"]"#));
    assert_eq!(xml("[Null]"), xml(r#"["Null"]"#));
}

#[test]
fn an_integer_keeps_every_digit_it_came_with() {
    let wide = "170141183460469231731687303715884105728";
    assert_eq!(scalar(wide), wide);
}

#[test]
fn a_float_keeps_its_shape_and_reads_back_as_the_same_double() {
    assert_eq!(scalar("0.0"), "0.0");
    assert_eq!(scalar("-0.0"), "-0.0");
    assert_eq!(scalar("1e5"), "100000.0");
    assert_eq!(scalar("1e300"), "1e300");
    for source in ["0.0", "-0.0", "1.5", "1e5", "1e-5", "-2.5e-3", "1e300"] {
        let written = scalar(source);
        let read: f64 = written
            .parse()
            .unwrap_or_else(|_| panic!("{written} is a double"));
        let want: f64 = source.parse().expect("the source is a double");
        assert_eq!(
            read.to_bits(),
            want.to_bits(),
            "{source} read back as {written}"
        );
        assert!(
            written.contains('.') || written.contains('e'),
            "{source} lost its float shape as {written}"
        );
    }
}

#[test]
fn text_takes_the_three_escapes_xml_content_needs_and_leaves_the_quotes_alone() {
    assert_eq!(scalar(r#""a & b""#), "a &amp; b");
    assert_eq!(scalar(r#""a < b""#), "a &lt; b");
    assert_eq!(scalar(r#""a > b""#), "a &gt; b");
    assert_eq!(scalar(r#""]]>""#), "]]&gt;");
    assert_eq!(scalar(r#""\"'""#), "\"'");
    assert_eq!(scalar(r#""&amp;""#), "&amp;amp;");
}

#[test]
fn a_carriage_return_is_written_as_a_reference_and_the_other_spaces_stand() {
    assert_eq!(scalar(r#""\r""#), "&#xD;");
    assert_eq!(scalar(r#""\r\n""#), "&#xD;\n");
    assert_eq!(scalar(r#""\n""#), "\n");
    assert_eq!(scalar(r#""\t""#), "\t");
}

#[test]
fn a_character_xml_admits_but_yaml_escapes_is_written_literally() {
    for source in [
        r#""\u{7F}""#,
        r#""\u{9F}""#,
        r#""\u{FEFF}""#,
        r#""\u{FFFD}""#,
        r#""\u{10FFFF}""#,
        r#""\u{1F600}""#,
    ] {
        let Ok(nuke_syntax::Value::String(text)) = parse(source) else {
            panic!("{source} is a string");
        };
        assert_eq!(scalar(source), text.as_str(), "for {source}");
    }
}

#[test]
fn a_character_xml_cannot_carry_is_refused_where_it_stands() {
    for (source, character) in [
        (r#"["\u{0}"]"#, '\u{0}'),
        (r#"["\u{8}"]"#, '\u{8}'),
        (r#"["\u{B}"]"#, '\u{B}'),
        (r#"["\u{C}"]"#, '\u{C}'),
        (r#"["\u{E}"]"#, '\u{E}'),
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
    }
    let error = refused(r#"{shell = {aliases = ["ok" "\u{0}"]}}"#);
    assert_eq!(error.path().to_string(), "shell.aliases[1]");
}

#[test]
fn a_forbidden_character_in_a_map_key_is_refused_at_the_entry_that_holds_it() {
    let error = refused(r#"{"a" => 1 "\u{0}" => 2}"#);
    assert_eq!(error.kind(), &ErrorKind::UnrepresentableCharacter('\u{0}'));
    assert_eq!(error.path().to_string(), "#2");
    assert_eq!(
        error.path().segments().last(),
        Some(&Segment::Entry(1)),
        "the entry, not its key, is what can be named"
    );
}

#[test]
fn a_path_names_the_root_when_nothing_encloses_the_value() {
    let error = refused(r#""\u{0}""#);
    assert!(error.path().is_root());
    assert_eq!(error.path().to_string(), "the document");
}

#[test]
fn a_tuple_is_named_children_and_a_map_is_entries_and_a_list_is_items() {
    assert_eq!(
        xml("{b = 1 a = 2}"),
        "<nuke>\n  <b>1</b>\n  <a>2</a>\n</nuke>"
    );
    assert_eq!(
        xml("{snake_case_1 = 1}"),
        "<nuke>\n  <snake_case_1>1</snake_case_1>\n</nuke>"
    );
    assert_eq!(
        xml(r#"{"k" => 1}"#),
        "<nuke>\n  <_entry>\n    <_key>k</_key>\n    <_value>1</_value>\n  </_entry>\n</nuke>"
    );
    assert_eq!(xml("[1]"), "<nuke>\n  <_item>1</_item>\n</nuke>");
    assert_eq!(
        xml("[[1]]"),
        "<nuke>\n  <_item>\n    <_item>1</_item>\n  </_item>\n</nuke>"
    );
}

#[test]
fn a_key_that_is_a_whole_collection_crosses_where_json_and_toml_refuse_it() {
    for source in [
        r#"{[1 2] => "list key"}"#,
        r#"{{x = 1} => "tuple key"}"#,
        "{[] => []}",
        "{42 => True}",
    ] {
        let value = parse(source).expect("the source should parse");
        to_string(&value).unwrap_or_else(|error| panic!("{source} should transpile: {error}"));
    }
}

#[test]
fn every_shape_the_writer_chooses_is_xml_a_real_parser_reads() {
    for source in [
        r#"{a = 1 b = {c = 2} d = {e = {f = 3}}}"#,
        r#"{k = [{a = 1} {a = 2}] z = {y = 1}}"#,
        r#"{"a.b" => {c = 1}}"#,
        r#"{"a b" => {"c d" => [{e = 1}]}}"#,
        "{a = {} b = [] c = [{}] d = [[1] [2]]}",
        r#"{a = "&<>]]>\r\n\t" b = ["x" Relative Null True]}"#,
        r#"{a = "\u{7F}\u{9F}\u{FEFF}\u{10FFFF}"}"#,
        "{a = {b = {c = {d = [{e = 1}]}}}}",
    ] {
        let written = xml(source);
        let mut reader = quick_xml::Reader::from_str(&written);
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(error) => {
                    panic!("{source} wrote XML that does not parse: {error}\n{written}")
                }
            }
        }
    }
}

#[test]
fn a_tuple_shaped_like_the_backends_own_vocabulary_still_writes_as_a_tuple() {
    assert_ne!(xml("{a = {item = 1}}"), xml("{a = [1]}"));
    assert_ne!(
        xml(r#"{a = {entry = {key = "k" value = 1}}}"#),
        xml(r#"{a = {"k" => 1}}"#)
    );
}

#[test]
fn a_field_name_the_xml_spec_reserves_is_still_written_as_itself() {
    assert_eq!(
        xml("{xml = 1 xmlns = 2 xml_version = 3}"),
        "<nuke>\n  <xml>1</xml>\n  <xmlns>2</xmlns>\n  <xml_version>3</xml_version>\n</nuke>"
    );
}

#[test]
fn an_empty_collection_and_an_empty_string_write_the_same_empty_element() {
    assert_eq!(xml(r#"{a = ""}"#), "<nuke>\n  <a></a>\n</nuke>");
    assert_eq!(xml("{a = []}"), xml(r#"{a = ""}"#));
    assert_eq!(xml("{a = {}}"), xml(r#"{a = ""}"#));
    assert_eq!(xml("[[]]"), xml("[{}]"));
}

#[test]
fn two_keys_no_other_backend_can_tell_apart_are_simply_two_entries_here() {
    for source in [
        r#"{Relative => 1 "Relative" => 2}"#,
        r#"{42 => 1 "42" => 2}"#,
        r#"{[] => 1 "" => 2}"#,
    ] {
        let value = parse(source).expect("the source should parse");
        let written =
            to_string(&value).unwrap_or_else(|error| panic!("{source} should transpile: {error}"));
        assert_eq!(written.matches("<_entry>").count(), 2, "for {source}");
    }
}

#[test]
fn an_element_that_holds_text_holds_it_on_one_line_with_no_indentation_of_its_own() {
    assert_eq!(xml(r#"{a = " "}"#), "<nuke>\n  <a> </a>\n</nuke>");
    assert_eq!(xml(r#"{a = "\n"}"#), "<nuke>\n  <a>\n</a>\n</nuke>");
    assert_eq!(xml(r#"{a = " x "}"#), "<nuke>\n  <a> x </a>\n</nuke>");
}

#[test]
fn the_root_wraps_any_document_and_carries_no_declaration() {
    assert_eq!(xml("42"), "<nuke>42</nuke>");
    assert_eq!(xml(r#""text""#), "<nuke>text</nuke>");
    assert_eq!(xml("True"), "<nuke>True</nuke>");
    assert_eq!(xml("[]"), "<nuke></nuke>");
    assert_eq!(xml("{}"), "<nuke></nuke>");
    assert_eq!(xml("[1]"), "<nuke>\n  <_item>1</_item>\n</nuke>");
}

#[test]
fn a_caller_may_name_the_root_and_a_name_xml_cannot_take_is_refused() {
    let value = parse("{a = 1}").expect("the source should parse");
    for root in ["fontconfig", "_x", "xml_thing", "größe", "a-b.c"] {
        let written = to_string_rooted(&value, root)
            .unwrap_or_else(|error| panic!("{root} should name an element: {error}"));
        assert!(written.starts_with(&format!("<{root}>")), "for {root}");
        assert!(written.ends_with(&format!("</{root}>")), "for {root}");
    }
    for root in ["", "1a", "a b", "a:b", "-x", "."] {
        let error = to_string_rooted(&value, root).expect_err("the name should be refused");
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableName(root.to_owned()),
            "for {root}"
        );
        assert!(error.path().is_root(), "for {root}");
    }
}

#[test]
fn a_dot_file_lays_out_the_way_a_hand_written_xml_file_would() {
    let source = include_str!("../../../fixtures/valid/dotfile.nuke");
    assert_eq!(
        xml(source),
        "\
<nuke>
  <editor>
    <theme>gruvbox-dark</theme>
    <tab_width>2</tab_width>
    <line_numbers>Relative</line_numbers>
  </editor>
  <shell>
    <aliases>
      <_entry>
        <_key>ll</_key>
        <_value>eza -l</_value>
      </_entry>
      <_entry>
        <_key>gs</_key>
        <_value>git status</_value>
      </_entry>
    </aliases>
    <history_size>10000</history_size>
  </shell>
  <keybindings>
    <_item>
      <keys>ctrl+t</keys>
      <action>OpenTerminal</action>
    </_item>
    <_item>
      <keys>ctrl+w</keys>
      <action>CloseWindow</action>
    </_item>
  </keybindings>
</nuke>"
    );
}

#[test]
fn a_key_xml_cannot_name_is_written_as_a_key_element_of_its_own() {
    let source = include_str!("../../../fixtures/valid/maps.nuke");
    assert_eq!(
        xml(source),
        "\
<nuke>
  <_entry>
    <_key>string key</_key>
    <_value>1</_value>
  </_entry>
  <_entry>
    <_key>42</_key>
    <_value>True</_value>
  </_entry>
  <_entry>
    <_key>-1.5</_key>
    <_value>float key</_value>
  </_entry>
  <_entry>
    <_key>True</_key>
    <_value>atom key</_value>
  </_entry>
  <_entry>
    <_key>
      <_item>1</_item>
      <_item>2</_item>
    </_key>
    <_value>list key</_value>
  </_entry>
  <_entry>
    <_key>
      <x>1</x>
    </_key>
    <_value>tuple key</_value>
  </_entry>
  <_entry>
    <_key></_key>
    <_value>empty key</_value>
  </_entry>
  <_entry>
    <_key>
      <_entry>
        <_key>nested</_key>
        <_value>1</_value>
      </_entry>
    </_key>
    <_value>map key</_value>
  </_entry>
</nuke>"
    );
}

#[test]
fn a_value_nested_past_the_parsers_own_limit_is_refused_rather_than_overflowing() {
    let mut value = nuke_syntax::Value::List(Vec::new());
    for _ in 1..nuke_syntax::MAX_DEPTH + 2 {
        value = nuke_syntax::Value::List(vec![value]);
    }
    let error = to_string(&value).expect_err("the writer should refuse it");
    assert_eq!(error.kind(), &ErrorKind::TooDeep);

    let mut key = nuke_syntax::Value::List(Vec::new());
    for _ in 1..nuke_syntax::MAX_DEPTH + 2 {
        key = nuke_syntax::Value::List(vec![key]);
    }
    let mut map = nuke_syntax::Map::new();
    map.insert(
        key,
        nuke_syntax::Value::Integer(nuke_syntax::Integer::parse("1").expect("1 is an integer")),
    );
    let error =
        to_string(&nuke_syntax::Value::Map(map)).expect_err("a key counts toward the limit");
    assert_eq!(error.kind(), &ErrorKind::TooDeep);
}
