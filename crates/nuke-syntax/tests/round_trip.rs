use std::collections::HashMap;

use nuke_syntax::{Atom, Integer, Tuple, Value, from_str, from_value, parse, to_value};
use serde::Deserialize;

fn parsed(source: &str) -> Value {
    match parse(source) {
        Ok(value) => value,
        Err(error) => panic!("{source} should parse, but: {error}"),
    }
}

fn read(source: &str) -> Value {
    match from_str::<Value>(source) {
        Ok(value) => value,
        Err(error) => panic!("{source} should read as a value, but: {error}"),
    }
}

fn written(value: &Value) -> Value {
    match to_value(value) {
        Ok(value) => value,
        Err(error) => panic!("{value:?} should be written, but: {error}"),
    }
}

#[test]
fn every_valid_fixture_parses_to_the_value_serde_reads_from_it() {
    for fixture in nuke_fixtures::valid() {
        let parsed = match parse(&fixture.source) {
            Ok(value) => value,
            Err(error) => panic!("{} should parse, but: {error}", fixture.display()),
        };
        match from_str::<Value>(&fixture.source) {
            Ok(read) => assert_eq!(
                read,
                parsed,
                "serde and the parser disagree about {}",
                fixture.display()
            ),
            Err(error) => panic!("{} should read as a value, but: {error}", fixture.display()),
        }
    }
}

#[test]
fn every_valid_fixture_survives_being_written_back() {
    for fixture in nuke_fixtures::valid() {
        let parsed = match parse(&fixture.source) {
            Ok(value) => value,
            Err(error) => panic!("{} should parse, but: {error}", fixture.display()),
        };
        assert_eq!(
            written(&parsed),
            parsed,
            "{} does not survive being written",
            fixture.display()
        );
    }
}

#[test]
fn an_atom_stays_an_atom_and_a_tuple_stays_a_tuple() {
    let value = parsed(r#"[Relative "Relative" {a = 1} {"a" => 1}]"#);
    assert_eq!(read(r#"[Relative "Relative" {a = 1} {"a" => 1}]"#), value);
    assert_eq!(written(&value), value);

    let Value::List(items) = &value else {
        panic!("the fixture is a list");
    };
    assert_ne!(items[0], items[1]);
    assert_ne!(items[2], items[3]);
}

#[test]
fn the_empty_block_stays_the_one_the_parser_records() {
    let empty = parsed("{}");
    assert_eq!(empty, Value::Tuple(Tuple::new()));
    assert_eq!(read("{}"), empty);
    assert_eq!(written(&empty), empty);
}

#[test]
fn an_integer_wider_than_any_rust_integer_survives_the_round_trip() {
    let wide = "123456789012345678901234567890123456789012345678901234567890";
    let value = parsed(wide);
    assert_eq!(read(wide), value);
    assert_eq!(written(&value), value);
    assert_eq!(from_value::<Value>(&value), Ok(value));
}

#[test]
fn a_value_nested_in_a_users_type_keeps_what_makes_it_nuke() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Program {
        name: String,
        settings: Value,
    }

    let program: Program =
        match from_str(r#"{name = "nuke" settings = {theme = Dark wrap = True}}"#) {
            Ok(program) => program,
            Err(error) => panic!("this should read, but: {error}"),
        };
    assert_eq!(program.name, "nuke");
    assert_eq!(program.settings, parsed("{theme = Dark wrap = True}"));
}

#[test]
fn a_catch_all_of_values_keeps_the_atoms_it_collects() {
    let settings: HashMap<String, Value> = match from_str("{theme = Dark wrap = True count = 2}") {
        Ok(settings) => settings,
        Err(error) => panic!("this should read, but: {error}"),
    };
    assert_eq!(settings["theme"], parsed("Dark"));
    assert_eq!(settings["wrap"], parsed("True"));
    assert_eq!(settings["count"], parsed("2"));
}

#[test]
fn an_atom_and_a_wide_integer_fill_their_own_types() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Entry {
        kind: Atom,
        id: Integer,
    }

    let entry: Entry = match from_str("{kind = Relative id = 123456789012345678901234567890}") {
        Ok(entry) => entry,
        Err(error) => panic!("this should read, but: {error}"),
    };
    assert_eq!(entry.kind.as_str(), "Relative");
    assert_eq!(entry.id.as_str(), "123456789012345678901234567890");

    assert!(from_str::<Entry>(r#"{kind = "Relative" id = 1}"#).is_err());
}
