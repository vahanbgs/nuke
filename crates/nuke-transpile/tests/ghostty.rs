use nuke_syntax::parse;
use nuke_transpile::ghostty::{Error, ErrorKind, to_string};

fn ghostty(source: &str) -> String {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect("the value should transpile")
}

fn value(source: &str) -> String {
    let written = ghostty(&format!("{{a = {source}}}"));
    written
        .strip_prefix("a = ")
        .expect("a scalar is written as the one option of the document")
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
        assert!(error.path().is_root(), "for {source}");
        assert_eq!(error.path().to_string(), "the document");
    }
    assert_eq!(ghostty("{}"), "");
}

#[test]
fn the_file_is_one_level_deep_so_a_table_below_the_root_is_refused() {
    for (source, form, path) in [
        ("{a = {b = 1}}", "tuple", "a"),
        ("{a = {}}", "tuple", "a"),
        ("{a = 1 b = {c = 2}}", "tuple", "b"),
        ("{a = {1 => 2}}", "map", "a"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableValue(form),
            "{error}"
        );
        assert_eq!(error.path().to_string(), path, "for {source}");
    }
}

#[test]
fn an_option_is_one_line_and_the_file_does_not_end_with_one() {
    let written = ghostty(r#"{a = 1 b = "two" c = 3.5}"#);
    assert_eq!(written, "a = 1\nb = two\nc = 3.5");
    assert!(!written.ends_with('\n'));
}

#[test]
fn a_repeated_key_is_a_list_and_order_survives() {
    assert_eq!(
        ghostty(r#"{a = ["one" "two" "three"]}"#),
        "a = one\na = two\na = three"
    );
    assert_eq!(ghostty("{a = [1] b = 2}"), "a = 1\nb = 2");
}

#[test]
fn a_one_element_list_and_a_scalar_are_one_text() {
    assert_eq!(ghostty("{a = 1}"), ghostty("{a = [1]}"));
}

#[test]
fn an_empty_list_is_refused_because_the_option_would_be_lost() {
    let error = refused("{a = []}");
    assert_eq!(error.kind(), &ErrorKind::EmptyList, "{error}");
    assert_eq!(error.path().to_string(), "a");
}

#[test]
fn a_list_holding_a_collection_has_no_spelling() {
    for (source, form, path) in [
        ("{a = [[1]]}", "list", "a[0]"),
        ("{a = [1 {b = 2}]}", "tuple", "a[1]"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableValue(form),
            "{error}"
        );
        assert_eq!(error.path().to_string(), path, "for {source}");
    }
}

#[test]
fn the_empty_string_is_refused_because_an_empty_value_resets_its_option() {
    for (source, path) in [(r#"{a = ""}"#, "a"), (r#"{a = ["x" ""]}"#, "a[1]")] {
        let error = refused(source);
        assert_eq!(error.kind(), &ErrorKind::EmptyString, "{error}");
        assert_eq!(error.path().to_string(), path, "for {source}");
    }
}

#[test]
fn an_atom_is_lowered_into_ghosttys_case() {
    for (atom, spelling) in [
        ("True", "true"),
        ("False", "false"),
        ("Null", "null"),
        ("Bar", "bar"),
        ("Relative", "relative"),
        ("TopLeft", "top-left"),
        ("AfterFirst", "after-first"),
        ("OpenTerminal", "open-terminal"),
        ("NonNativePaddedNotch", "non-native-padded-notch"),
    ] {
        assert_eq!(value(atom), spelling, "for {atom}");
    }
}

#[test]
fn a_digit_joins_the_word_before_it_and_opens_none() {
    for (atom, spelling) in [
        ("DisplayP3", "display-p3"),
        ("Osc8", "osc8"),
        ("Atom99", "atom99"),
    ] {
        assert_eq!(value(atom), spelling, "for {atom}");
    }
}

#[test]
fn the_two_values_the_rule_cannot_reach_are_written_as_strings() {
    assert_eq!(value("BlockHollow"), "block-hollow");
    assert_eq!(value("IoUring"), "io-uring");
    assert_eq!(value(r#""block_hollow""#), "block_hollow");
    assert_eq!(value(r#""io_uring""#), "io_uring");
}

#[test]
fn an_atom_is_lowered_in_value_position_and_never_names_an_option() {
    let error = refused("{Relative => 1}");
    assert_eq!(
        error.kind(),
        &ErrorKind::UnspellableName("Relative".to_owned()),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "#1");
}

#[test]
fn a_number_keeps_the_shape_it_had() {
    assert_eq!(value("42"), "42");
    assert_eq!(
        value("170141183460469231731687303715884105728"),
        "170141183460469231731687303715884105728"
    );
    assert_eq!(value("1.0"), "1.0");
    assert_eq!(value("1.5"), "1.5");
}

#[test]
fn a_value_is_bare_where_a_comment_cannot_begin() {
    for text in ["#FE8019", "0=#FE8019", ";x", "?x", "a # b", "a=b", "a\"b"] {
        assert_eq!(value(&format!("{text:?}")), text);
    }
}

#[test]
fn a_value_whose_edge_is_whitespace_is_fenced_so_the_trim_gives_it_back() {
    assert_eq!(value(r#"" ""#), "\" \"");
    assert_eq!(value(r#"" a""#), "\" a\"");
    assert_eq!(value(r#""a ""#), "\"a \"");
    assert_eq!(value(r#""a b""#), "a b");
}

#[test]
fn a_value_that_is_already_fenced_is_fenced_again_so_the_strip_gives_it_back() {
    assert_eq!(value(r#""\"a\"""#), "\"\"a\"\"");
    assert_eq!(value(r#""\"\"""#), "\"\"\"\"");
    assert_eq!(value(r#""\"""#), "\"");
    assert_eq!(value(r#""\"a""#), "\"a");
    assert_eq!(value(r#""a\"""#), "a\"");
}

#[test]
fn a_control_is_refused_because_there_is_no_escape_to_write_one_with() {
    for (source, character) in [
        (r#""a\nb""#, '\n'),
        (r#""a\tb""#, '\t'),
        (r#""a\rb""#, '\r'),
        (r#""a\u{0}b""#, '\u{0}'),
        (r#""a\u{1F}b""#, '\u{1F}'),
    ] {
        let error = refused(&format!("{{a = {source}}}"));
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableCharacter(character),
            "{error}"
        );
    }
    assert_eq!(value(r#""a\u{7F}b""#), "a\u{7F}b");
    assert_eq!(value(r#""é ✓ 😀""#), "é ✓ 😀");
}

#[test]
fn a_name_ghostty_cannot_spell_is_one_only_a_key_can_carry() {
    for (source, name) in [
        (r#"{"Title" => 1}"#, "Title"),
        (r#"{"font-Family" => 1}"#, "font-Family"),
        (r#"{"2x" => 1}"#, "2x"),
        (r#"{"" => 1}"#, ""),
        (r#"{"a b" => 1}"#, "a b"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::UnspellableName(name.to_owned()),
            "{error}"
        );
    }
    assert_eq!(ghostty(r#"{"font-family" => 1}"#), "font-family = 1");
    assert_eq!(ghostty("{a2 = 1}"), "a2 = 1");
}

#[test]
fn a_key_that_is_neither_a_string_nor_an_atom_is_refused() {
    for (source, form) in [
        ("{1 => 2}", "integer"),
        ("{[1] => 2}", "list"),
        ("{{a = 1} => 2}", "tuple"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableKey(form),
            "{error}"
        );
        assert_eq!(error.path().to_string(), "#1", "for {source}");
    }
}

#[test]
fn no_two_names_can_name_one_option_so_there_is_no_fold_to_refuse() {
    assert!(parse(r#"{"a" => 1 "a" => 2}"#).is_err());
    assert!(
        parse("{a_b = 1 a_b = 2}").is_err(),
        "a tuple refuses a repeated field, and `-` never occurs in an identifier, so the transliteration is injective"
    );
    let error = refused(r#"{Relative => 1 "Relative" => 2}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::UnspellableName("Relative".to_owned()),
        "an atom is capitalised, so the collision JSON refuses cannot spell a Ghostty option: {error}"
    );
}

#[test]
fn a_field_is_transliterated_into_the_alphabet_and_a_key_is_taken_as_it_stands() {
    assert_eq!(
        ghostty(r#"{font_family = "FiraCode Nerd Font"}"#),
        "font-family = FiraCode Nerd Font"
    );
    assert_eq!(ghostty("{tab_width = 2}"), "tab-width = 2");
    assert_eq!(
        ghostty("{font_family = 1}"),
        ghostty(r#"{"font-family" => 1}"#),
        "the field and the key reach one option, and a tuple and a map are never siblings"
    );
    let error = refused(r#"{"font_family" => 1}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::UnspellableName("font_family".to_owned()),
        "a string is a literal and is written as it stands, so the key stays refused: {error}"
    );
}

#[test]
fn the_transliteration_leaves_a_value_alone() {
    assert_eq!(
        ghostty(r#"{shell_integration_features = "no-cursor, ssh-env"}"#),
        "shell-integration-features = no-cursor, ssh-env"
    );
    assert_eq!(ghostty(r#"{command = "a_b"}"#), "command = a_b");
}

#[test]
fn every_identifier_spells_a_name_and_no_two_spell_one() {
    let mut written = std::collections::HashSet::new();
    let mut count = 0;
    for first in ['a', 'z'] {
        for rest in ["", "a", "z", "0", "9", "_", "_a", "a_", "__", "9_z"] {
            let field = format!("{first}{rest}");
            let text = ghostty(&format!("{{{field} = 1}}"));
            let name = text.strip_suffix(" = 1").expect("one option on one line");
            assert!(
                !name.contains('_')
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "`{field}` spells `{name}`, which is not a name Ghostty could give an option"
            );
            count += 1;
            assert!(
                written.insert(name.to_owned()),
                "`{field}` folded onto `{name}`"
            );
        }
    }
    assert_eq!(written.len(), count, "the map is injective");
}

#[test]
fn a_dot_file_is_not_a_ghostty_config_because_a_ghostty_config_has_no_section() {
    let source = include_str!("../../../fixtures/valid/dotfile.nuke");
    let error = refused(source);
    assert_eq!(error.kind(), &ErrorKind::UnrepresentableValue("tuple"));
    assert_eq!(error.path().to_string(), "editor");
}
