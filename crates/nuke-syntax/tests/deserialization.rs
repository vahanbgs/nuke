use std::collections::HashMap;
use std::fmt::Debug;

use nuke_syntax::de::ErrorKind;
use nuke_syntax::from_str;
use serde::Deserialize;
use serde::de::DeserializeOwned;

fn read<T: DeserializeOwned>(source: &str) -> T {
    match from_str(source) {
        Ok(value) => value,
        Err(error) => panic!("{source} should read, but: {error}"),
    }
}

fn fault<T: DeserializeOwned + Debug>(source: &str) -> ErrorKind {
    match from_str::<T>(source) {
        Ok(value) => panic!("{source} should not read, but gave {value:?}"),
        Err(error) => error.kind().clone(),
    }
}

fn mismatched<T: DeserializeOwned + Debug>(source: &str) -> bool {
    matches!(fault::<T>(source), ErrorKind::Mismatch { .. })
}

#[derive(Debug, Deserialize, PartialEq)]
struct Editor {
    theme: String,
    tab_width: u8,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Shell {
    history_size: u32,
    prompt: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
enum Action {
    OpenTerminal,
    Run(String),
    Move(i32, i32),
    Resize { width: u32, height: u32 },
}

#[test]
fn a_struct_reads_a_tuple_and_refuses_a_map() {
    assert_eq!(
        read::<Editor>(r#"{theme = "gruvbox-dark" tab_width = 2}"#),
        Editor {
            theme: "gruvbox-dark".to_owned(),
            tab_width: 2,
        }
    );
    assert!(mismatched::<Editor>(
        r#"{"theme" => "gruvbox-dark" "tab_width" => 2}"#
    ));
}

#[test]
fn a_map_reads_a_map_and_also_a_tuple_because_flatten_needs_it() {
    let entries: HashMap<String, i32> = read(r#"{"a" => 1 "b" => 2}"#);
    let fields: HashMap<String, i32> = read("{a = 1 b = 2}");
    assert_eq!(entries.len(), 2);
    assert_eq!(fields, entries);
}

#[test]
fn an_empty_block_fills_an_empty_map() {
    let empty: HashMap<String, i32> = read("{}");
    assert!(empty.is_empty());
}

#[test]
fn a_field_the_struct_does_not_name_is_skipped() {
    assert_eq!(
        read::<Editor>(r#"{theme = "gruvbox-dark" tab_width = 2 wrap = [1 2]}"#),
        Editor {
            theme: "gruvbox-dark".to_owned(),
            tab_width: 2,
        }
    );
}

#[test]
fn true_and_false_are_the_only_atoms_a_bool_accepts() {
    assert!(read::<bool>("True"));
    assert!(!read::<bool>("False"));
    assert!(mismatched::<bool>("Null"));
    assert!(mismatched::<bool>("Yes"));
    assert!(mismatched::<bool>(r#""True""#));
}

#[test]
fn null_is_an_atom_like_any_other_and_does_not_fill_an_option() {
    assert!(mismatched::<Option<i32>>("Null"));
    assert_eq!(read::<Option<String>>(r#""text""#), Some("text".to_owned()));
}

#[test]
fn a_missing_field_is_none_and_a_present_one_is_some() {
    assert_eq!(
        read::<Shell>("{history_size = 10000}"),
        Shell {
            history_size: 10000,
            prompt: None,
        }
    );
    assert_eq!(
        read::<Shell>(r#"{history_size = 10000 prompt = "$ "}"#),
        Shell {
            history_size: 10000,
            prompt: Some("$ ".to_owned()),
        }
    );
}

#[test]
fn a_float_field_refuses_an_integer_because_the_two_are_distinct() {
    assert!((read::<f64>("-2.5e-3") - -0.0025).abs() < f64::EPSILON);
    assert!(mismatched::<f64>("1"));
    assert!(mismatched::<i32>("1.0"));
}

#[test]
fn an_integer_wider_than_its_field_names_the_width_it_could_not_fill() {
    assert_eq!(read::<u8>("255"), 255);
    assert_eq!(
        fault::<u8>("256"),
        ErrorKind::IntegerOutOfRange {
            text: "256".to_owned(),
            wanted: "u8",
        }
    );
    assert_eq!(
        read::<i128>("-170141183460469231731687303715884105728"),
        i128::MIN
    );
}

#[test]
fn a_string_field_refuses_an_atom() {
    assert_eq!(read::<String>(r#""text""#), "text");
    assert!(mismatched::<String>("Text"));
}

#[test]
fn a_list_fills_a_sequence_and_a_rust_tuple_of_the_same_length() {
    assert_eq!(read::<Vec<i32>>("[1 2 3]"), vec![1, 2, 3]);
    assert_eq!(read::<(i32, String)>(r#"[1 "two"]"#), (1, "two".to_owned()));
    assert!(matches!(
        fault::<(i32, String)>(r#"[1 "two" 3]"#),
        ErrorKind::Message(_)
    ));
}

#[test]
fn a_map_key_is_any_value_the_rust_type_can_build() {
    let by_number: HashMap<i64, String> = read(r#"{1 => "one" 2 => "two"}"#);
    assert_eq!(by_number[&1], "one");
    assert_eq!(by_number[&2], "two");
    assert!(mismatched::<HashMap<String, i32>>(r#"{1 => 1}"#));
}

#[test]
fn a_unit_struct_is_the_empty_block() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Marker;

    assert_eq!(read::<Marker>("{}"), Marker);
    assert!(mismatched::<Marker>("{a = 1}"));
}

#[test]
fn a_unit_variant_is_a_bare_atom_and_the_others_are_a_single_entry_map() {
    assert_eq!(read::<Action>("OpenTerminal"), Action::OpenTerminal);
    assert_eq!(
        read::<Action>(r#"{Run => "vim"}"#),
        Action::Run("vim".to_owned())
    );
    assert_eq!(read::<Action>("{Move => [1 -1]}"), Action::Move(1, -1));
    assert_eq!(
        read::<Action>("{Resize => {width = 80 height = 24}}"),
        Action::Resize {
            width: 80,
            height: 24,
        }
    );
}

#[test]
fn a_unit_variant_written_as_a_map_says_how_to_write_it() {
    assert!(matches!(
        fault::<Action>("{OpenTerminal => Null}"),
        ErrorKind::Message(_)
    ));
    assert!(mismatched::<Action>(r#"{Run => "vim" Move => [1 -1]}"#));
}

#[test]
fn a_variant_renamed_out_of_atom_shape_is_keyed_by_a_string() {
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum Theme {
        GruvboxDark,
        Custom(String),
    }

    assert_eq!(read::<Theme>(r#""gruvbox_dark""#), Theme::GruvboxDark);
    assert_eq!(
        read::<Theme>(r#"{"custom" => "solarized"}"#),
        Theme::Custom("solarized".to_owned())
    );
}

#[test]
fn an_internally_tagged_enum_reads_a_tag_written_as_an_atom_or_as_a_string() {
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(tag = "kind")]
    enum Source {
        File { path: String },
        Url { url: String },
    }

    let expected = Source::File {
        path: "/etc/hosts".to_owned(),
    };
    assert_eq!(
        read::<Source>(r#"{kind = File path = "/etc/hosts"}"#),
        expected
    );
    assert_eq!(
        read::<Source>(r#"{kind = "File" path = "/etc/hosts"}"#),
        expected
    );
    assert_eq!(
        read::<Source>(r#"{kind = Url url = "https://example.com"}"#),
        Source::Url {
            url: "https://example.com".to_owned(),
        }
    );
}

#[test]
fn flatten_collects_the_fields_the_struct_does_not_name() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Program {
        name: String,
        #[serde(flatten)]
        rest: HashMap<String, String>,
    }

    let program: Program = read(r#"{name = "nuke" home = "/tmp" shell = "fish"}"#);
    assert_eq!(program.name, "nuke");
    assert_eq!(program.rest.len(), 2);
    assert_eq!(program.rest["shell"], "fish");
}

#[test]
fn an_atom_loses_its_atom_hood_when_serde_buffers_it() {
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(untagged)]
    enum Loose {
        Number(i64),
        Text(String),
    }

    assert_eq!(read::<Loose>("42"), Loose::Number(42));
    assert_eq!(
        read::<Loose>(r#""Relative""#),
        Loose::Text("Relative".to_owned())
    );
    assert_eq!(
        read::<Loose>("Relative"),
        Loose::Text("Relative".to_owned())
    );
}

#[test]
fn a_syntax_error_arrives_with_the_span_the_parser_gave_it() {
    let error = match from_str::<Vec<i32>>("[1 2") {
        Ok(value) => panic!("an unterminated list should not read, but gave {value:?}"),
        Err(error) => error,
    };
    assert!(matches!(error.kind(), ErrorKind::Syntax(_)));
    assert!(error.span().is_some());
}
