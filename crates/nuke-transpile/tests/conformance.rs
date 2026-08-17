use nuke_fixtures::Fixture;
use nuke_syntax::{Value, parse};
use nuke_transpile::json::{ErrorKind, to_string, to_string_compact};
use nuke_transpile::toml as toml_backend;
use nuke_transpile::xml;
use nuke_transpile::yaml;
use quick_xml::Reader;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::Event;
use quick_xml::name::QName;
use saphyr::{LoadableYamlNode, Yaml};

fn refusals() -> Vec<(&'static str, ErrorKind, &'static str)> {
    vec![
        (
            "collections.nuke",
            ErrorKind::UnrepresentableKey("list"),
            "[6]#1",
        ),
        ("maps.nuke", ErrorKind::UnrepresentableKey("integer"), "#2"),
    ]
}

fn fixture(name: &str) -> Fixture {
    nuke_fixtures::valid()
        .into_iter()
        .find(|fixture| fixture.name() == name)
        .unwrap_or_else(|| panic!("{name} is no longer a fixture"))
}

fn value_of(fixture: &Fixture) -> Value {
    parse(&fixture.source)
        .unwrap_or_else(|error| panic!("{} should parse: {error}", fixture.display()))
}

fn transpiled() -> Vec<(String, String, String)> {
    let refused = refusals();
    nuke_fixtures::valid()
        .into_iter()
        .filter(|fixture| !refused.iter().any(|(name, _, _)| *name == fixture.name()))
        .map(|fixture| {
            let value = value_of(&fixture);
            let pretty = to_string(&value)
                .unwrap_or_else(|error| panic!("{} should transpile: {error}", fixture.display()));
            let compact = to_string_compact(&value)
                .unwrap_or_else(|error| panic!("{} should transpile: {error}", fixture.display()));
            (fixture.name().to_owned(), pretty, compact)
        })
        .collect()
}

#[test]
fn every_fixture_json_cannot_carry_is_refused_by_the_error_that_names_its_fault() {
    for (name, kind, path) in refusals() {
        let fixture = fixture(name);
        let error = to_string(&value_of(&fixture))
            .err()
            .unwrap_or_else(|| panic!("{name} should have been refused"));
        assert_eq!(error.kind(), &kind, "for {name}");
        assert_eq!(error.path().to_string(), path, "for {name}");
    }
}

#[test]
fn what_the_backend_writes_is_json_a_real_parser_accepts() {
    for (name, pretty, compact) in transpiled() {
        let laid_out: serde_json::Value = serde_json::from_str(&pretty)
            .unwrap_or_else(|error| panic!("the JSON of {name} does not parse: {error}\n{pretty}"));
        let packed: serde_json::Value = serde_json::from_str(&compact).unwrap_or_else(|error| {
            panic!("the JSON of {name} does not parse: {error}\n{compact}")
        });
        assert_eq!(laid_out, packed, "the two layouts of {name} disagree");
    }
}

#[test]
fn the_two_layouts_differ_only_in_the_whitespace_between_tokens() {
    for (name, pretty, compact) in transpiled() {
        assert!(
            !compact.contains('\n'),
            "the compact layout of {name} has a line break"
        );
        assert_eq!(
            pretty.trim_end(),
            pretty,
            "the pretty layout of {name} ends with whitespace"
        );
    }
}

#[test]
fn a_dot_file_reads_the_way_a_program_consuming_json_would_expect() {
    let fixture = fixture("dotfile.nuke");
    let json: serde_json::Value =
        serde_json::from_str(&to_string(&value_of(&fixture)).expect("it should transpile"))
            .expect("its JSON should parse");

    assert_eq!(json["editor"]["theme"], "gruvbox-dark");
    assert_eq!(json["editor"]["tab_width"], 2);
    assert_eq!(json["editor"]["line_numbers"], "Relative");
    assert_eq!(json["shell"]["aliases"]["ll"], "eza -l");
    assert_eq!(json["keybindings"][0]["action"], "OpenTerminal");
}

#[test]
fn the_escapes_survive_a_round_trip_through_a_json_parser() {
    let fixture = fixture("strings.nuke");
    let value = value_of(&fixture);
    let json: serde_json::Value =
        serde_json::from_str(&to_string_compact(&value).expect("it should transpile"))
            .expect("its JSON should parse");
    let Value::List(items) = &value else {
        panic!("the strings fixture is a list");
    };

    let strings = json.as_array().expect("its JSON is an array");
    assert_eq!(strings.len(), items.len());
    for (item, string) in items.iter().zip(strings) {
        let Value::String(text) = item else {
            panic!("every element of the strings fixture is a string");
        };
        assert_eq!(string.as_str(), Some(text.as_str()));
    }
}

#[test]
fn there_is_no_fixture_yaml_has_to_refuse() {
    for fixture in nuke_fixtures::valid() {
        yaml::to_string(&value_of(&fixture))
            .unwrap_or_else(|error| panic!("{} should transpile: {error}", fixture.display()));
    }
}

#[test]
fn what_the_backend_writes_is_the_document_a_real_yaml_parser_loads_back() {
    for fixture in nuke_fixtures::valid() {
        let value = value_of(&fixture);
        let written = yaml::to_string(&value).expect("it should transpile");
        let loaded = Yaml::load_from_str(&written).unwrap_or_else(|error| {
            panic!(
                "the YAML of {} does not parse: {error}\n{written}",
                fixture.name()
            )
        });
        assert_eq!(
            loaded.len(),
            1,
            "{} wrote more than one document",
            fixture.name()
        );
        agrees(&value, &loaded[0], fixture.name());
    }
}

fn agrees(value: &Value, loaded: &Yaml, at: &str) {
    match value {
        Value::Tuple(tuple) => {
            let mapping = mapping(loaded, at);
            assert_eq!(mapping.len(), tuple.len(), "{at} lost an entry");
            for ((name, item), (key, item_yaml)) in tuple.iter().zip(mapping.iter()) {
                assert_eq!(key.as_str(), Some(name.as_str()), "{at} renamed a field");
                agrees(item, item_yaml, &format!("{at}.{name}"));
            }
        }
        Value::Map(map) => {
            let mapping = mapping(loaded, at);
            assert_eq!(mapping.len(), map.len(), "{at} lost an entry");
            for (position, ((key, item), (key_yaml, item_yaml))) in
                map.iter().zip(mapping.iter()).enumerate()
            {
                let at = format!("{at}#{}", position + 1);
                agrees(key, key_yaml, &at);
                agrees(item, item_yaml, &at);
            }
        }
        Value::List(items) => {
            let sequence = loaded
                .as_sequence()
                .unwrap_or_else(|| panic!("{at} should be a sequence, not {loaded:?}"));
            assert_eq!(sequence.len(), items.len(), "{at} lost an element");
            for (index, (item, item_yaml)) in items.iter().zip(sequence).enumerate() {
                agrees(item, item_yaml, &format!("{at}[{index}]"));
            }
        }
        Value::Atom(atom) => match atom.as_str() {
            "True" => assert_eq!(loaded.as_bool(), Some(true), "at {at}"),
            "False" => assert_eq!(loaded.as_bool(), Some(false), "at {at}"),
            "Null" => assert!(loaded.is_null(), "{at} is {loaded:?} rather than null"),
            spelling => assert_eq!(loaded.as_str(), Some(spelling), "at {at}"),
        },
        Value::String(text) => assert_eq!(loaded.as_str(), Some(text.as_str()), "at {at}"),
        Value::Integer(integer) => assert_eq!(loaded.as_integer(), integer.to_i64(), "at {at}"),
        Value::Float(number) => {
            assert_eq!(loaded.as_floating_point(), Some(number.get()), "at {at}");
        }
    }
}

fn mapping<'a>(loaded: &'a Yaml<'a>, at: &str) -> &'a saphyr::Mapping<'a> {
    loaded
        .as_mapping()
        .unwrap_or_else(|| panic!("{at} should be a mapping, not {loaded:?}"))
}

fn toml_refusals() -> Vec<(&'static str, toml_backend::ErrorKind, &'static str)> {
    let root = toml_backend::ErrorKind::UnrepresentableRoot("list");
    vec![
        ("collections.nuke", root.clone(), "the document"),
        ("comments.nuke", root.clone(), "the document"),
        ("scalars.nuke", root.clone(), "the document"),
        ("strings.nuke", root.clone(), "the document"),
        ("whitespace.nuke", root, "the document"),
        (
            "maps.nuke",
            toml_backend::ErrorKind::UnrepresentableKey("integer"),
            "#2",
        ),
    ]
}

#[test]
fn every_fixture_toml_cannot_carry_is_refused_by_the_error_that_names_its_fault() {
    for (name, kind, path) in toml_refusals() {
        let fixture = fixture(name);
        let error = toml_backend::to_string(&value_of(&fixture))
            .err()
            .unwrap_or_else(|| panic!("{name} should have been refused"));
        assert_eq!(error.kind(), &kind, "for {name}");
        assert_eq!(error.path().to_string(), path, "for {name}");
    }
}

#[test]
fn what_the_backend_writes_is_the_document_a_real_toml_parser_loads_back() {
    let refused = toml_refusals();
    for fixture in nuke_fixtures::valid() {
        if refused.iter().any(|(name, _, _)| *name == fixture.name()) {
            continue;
        }
        let value = value_of(&fixture);
        let written = toml_backend::to_string(&value).expect("it should transpile");
        let loaded: toml::Value = toml::from_str(&written).unwrap_or_else(|error| {
            panic!(
                "the TOML of {} does not parse: {error}\n{written}",
                fixture.name()
            )
        });
        holds(&value, &loaded, fixture.name());
    }
}

fn holds(value: &Value, loaded: &toml::Value, at: &str) {
    match value {
        Value::Tuple(tuple) => {
            let table = table(loaded, at);
            assert_eq!(table.len(), tuple.len(), "{at} lost an entry");
            for ((name, item), (key, item_toml)) in tuple.iter().zip(table) {
                assert_eq!(key.as_str(), name.as_str(), "{at} renamed a field");
                holds(item, item_toml, &format!("{at}.{name}"));
            }
        }
        Value::Map(map) => {
            let table = table(loaded, at);
            assert_eq!(table.len(), map.len(), "{at} lost an entry");
            for (position, ((key, item), (name, item_toml))) in map.iter().zip(table).enumerate() {
                let at = format!("{at}#{}", position + 1);
                let spelling = match key {
                    Value::String(text) => text.as_str(),
                    Value::Atom(atom) => atom.as_str(),
                    other => panic!("{at} has a key TOML cannot name: {other:?}"),
                };
                assert_eq!(name.as_str(), spelling, "{at} renamed a key");
                holds(item, item_toml, &at);
            }
        }
        Value::List(items) => {
            let array = loaded
                .as_array()
                .unwrap_or_else(|| panic!("{at} should be an array, not {loaded:?}"));
            assert_eq!(array.len(), items.len(), "{at} lost an element");
            for (index, (item, item_toml)) in items.iter().zip(array).enumerate() {
                holds(item, item_toml, &format!("{at}[{index}]"));
            }
        }
        Value::Atom(atom) => match atom.as_str() {
            "True" => assert_eq!(loaded.as_bool(), Some(true), "at {at}"),
            "False" => assert_eq!(loaded.as_bool(), Some(false), "at {at}"),
            spelling => assert_eq!(loaded.as_str(), Some(spelling), "at {at}"),
        },
        Value::String(text) => assert_eq!(loaded.as_str(), Some(text.as_str()), "at {at}"),
        Value::Integer(integer) => assert_eq!(loaded.as_integer(), integer.to_i64(), "at {at}"),
        Value::Float(number) => assert_eq!(loaded.as_float(), Some(number.get()), "at {at}"),
    }
}

fn table<'a>(loaded: &'a toml::Value, at: &str) -> &'a toml::Table {
    loaded
        .as_table()
        .unwrap_or_else(|| panic!("{at} should be a table, not {loaded:?}"))
}

fn xml_refusals() -> Vec<(&'static str, xml::ErrorKind, &'static str)> {
    vec![(
        "strings.nuke",
        xml::ErrorKind::UnrepresentableCharacter('\u{0}'),
        "[5]",
    )]
}

#[test]
fn every_fixture_xml_cannot_carry_is_refused_by_the_error_that_names_its_fault() {
    for (name, kind, path) in xml_refusals() {
        let fixture = fixture(name);
        let error = xml::to_string(&value_of(&fixture))
            .err()
            .unwrap_or_else(|| panic!("{name} should have been refused"));
        assert_eq!(error.kind(), &kind, "for {name}");
        assert_eq!(error.path().to_string(), path, "for {name}");
    }
}

#[test]
fn every_fixture_but_the_one_holding_a_character_xml_forbids_crosses_whole() {
    let refused = xml_refusals();
    for fixture in nuke_fixtures::valid() {
        if refused.iter().any(|(name, _, _)| *name == fixture.name()) {
            continue;
        }
        xml::to_string(&value_of(&fixture))
            .unwrap_or_else(|error| panic!("{} should transpile: {error}", fixture.display()));
    }
}

#[test]
fn what_the_backend_writes_is_the_document_a_real_xml_parser_reads_back() {
    for (name, value, written) in written_as_xml() {
        let mut reader = Reader::from_str(&written);
        open(&mut reader, "nuke", &name);
        reads(&value, &mut reader, "nuke", &name);
        assert!(
            matches!(reader.read_event(), Ok(Event::Eof)),
            "{name} writes something beside its root\n{written}"
        );
    }
}

#[test]
fn every_element_the_backend_writes_holds_text_or_children_and_never_both() {
    for (name, _, written) in written_as_xml() {
        let mut reader = Reader::from_str(&written);
        let mut stack: Vec<(bool, bool)> = Vec::new();
        loop {
            match reader.read_event() {
                Ok(Event::Start(_)) => {
                    if let Some((_, children)) = stack.last_mut() {
                        *children = true;
                    }
                    stack.push((false, false));
                }
                Ok(Event::End(_)) => {
                    let (text, children) = stack.pop().expect("an end tag closes an open element");
                    assert!(
                        !(text && children),
                        "{name} mixes text and elements\n{written}"
                    );
                }
                Ok(Event::Text(chunk)) => {
                    let content = chunk.xml10_content().expect("the text decodes");
                    if !content.trim().is_empty()
                        && let Some((text, _)) = stack.last_mut()
                    {
                        *text = true;
                    }
                }
                Ok(Event::GeneralRef(_)) => {
                    if let Some((text, _)) = stack.last_mut() {
                        *text = true;
                    }
                }
                Ok(Event::Eof) => break,
                Ok(other) => panic!("{name} writes {other:?} beside elements and text"),
                Err(error) => panic!("the XML of {name} does not parse: {error}\n{written}"),
            }
        }
    }
}

#[test]
fn the_escapes_survive_a_round_trip_through_an_xml_parser() {
    let Value::List(items) = value_of(&fixture("strings.nuke")) else {
        panic!("the strings fixture is a list");
    };
    let carried: Vec<Value> = items
        .into_iter()
        .filter(|item| xml::to_string(item).is_ok())
        .collect();
    assert_eq!(
        carried.len(),
        10,
        "two of the twelve hold a character XML cannot carry"
    );

    let value = Value::List(carried);
    let written = xml::to_string(&value).expect("the rest should transpile");
    let mut reader = Reader::from_str(&written);
    open(&mut reader, "nuke", "strings.nuke");
    reads(&value, &mut reader, "nuke", "strings.nuke");
}

fn written_as_xml() -> Vec<(String, Value, String)> {
    let refused = xml_refusals();
    nuke_fixtures::valid()
        .into_iter()
        .filter(|fixture| !refused.iter().any(|(name, _, _)| *name == fixture.name()))
        .map(|fixture| {
            let value = value_of(&fixture);
            let written = xml::to_string(&value)
                .unwrap_or_else(|error| panic!("{} should transpile: {error}", fixture.display()));
            (fixture.name().to_owned(), value, written)
        })
        .collect()
}

fn reads(value: &Value, xml: &mut Reader<&[u8]>, name: &str, at: &str) {
    match value {
        Value::Tuple(tuple) if !tuple.is_empty() => {
            for (field, item) in tuple.iter() {
                let at = format!("{at}.{field}");
                open(xml, field.as_str(), &at);
                reads(item, xml, field.as_str(), &at);
            }
            close(xml, name, at);
        }
        Value::Map(map) if !map.is_empty() => {
            for (position, (key, item)) in map.iter().enumerate() {
                let at = format!("{at}#{}", position + 1);
                open(xml, "_entry", &at);
                open(xml, "_key", &at);
                reads(key, xml, "_key", &at);
                open(xml, "_value", &at);
                reads(item, xml, "_value", &at);
                close(xml, "_entry", &at);
            }
            close(xml, name, at);
        }
        Value::List(items) if !items.is_empty() => {
            for (index, item) in items.iter().enumerate() {
                let at = format!("{at}[{index}]");
                open(xml, "_item", &at);
                reads(item, xml, "_item", &at);
            }
            close(xml, name, at);
        }
        Value::Atom(atom) => assert_eq!(content(xml, name, at), atom.as_str(), "at {at}"),
        Value::String(text) => assert_eq!(content(xml, name, at), text.as_str(), "at {at}"),
        Value::Integer(integer) => {
            assert_eq!(content(xml, name, at), integer.as_str(), "at {at}");
        }
        Value::Float(number) => {
            let text = content(xml, name, at);
            let read: f64 = text
                .parse()
                .unwrap_or_else(|_| panic!("{at} should be a double, not {text}"));
            assert_eq!(read.to_bits(), number.get().to_bits(), "at {at}");
        }
        Value::Tuple(_) | Value::Map(_) | Value::List(_) => {
            assert_eq!(content(xml, name, at), "", "{at} is empty");
        }
    }
}

fn open(xml: &mut Reader<&[u8]>, expected: &str, at: &str) {
    match between(xml, at) {
        Event::Start(start) => {
            assert_eq!(tag(start.name()), expected, "{at} opens the wrong element");
        }
        other => panic!("{at} should open `{expected}`, not {other:?}"),
    }
}

fn close(xml: &mut Reader<&[u8]>, expected: &str, at: &str) {
    match between(xml, at) {
        Event::End(end) => {
            assert_eq!(tag(end.name()), expected, "{at} closes the wrong element");
        }
        other => panic!("{at} should close `{expected}`, not {other:?}"),
    }
}

fn content(xml: &mut Reader<&[u8]>, name: &str, at: &str) -> String {
    let mut text = String::new();
    loop {
        match read(xml, at) {
            Event::Text(chunk) => {
                text.push_str(&chunk.xml10_content().expect("the text decodes"));
            }
            Event::GeneralRef(reference) => {
                let resolved = reference.resolve_char_ref().expect("a reference resolves");
                match resolved {
                    Some(character) => text.push(character),
                    None => {
                        let entity = reference.decode().expect("a reference decodes");
                        text.push_str(resolve_predefined_entity(&entity).unwrap_or_else(|| {
                            panic!("{at} writes the entity `{entity}`, which XML does not define")
                        }));
                    }
                }
            }
            Event::End(end) => {
                assert_eq!(tag(end.name()), name, "{at} closes the wrong element");
                return text;
            }
            other => panic!("{at} should hold text, not {other:?}"),
        }
    }
}

fn between<'a>(xml: &mut Reader<&'a [u8]>, at: &str) -> Event<'a> {
    loop {
        match read(xml, at) {
            Event::Text(chunk) => {
                let content = chunk.xml10_content().expect("the text decodes");
                assert!(
                    content.trim().is_empty(),
                    "{at} holds the text {content:?} beside its children"
                );
            }
            other => return other,
        }
    }
}

fn read<'a>(xml: &mut Reader<&'a [u8]>, at: &str) -> Event<'a> {
    xml.read_event()
        .unwrap_or_else(|error| panic!("the XML of {at} does not parse: {error}"))
}

fn tag<'a>(name: QName<'a>) -> &'a str {
    std::str::from_utf8(name.into_inner()).expect("a name the backend writes is UTF-8")
}
