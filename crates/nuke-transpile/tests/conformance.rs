use nuke_fixtures::Fixture;
use nuke_syntax::{Value, parse};
use nuke_transpile::json::{ErrorKind, to_string, to_string_compact};
use nuke_transpile::yaml;
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
