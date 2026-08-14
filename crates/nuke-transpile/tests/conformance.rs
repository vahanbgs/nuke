use nuke_fixtures::Fixture;
use nuke_syntax::{Value, parse};
use nuke_transpile::ErrorKind;
use nuke_transpile::json::{to_string, to_string_compact};

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
