use std::collections::BTreeMap;

use nuke_syntax::ser::ErrorKind;
use nuke_syntax::{MAX_DEPTH, Value, from_value, parse, to_value};
use serde::ser::{SerializeMap, SerializeStruct};
use serde::{Deserialize, Serialize, Serializer};

fn written<T: Serialize + ?Sized>(value: &T) -> Value {
    match to_value(value) {
        Ok(value) => value,
        Err(error) => panic!("this value should be written, but: {error}"),
    }
}

fn fault<T: Serialize + ?Sized>(value: &T) -> ErrorKind {
    match to_value(value) {
        Ok(value) => panic!("this value should not be written, but gave {value:?}"),
        Err(error) => error.kind().clone(),
    }
}

fn parsed(source: &str) -> Value {
    match parse(source) {
        Ok(value) => value,
        Err(error) => panic!("{source} should parse, but: {error}"),
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Editor {
    theme: String,
    tab_width: u8,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Shell {
    history_size: u32,
    prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
enum Action {
    OpenTerminal,
    Run(String),
    Move(i32, i32),
    Resize { width: u32, height: u32 },
}

#[test]
fn a_struct_writes_a_tuple_and_a_map_writes_a_map() {
    let editor = Editor {
        theme: "gruvbox-dark".to_owned(),
        tab_width: 2,
    };
    assert_eq!(
        written(&editor),
        parsed(r#"{theme = "gruvbox-dark" tab_width = 2}"#)
    );

    let aliases = BTreeMap::from([("ll", "eza -l"), ("gs", "git status")]);
    assert_eq!(
        written(&aliases),
        parsed(r#"{"gs" => "git status" "ll" => "eza -l"}"#)
    );
}

#[test]
fn an_empty_map_writes_the_empty_block() {
    let empty: BTreeMap<String, i32> = BTreeMap::new();
    assert_eq!(written(&empty), parsed("{}"));
    assert_eq!(written(&empty), Value::Tuple(nuke_syntax::Tuple::new()));
}

#[test]
fn a_field_name_that_is_not_an_identifier_cannot_be_written() {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Camel {
        tab_width: u8,
    }

    assert_eq!(
        fault(&Camel { tab_width: 2 }),
        ErrorKind::FieldName("tabWidth".to_owned())
    );
}

#[test]
fn nan_and_infinity_have_no_canonical_spelling() {
    assert_eq!(written(&1.5_f64), parsed("1.5"));
    assert_eq!(fault(&f64::NAN), ErrorKind::NotFinite);
    assert_eq!(fault(&f64::INFINITY), ErrorKind::NotFinite);
    assert_eq!(fault(&f32::NEG_INFINITY), ErrorKind::NotFinite);
}

#[test]
fn an_integer_of_any_width_survives_being_written() {
    assert_eq!(
        written(&i128::MIN),
        parsed("-170141183460469231731687303715884105728")
    );
    assert_eq!(
        written(&u128::MAX),
        parsed("340282366920938463463374607431768211455")
    );
    assert_eq!(written(&0_i32), parsed("0"));
}

#[test]
fn a_none_field_is_left_out_rather_than_written() {
    let quiet = Shell {
        history_size: 10000,
        prompt: None,
    };
    assert_eq!(written(&quiet), parsed("{history_size = 10000}"));
}

#[test]
fn absence_cannot_be_written_where_a_field_cannot_be_dropped() {
    assert_eq!(
        fault(&None::<i32>),
        ErrorKind::Absent("at the top of a document")
    );
    assert_eq!(fault(&vec![Some(1), None]), ErrorKind::Absent("in a list"));
    assert_eq!(
        fault(&BTreeMap::from([("a", None::<i32>)])),
        ErrorKind::Absent("as a map value")
    );
}

#[test]
fn an_option_field_survives_being_written_and_read_back() {
    for shell in [
        Shell {
            history_size: 10000,
            prompt: None,
        },
        Shell {
            history_size: 10000,
            prompt: Some("$ ".to_owned()),
        },
    ] {
        let value = written(&shell);
        assert_eq!(from_value::<Shell>(&value), Ok(shell));
    }
}

#[test]
fn an_enum_writes_the_shape_the_deserializer_reads() {
    assert_eq!(written(&Action::OpenTerminal), parsed("OpenTerminal"));
    assert_eq!(
        written(&Action::Run("vim".to_owned())),
        parsed(r#"{Run => "vim"}"#)
    );
    assert_eq!(written(&Action::Move(1, -1)), parsed("{Move => [1 -1]}"));
    assert_eq!(
        written(&Action::Resize {
            width: 80,
            height: 24,
        }),
        parsed("{Resize => {width = 80 height = 24}}")
    );

    for action in [
        Action::OpenTerminal,
        Action::Run("vim".to_owned()),
        Action::Move(1, -1),
        Action::Resize {
            width: 80,
            height: 24,
        },
    ] {
        let value = written(&action);
        assert_eq!(from_value::<Action>(&value), Ok(action));
    }
}

#[test]
fn a_variant_no_atom_can_hold_is_tagged_by_a_string() {
    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    enum Theme {
        GruvboxDark,
        Custom(String),
    }

    assert_eq!(written(&Theme::GruvboxDark), parsed(r#""gruvbox_dark""#));
    assert_eq!(
        written(&Theme::Custom("solarized".to_owned())),
        parsed(r#"{"custom" => "solarized"}"#)
    );
}

#[test]
fn a_repeated_key_is_refused_the_way_the_parser_refuses_one() {
    struct Twice;

    impl Serialize for Twice {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut entries = serializer.serialize_map(Some(2))?;
            entries.serialize_entry("a", &1)?;
            entries.serialize_entry("a", &2)?;
            entries.end()
        }
    }

    struct Both;

    impl Serialize for Both {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut fields = serializer.serialize_struct("Both", 2)?;
            fields.serialize_field("a", &1)?;
            fields.serialize_field("a", &2)?;
            fields.end()
        }
    }

    assert_eq!(fault(&Twice), ErrorKind::DuplicateKey);
    assert_eq!(fault(&Both), ErrorKind::DuplicateField("a".to_owned()));
}

#[test]
fn a_value_written_past_the_depth_limit_is_refused() {
    struct Nest(usize);

    impl Serialize for Nest {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            match self.0 {
                0 => serializer.serialize_i32(0),
                deeper => serializer.collect_seq([Self(deeper - 1)]),
            }
        }
    }

    assert!(to_value(&Nest(MAX_DEPTH)).is_ok());
    assert_eq!(fault(&Nest(MAX_DEPTH + 1)), ErrorKind::TooDeep);
}

#[test]
fn bytes_are_written_as_a_list_of_integers() {
    struct Raw;

    impl Serialize for Raw {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_bytes(&[0, 127, 255])
        }
    }

    assert_eq!(written(&Raw), parsed("[0 127 255]"));
}

#[test]
fn a_char_writes_a_string_and_a_unit_struct_writes_the_empty_block() {
    #[derive(Serialize)]
    struct Marker;

    assert_eq!(written(&'é'), parsed(r#""é""#));
    assert_eq!(written(&Marker), parsed("{}"));
}
