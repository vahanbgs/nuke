use kdl::{KdlDocument, KdlNode, KdlValue};
use nuke_fixtures::Fixture;
use nuke_syntax::{Value, parse};
use nuke_transpile::gitconfig;
use nuke_transpile::ini as ini_backend;
use nuke_transpile::json::{ErrorKind, to_string, to_string_compact};
use nuke_transpile::kdl as kdl_backend;
use nuke_transpile::lua;
use nuke_transpile::nix;
use nuke_transpile::toml as toml_backend;
use nuke_transpile::xml;
use nuke_transpile::yaml;
use quick_xml::Reader;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::Event;
use quick_xml::name::QName;
use rnix::ast::{Attr, Entry, Expr, HasEntry, InterpolPart, LiteralKind, Str, UnaryOpKind};
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

fn kdl_refusals() -> Vec<(&'static str, kdl_backend::ErrorKind, &'static str)> {
    vec![
        (
            "collections.nuke",
            kdl_backend::ErrorKind::UnrepresentableKey("list"),
            "[6]#1",
        ),
        (
            "maps.nuke",
            kdl_backend::ErrorKind::UnrepresentableKey("integer"),
            "#2",
        ),
    ]
}

#[test]
fn every_fixture_kdl_cannot_carry_is_refused_by_the_error_that_names_its_fault() {
    for (name, kind, path) in kdl_refusals() {
        let fixture = fixture(name);
        let error = kdl_backend::to_string(&value_of(&fixture))
            .err()
            .unwrap_or_else(|| panic!("{name} should have been refused"));
        assert_eq!(error.kind(), &kind, "for {name}");
        assert_eq!(error.path().to_string(), path, "for {name}");
    }
}

#[test]
fn the_fixtures_kdl_refuses_are_the_ones_json_refuses_for_the_same_reason() {
    let json: Vec<&str> = refusals().into_iter().map(|(name, _, _)| name).collect();
    let kdl: Vec<&str> = kdl_refusals()
        .into_iter()
        .map(|(name, _, _)| name)
        .collect();
    assert_eq!(json, kdl);
}

#[test]
fn what_the_backend_writes_is_the_document_a_real_kdl_parser_loads_back() {
    let refused = kdl_refusals();
    for fixture in nuke_fixtures::valid() {
        if refused.iter().any(|(name, _, _)| *name == fixture.name()) {
            continue;
        }
        let value = value_of(&fixture);
        let written = kdl_backend::to_string(&value).expect("it should transpile");
        let document = KdlDocument::parse_v2(&written).unwrap_or_else(|error| {
            panic!(
                "the KDL of {} does not parse: {error}\n{written}",
                fixture.name()
            )
        });
        carries(&value, document.nodes(), fixture.name());
    }
}

fn carries(value: &Value, nodes: &[KdlNode], at: &str) {
    match value {
        Value::Tuple(tuple) => {
            assert_eq!(nodes.len(), tuple.len(), "{at} lost a field");
            for ((field, item), node) in tuple.iter().zip(nodes) {
                assert_eq!(named(node), field.as_str(), "{at} renamed a field");
                stands(item, node, &format!("{at}.{field}"));
            }
        }
        Value::Map(map) => {
            assert_eq!(nodes.len(), map.len(), "{at} lost an entry");
            for (position, ((key, item), node)) in map.iter().zip(nodes).enumerate() {
                let at = format!("{at}#{}", position + 1);
                let spelling = match key {
                    Value::String(text) => text.as_str(),
                    Value::Atom(atom) => atom.as_str(),
                    other => panic!("{at} has a key KDL cannot name: {other:?}"),
                };
                assert_eq!(named(node), spelling, "{at} renamed a key");
                stands(item, node, &at);
            }
        }
        Value::List(items) => {
            assert_eq!(nodes.len(), items.len(), "{at} lost an element");
            for (index, (item, node)) in items.iter().zip(nodes).enumerate() {
                let at = format!("{at}[{index}]");
                assert_eq!(named(node), "_item", "{at} is not an item");
                stands(item, node, &at);
            }
        }
        other => panic!("{at} is a scalar no KDL document can open with: {other:?}"),
    }
}

fn stands(value: &Value, node: &KdlNode, at: &str) {
    if matches!(value, Value::Tuple(_) | Value::Map(_) | Value::List(_)) {
        carries(value, block(node, at), at);
        return;
    }
    let loaded = argument(node, at);
    match value {
        Value::Atom(atom) => match atom.as_str() {
            "True" => assert_eq!(loaded.as_bool(), Some(true), "at {at}"),
            "False" => assert_eq!(loaded.as_bool(), Some(false), "at {at}"),
            "Null" => assert!(loaded.is_null(), "{at} is {loaded} rather than null"),
            spelling => assert_eq!(loaded.as_string(), Some(spelling), "at {at}"),
        },
        Value::String(text) => assert_eq!(loaded.as_string(), Some(text.as_str()), "at {at}"),
        Value::Integer(integer) => assert_eq!(loaded.as_integer(), integer.to_i128(), "at {at}"),
        Value::Float(number) => {
            let read = loaded
                .as_float()
                .unwrap_or_else(|| panic!("{at} should be a float, not {loaded}"));
            assert_eq!(read.to_bits(), number.get().to_bits(), "at {at}");
        }
        Value::Tuple(_) | Value::Map(_) | Value::List(_) => {}
    }
}

fn block<'a>(node: &'a KdlNode, at: &str) -> &'a [KdlNode] {
    assert!(
        node.entries().is_empty(),
        "{at} carries an argument beside its children"
    );
    node.children().map_or(&[][..], KdlDocument::nodes)
}

fn argument<'a>(node: &'a KdlNode, at: &str) -> &'a KdlValue {
    assert!(
        node.children().is_none(),
        "{at} opens a block around a scalar"
    );
    let [entry] = node.entries() else {
        panic!(
            "{at} should carry one argument, not {}",
            node.entries().len()
        );
    };
    assert!(
        entry.name().is_none(),
        "{at} writes a property rather than an argument"
    );
    entry.value()
}

fn named(node: &KdlNode) -> &str {
    node.name().value()
}

fn lua_refusals() -> Vec<(&'static str, lua::ErrorKind, &'static str)> {
    vec![
        (
            "collections.nuke",
            lua::ErrorKind::UnrepresentableKey("list"),
            "[6]#1",
        ),
        (
            "maps.nuke",
            lua::ErrorKind::UnrepresentableKey("list"),
            "#5",
        ),
    ]
}

#[test]
fn every_fixture_lua_cannot_carry_is_refused_by_the_error_that_names_its_fault() {
    for (name, kind, path) in lua_refusals() {
        let fixture = fixture(name);
        let error = lua::to_string(&value_of(&fixture))
            .err()
            .unwrap_or_else(|| panic!("{name} should have been refused"));
        assert_eq!(error.kind(), &kind, "for {name}");
        assert_eq!(error.path().to_string(), path, "for {name}");
    }
}

#[test]
fn lua_stops_later_in_the_two_fixtures_json_also_refuses() {
    let json: Vec<&str> = refusals().into_iter().map(|(name, _, _)| name).collect();
    let lua: Vec<&str> = lua_refusals()
        .into_iter()
        .map(|(name, _, _)| name)
        .collect();
    assert_eq!(json, lua);
    let json = refusals()
        .into_iter()
        .find(|(name, _, _)| *name == "maps.nuke");
    let lua = lua_refusals()
        .into_iter()
        .find(|(name, _, _)| *name == "maps.nuke");
    assert_eq!(json.map(|(_, _, path)| path), Some("#2"));
    assert_eq!(lua.map(|(_, _, path)| path), Some("#5"));
}

#[test]
fn what_the_backend_writes_is_the_value_a_real_lua_returns() {
    let state = mlua::Lua::new();
    let refused = lua_refusals();
    for fixture in nuke_fixtures::valid() {
        if refused.iter().any(|(name, _, _)| *name == fixture.name()) {
            continue;
        }
        let value = value_of(&fixture);
        let written = lua::to_string(&value).expect("it should transpile");
        let loaded = state
            .load(written.as_str())
            .eval::<mlua::Value>()
            .unwrap_or_else(|error| {
                panic!(
                    "the Lua of {} does not load: {error}\n{written}",
                    fixture.name()
                )
            });
        keeps(&state, &value, &loaded, fixture.name());
    }
}

#[test]
fn the_deepest_document_the_backend_writes_is_one_a_real_lua_still_loads() {
    let mut value = Value::List(Vec::new());
    for _ in 1..nuke_syntax::MAX_DEPTH + 1 {
        value = Value::List(vec![value]);
    }
    let written = lua::to_string(&value).expect("the deepest document should transpile");
    let state = mlua::Lua::new();
    state
        .load(written.as_str())
        .eval::<mlua::Value>()
        .expect("Lua caps its own nesting near 200 levels, and MAX_DEPTH is under it");
}

fn keeps(state: &mlua::Lua, value: &Value, loaded: &mlua::Value, at: &str) {
    match value {
        Value::Tuple(tuple) => {
            let table = fields(loaded, at);
            assert_eq!(count(table), tuple.len(), "{at} lost a field");
            for (name, item) in tuple.iter() {
                let at = format!("{at}.{name}");
                keeps(state, item, &held(table, name.as_str(), &at), &at);
            }
        }
        Value::Map(map) => {
            let table = fields(loaded, at);
            assert_eq!(count(table), map.len(), "{at} lost an entry");
            for (position, (key, item)) in map.iter().enumerate() {
                let at = format!("{at}#{}", position + 1);
                keeps(state, item, &held(table, index(state, key, &at), &at), &at);
            }
        }
        Value::List(items) => {
            let table = fields(loaded, at);
            assert_eq!(count(table), items.len(), "{at} lost an element");
            assert_eq!(table.raw_len(), items.len(), "{at} is not an array");
            for (position, item) in items.iter().enumerate() {
                let at = format!("{at}[{position}]");
                keeps(state, item, &held(table, position + 1, &at), &at);
            }
        }
        other => means(other, loaded, at),
    }
}

fn means(value: &Value, loaded: &mlua::Value, at: &str) {
    match value {
        Value::Atom(atom) => match atom.as_str() {
            "True" => assert_eq!(loaded.as_boolean(), Some(true), "at {at}"),
            "False" => assert_eq!(loaded.as_boolean(), Some(false), "at {at}"),
            spelling => text(loaded, spelling, at),
        },
        Value::String(item) => text(loaded, item, at),
        Value::Integer(integer) => {
            let mlua::Value::Integer(read) = loaded else {
                panic!("{at} should be an integer, not {loaded:?}");
            };
            assert_eq!(Some(*read), integer.to_i64(), "at {at}");
        }
        Value::Float(number) => {
            let mlua::Value::Number(read) = loaded else {
                panic!("{at} should be a float, not {loaded:?}");
            };
            assert_eq!(read.to_bits(), number.get().to_bits(), "at {at}");
        }
        Value::Tuple(_) | Value::Map(_) | Value::List(_) => {}
    }
}

fn text(loaded: &mlua::Value, spelling: &str, at: &str) {
    let mlua::Value::String(read) = loaded else {
        panic!("{at} should be a string, not {loaded:?}");
    };
    assert_eq!(&*read.as_bytes(), spelling.as_bytes(), "at {at}");
}

fn index(state: &mlua::Lua, key: &Value, at: &str) -> mlua::Value {
    match key {
        Value::Atom(atom) => match atom.as_str() {
            "True" => mlua::Value::Boolean(true),
            "False" => mlua::Value::Boolean(false),
            spelling => spelled(state, spelling),
        },
        Value::String(item) => spelled(state, item),
        Value::Integer(integer) => mlua::Value::Integer(
            integer
                .to_i64()
                .unwrap_or_else(|| panic!("{at} keys with an integer Lua refused")),
        ),
        Value::Float(number) => mlua::Value::Number(number.get()),
        other => panic!("{at} has a key Lua holds by identity: {other:?}"),
    }
}

fn spelled(state: &mlua::Lua, text: &str) -> mlua::Value {
    mlua::Value::String(state.create_string(text).expect("a string is created"))
}

fn held(table: &mlua::Table, key: impl mlua::IntoLua, at: &str) -> mlua::Value {
    table
        .get(key)
        .unwrap_or_else(|error| panic!("{at} cannot be read back: {error}"))
}

fn fields<'a>(loaded: &'a mlua::Value, at: &str) -> &'a mlua::Table {
    loaded
        .as_table()
        .unwrap_or_else(|| panic!("{at} should be a table, not {loaded:?}"))
}

fn count(table: &mlua::Table) -> usize {
    let mut pairs = 0;
    for pair in table.clone().pairs::<mlua::Value, mlua::Value>() {
        assert!(pair.is_ok(), "a pair of the table does not read back");
        pairs += 1;
    }
    pairs
}

fn ini_refusals() -> Vec<(&'static str, ini_backend::ErrorKind, &'static str)> {
    let root = ini_backend::ErrorKind::UnrepresentableRoot("list");
    vec![
        ("collections.nuke", root.clone(), "the document"),
        ("comments.nuke", root.clone(), "the document"),
        (
            "dotfile.nuke",
            ini_backend::ErrorKind::UnrepresentableValue("map"),
            "shell.aliases",
        ),
        (
            "maps.nuke",
            ini_backend::ErrorKind::UnrepresentableKey("integer"),
            "#2",
        ),
        ("scalars.nuke", root.clone(), "the document"),
        ("strings.nuke", root.clone(), "the document"),
        (
            "tuples.nuke",
            ini_backend::ErrorKind::UnrepresentableValue("list"),
            "nested.a",
        ),
        ("whitespace.nuke", root, "the document"),
    ]
}

#[test]
fn every_fixture_ini_cannot_carry_is_refused_by_the_error_that_names_its_fault() {
    for (name, kind, path) in ini_refusals() {
        let fixture = fixture(name);
        let error = ini_backend::to_string(&value_of(&fixture))
            .err()
            .unwrap_or_else(|| panic!("{name} should have been refused"));
        assert_eq!(error.kind(), &kind, "for {name}");
        assert_eq!(error.path().to_string(), path, "for {name}");
    }
}

#[test]
fn the_flat_targets_are_the_ones_no_fixture_crosses_into_at_all() {
    let written: Vec<String> = nuke_fixtures::valid()
        .iter()
        .map(|fixture| fixture.name().to_owned())
        .collect();
    for (target, refused) in [
        ("an INI", names(ini_refusals())),
        ("a git config", names(gitconfig_refusals())),
    ] {
        assert_eq!(
            refused, written,
            "some document written in this language is {target} file"
        );
    }

    for (target, refusals) in [
        ("JSON", refusals().len()),
        ("TOML", toml_refusals().len()),
        ("XML", xml_refusals().len()),
        ("KDL", kdl_refusals().len()),
        ("Lua", lua_refusals().len()),
        ("Nix", nix_refusals().len()),
    ] {
        assert!(
            refusals < written.len(),
            "{target} refuses every fixture too, so the flat targets are no longer the ones that do"
        );
    }
}

#[test]
fn the_key_rule_ini_inherits_stops_it_where_json_stops() {
    let json = refusals()
        .into_iter()
        .find(|(name, _, _)| *name == "maps.nuke")
        .map(|(_, _, path)| path);
    let ini = ini_refusals()
        .into_iter()
        .find(|(name, _, _)| *name == "maps.nuke")
        .map(|(_, _, path)| path);
    assert_eq!(json, Some("#2"));
    assert_eq!(
        ini, json,
        "a name is a string or an atom here for the reason it is there"
    );
}

fn gitconfig_refusals() -> Vec<(&'static str, gitconfig::ErrorKind, &'static str)> {
    let root = gitconfig::ErrorKind::UnrepresentableRoot("list");
    vec![
        ("collections.nuke", root.clone(), "the document"),
        ("comments.nuke", root.clone(), "the document"),
        (
            "dotfile.nuke",
            gitconfig::ErrorKind::UnspellableName("tab_width".to_owned()),
            "editor.tab_width",
        ),
        (
            "maps.nuke",
            gitconfig::ErrorKind::SectionlessKey("string key".to_owned()),
            "#1",
        ),
        ("scalars.nuke", root.clone(), "the document"),
        ("strings.nuke", root.clone(), "the document"),
        (
            "tuples.nuke",
            gitconfig::ErrorKind::SectionlessKey("name".to_owned()),
            "name",
        ),
        ("whitespace.nuke", root, "the document"),
    ]
}

#[test]
fn every_fixture_gitconfig_cannot_carry_is_refused_by_the_error_that_names_its_fault() {
    for (name, kind, path) in gitconfig_refusals() {
        let fixture = fixture(name);
        let error = gitconfig::to_string(&value_of(&fixture))
            .err()
            .unwrap_or_else(|| panic!("{name} should have been refused"));
        assert_eq!(error.kind(), &kind, "for {name}");
        assert_eq!(error.path().to_string(), path, "for {name}");
    }
}

#[test]
fn gitconfig_stops_the_dot_file_at_a_field_name_and_ini_stops_it_later_at_a_value() {
    let ini = stop_of(ini_refusals(), "dotfile.nuke");
    let git = stop_of(gitconfig_refusals(), "dotfile.nuke");
    assert_eq!(ini, "shell.aliases");
    assert_eq!(git, "editor.tab_width");
    assert!(
        !git.contains('#'),
        "a field name is what gitconfig cannot spell, and only a map's key carries a #"
    );
}

#[test]
fn gitconfig_stops_the_map_fixture_one_entry_before_json_and_ini_do() {
    assert_eq!(stop_of(refusals(), "maps.nuke"), "#2");
    assert_eq!(stop_of(ini_refusals(), "maps.nuke"), "#2");
    assert_eq!(
        stop_of(gitconfig_refusals(), "maps.nuke"),
        "#1",
        "a variable outside a section is a fault the first entry already has"
    );
}

fn nix_refusals() -> Vec<(&'static str, nix::ErrorKind, &'static str)> {
    vec![
        (
            "collections.nuke",
            nix::ErrorKind::UnrepresentableKey("list"),
            "[6]#1",
        ),
        (
            "maps.nuke",
            nix::ErrorKind::UnrepresentableKey("integer"),
            "#2",
        ),
        (
            "strings.nuke",
            nix::ErrorKind::UnrepresentableCharacter('\u{0}'),
            "[5]",
        ),
    ]
}

#[test]
fn every_fixture_nix_cannot_carry_is_refused_by_the_error_that_names_its_fault() {
    for (name, kind, path) in nix_refusals() {
        let fixture = fixture(name);
        let error = nix::to_string(&value_of(&fixture))
            .err()
            .unwrap_or_else(|| panic!("{name} should have been refused"));
        assert_eq!(error.kind(), &kind, "for {name}");
        assert_eq!(error.path().to_string(), path, "for {name}");
    }
}

#[test]
fn the_fixtures_nix_refuses_are_the_ones_json_and_xml_refuse_together() {
    let mut both = names(refusals());
    both.extend(names(xml_refusals()));
    both.sort();
    assert_eq!(names(nix_refusals()), both);
    for fixture in ["collections.nuke", "maps.nuke"] {
        assert_eq!(
            stop_of(nix_refusals(), fixture),
            stop_of(refusals(), fixture),
            "the key rule Nix inherits from JSON stops it elsewhere in {fixture}"
        );
    }
    assert_eq!(
        stop_of(nix_refusals(), "strings.nuke"),
        stop_of(xml_refusals(), "strings.nuke"),
        "the character neither target can carry is not the same one"
    );
}

#[test]
fn what_the_backend_writes_is_the_document_a_real_nix_parser_reads_back() {
    let refused = nix_refusals();
    for fixture in nuke_fixtures::valid() {
        if refused.iter().any(|(name, _, _)| *name == fixture.name()) {
            continue;
        }
        let value = value_of(&fixture);
        let written = nix::to_string(&value).expect("it should transpile");
        let parsed = rnix::Root::parse(&written);
        assert!(
            parsed.errors().is_empty(),
            "the Nix of {} does not parse: {:?}\n{written}",
            fixture.name(),
            parsed.errors()
        );
        let expression = parsed.tree().expr().expect("a document is one expression");
        survives(&value, &expression, fixture.name());
    }
}

fn survives(value: &Value, expression: &Expr, at: &str) {
    match value {
        Value::Tuple(tuple) => {
            let written = attributes(expression, at);
            assert_eq!(written.len(), tuple.len(), "{at} lost a field");
            for ((field, item), (name, held)) in tuple.iter().zip(&written) {
                assert_eq!(name, field.as_str(), "{at} renamed a field");
                survives(item, held, &format!("{at}.{field}"));
            }
        }
        Value::Map(map) => {
            let written = attributes(expression, at);
            assert_eq!(written.len(), map.len(), "{at} lost an entry");
            for (position, ((key, item), (name, held))) in map.iter().zip(&written).enumerate() {
                let at = format!("{at}#{}", position + 1);
                let spelling = match key {
                    Value::String(text) => text.as_str(),
                    Value::Atom(atom) => atom.as_str(),
                    other => panic!("{at} has a key Nix cannot name: {other:?}"),
                };
                assert_eq!(name, spelling, "{at} renamed a key");
                survives(item, held, &at);
            }
        }
        Value::List(items) => {
            let written = elements(expression, at);
            assert_eq!(written.len(), items.len(), "{at} lost an element");
            for (index, (item, held)) in items.iter().zip(&written).enumerate() {
                survives(item, held, &format!("{at}[{index}]"));
            }
        }
        Value::Atom(atom) => match atom.as_str() {
            "True" => assert_eq!(word(expression, at), "true", "at {at}"),
            "False" => assert_eq!(word(expression, at), "false", "at {at}"),
            "Null" => assert_eq!(word(expression, at), "null", "at {at}"),
            spelling => assert_eq!(quoted(expression, at), spelling, "at {at}"),
        },
        Value::String(text) => assert_eq!(quoted(expression, at), text.as_str(), "at {at}"),
        Value::Integer(integer) => assert_eq!(whole(expression, at), integer.to_i64(), "at {at}"),
        Value::Float(number) => assert_eq!(
            fraction(expression, at).to_bits(),
            number.get().to_bits(),
            "at {at}"
        ),
    }
}

fn attributes(expression: &Expr, at: &str) -> Vec<(String, Expr)> {
    let Expr::AttrSet(set) = bare(expression) else {
        panic!("{at} is not an attribute set: {expression}");
    };
    set.entries()
        .map(|entry| {
            let Entry::AttrpathValue(pair) = entry else {
                panic!("{at} inherits a name rather than binding one");
            };
            let path = pair.attrpath().expect("a binding carries a path");
            let attrs: Vec<Attr> = path.attrs().collect();
            let [attr] = attrs.as_slice() else {
                panic!("{at} writes a nested attribute path");
            };
            let name = match attr {
                Attr::Ident(ident) => ident.to_string(),
                Attr::Str(text) => spelling(text, at),
                Attr::Dynamic(_) => panic!("{at} names an attribute with an expression"),
            };
            (name, pair.value().expect("a binding carries a value"))
        })
        .collect()
}

fn elements(expression: &Expr, at: &str) -> Vec<Expr> {
    let Expr::List(list) = bare(expression) else {
        panic!("{at} is not a list: {expression}");
    };
    list.items().collect()
}

fn word(expression: &Expr, at: &str) -> String {
    let Expr::Ident(ident) = bare(expression) else {
        panic!("{at} is not a word Nix knows: {expression}");
    };
    ident.to_string()
}

fn quoted(expression: &Expr, at: &str) -> String {
    let Expr::Str(text) = bare(expression) else {
        panic!("{at} is not a string: {expression}");
    };
    spelling(&text, at)
}

fn spelling(text: &Str, at: &str) -> String {
    match text.normalized_parts().as_slice() {
        [] => String::new(),
        [InterpolPart::Literal(literal)] => literal.clone(),
        _ => panic!("{at} writes a string an antiquotation opens into"),
    }
}

fn whole(expression: &Expr, at: &str) -> Option<i64> {
    match negated(expression) {
        (true, Expr::Literal(literal)) => match literal.kind() {
            LiteralKind::Integer(token) => token.value().ok().map(|value| -value),
            _ => panic!("{at} negates something that is not an integer"),
        },
        (false, Expr::Literal(literal)) => match literal.kind() {
            LiteralKind::Integer(token) => token.value().ok(),
            _ => panic!("{at} is not an integer"),
        },
        _ => panic!("{at} is not an integer: {expression}"),
    }
}

fn fraction(expression: &Expr, at: &str) -> f64 {
    let (negative, literal) = match negated(expression) {
        (negative, Expr::Literal(literal)) => (negative, literal),
        _ => panic!("{at} is not a float: {expression}"),
    };
    let LiteralKind::Float(token) = literal.kind() else {
        panic!("{at} is not a float: {expression}");
    };
    let read = token.value().expect("a float token reads back as a double");
    if negative { -read } else { read }
}

fn negated(expression: &Expr) -> (bool, Expr) {
    match bare(expression) {
        Expr::UnaryOp(operation) => {
            assert_eq!(
                operation.operator(),
                Some(UnaryOpKind::Negate),
                "only a negation stands before a number"
            );
            let (negative, inner) = negated(&operation.expr().expect("a negation holds a value"));
            (!negative, inner)
        }
        other => (false, other),
    }
}

fn bare(expression: &Expr) -> Expr {
    match expression {
        Expr::Paren(paren) => bare(&paren.expr().expect("a parenthesis holds an expression")),
        other => other.clone(),
    }
}

fn names<K>(refusals: Vec<(&'static str, K, &'static str)>) -> Vec<String> {
    refusals
        .into_iter()
        .map(|(name, _, _)| name.to_owned())
        .collect()
}

fn stop_of<K>(refusals: Vec<(&'static str, K, &'static str)>, fixture: &str) -> &'static str {
    refusals
        .into_iter()
        .find(|(name, _, _)| *name == fixture)
        .map(|(_, _, path)| path)
        .unwrap_or_else(|| panic!("{fixture} is no longer refused"))
}
