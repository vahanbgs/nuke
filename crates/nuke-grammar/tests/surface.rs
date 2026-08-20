use nuke_grammar::{Grammar, Shape};

fn scalar(kind: &str) -> Shape {
    Shape::Scalar(kind.to_owned())
}

fn admitted(grammar: &Grammar, sources: &[&str]) {
    for source in sources {
        if let Err(error) = grammar.parse(source) {
            panic!("{source} should parse, but: {error}");
        }
    }
}

fn refused(grammar: &Grammar, sources: &[&str]) {
    for source in sources {
        assert!(
            grammar.parse(source).is_err(),
            "{source} should have been rejected"
        );
    }
}

#[test]
fn the_surface_grammar_translates_to_a_usable_parser() {
    Grammar::surface().expect("the surface ABNF should translate and compile");
}

#[test]
fn every_canonical_document_is_a_surface_document() {
    let grammar = Grammar::surface().unwrap();
    for fixture in nuke_fixtures::valid() {
        if let Err(error) = grammar.parse(&fixture.source) {
            panic!("{} should parse, but: {error}", fixture.display());
        }
    }
}

#[test]
fn the_surface_language_admits_five_documents_the_canonical_form_refuses_and_no_others() {
    let canonical = Grammar::canonical().unwrap();
    let surface = Grammar::surface().unwrap();
    let admits = [
        "bare-ident.nuke",
        "braceless-fields.nuke",
        "hex-literal.nuke",
        "ident-as-map-key.nuke",
        "interpolated-string.nuke",
    ];
    for fixture in nuke_fixtures::invalid() {
        assert!(
            canonical.parse(&fixture.source).is_err(),
            "{} is no longer a canonically invalid fixture",
            fixture.display()
        );
        assert_eq!(
            surface.parse(&fixture.source).is_ok(),
            admits.contains(&fixture.name()),
            "{} crosses the boundary between the two languages",
            fixture.display()
        );
    }
}

#[test]
fn every_surface_fixture_parses_completely() {
    let grammar = Grammar::surface().unwrap();
    for reduction in nuke_fixtures::reductions() {
        if let Err(error) = grammar.parse(&reduction.source.source) {
            panic!("{} should parse, but: {error}", reduction.display());
        }
    }
}

#[test]
fn every_recorded_reduction_is_itself_a_canonical_document() {
    let grammar = Grammar::canonical().unwrap();
    for reduction in nuke_fixtures::reductions() {
        if let Err(error) = grammar.parse(&reduction.reduced.source) {
            panic!("{} should parse, but: {error}", reduction.reduced.display());
        }
    }
}

#[test]
fn every_invalid_surface_fixture_is_rejected() {
    let grammar = Grammar::surface().unwrap();
    for fixture in nuke_fixtures::surface_invalid() {
        assert!(
            grammar.parse(&fixture.source).is_err(),
            "{} should have been rejected",
            fixture.display()
        );
    }
}

#[test]
fn the_fixtures_the_reduction_refuses_are_ones_the_grammar_admits() {
    let grammar = Grammar::surface().unwrap();
    for fixture in nuke_fixtures::surface_refused() {
        if let Err(error) = grammar.parse(&fixture.source) {
            panic!(
                "{} is refused by the grammar, so it belongs in surface/invalid: {error}",
                fixture.display()
            );
        }
    }
}

#[test]
fn a_binding_stands_at_the_head_of_a_document_and_of_a_brace_block() {
    let grammar = Grammar::surface().unwrap();
    admitted(
        &grammar,
        &[
            "n := 1 [n]",
            "n := 1 m := n {a = m}",
            "{n := 1 a = n}",
            "{n := 1 \"a\" => n}",
            "{n := 1}",
            "{n:=1 a=n}",
        ],
    );
}

#[test]
fn a_file_stands_in_for_the_braces_of_the_tuple_it_holds() {
    let grammar = Grammar::surface().unwrap();
    admitted(
        &grammar,
        &[
            "a = 1",
            "a = 1 b = 2",
            "a = 1\nb = 2",
            "n := 1 a = n",
            "a = {b = 1}",
            "a=1b=2",
        ],
    );
    assert_eq!(
        grammar.parse("bg = 1 fg = 2").unwrap().shape(),
        grammar.parse("{bg = 1 fg = 2}").unwrap().shape(),
        "a file's fields are the tuple its braces would have held"
    );
}

#[test]
fn nothing_stands_beside_a_file_of_fields_and_a_map_keeps_its_braces() {
    let grammar = Grammar::surface().unwrap();
    refused(
        &grammar,
        &[
            "a = 1 2",
            "1 a = 2",
            "a = 1 b",
            "a = 1 }",
            "\"a\" => 1",
            "n := 1 \"a\" => 1",
        ],
    );
    admitted(&grammar, &["{\"a\" => 1}"]);
}

#[test]
fn a_binding_after_a_pair_is_refused_and_a_list_holds_none_at_all() {
    let grammar = Grammar::surface().unwrap();
    refused(
        &grammar,
        &[
            "{a = 1 n := 2}",
            "{\"a\" => 1 n := 2}",
            "[n := 1]",
            "[1 n := 2]",
            "n := 1",
            "{a = n := 1}",
        ],
    );
}

#[test]
fn a_binding_is_not_a_field_and_a_block_that_holds_only_bindings_is_empty() {
    let grammar = Grammar::surface().unwrap();
    let cases = [
        (
            "{n := 1 a = n}",
            Shape::Tuple(vec![("a".to_owned(), scalar("reference"))]),
        ),
        ("{n := 1}", Shape::Tuple(vec![])),
        ("{}", Shape::Tuple(vec![])),
    ];
    for (source, expected) in cases {
        let parse = grammar
            .parse(source)
            .unwrap_or_else(|error| panic!("{source} should parse, but: {error}"));
        assert_eq!(parse.shape(), expected, "for {source}");
    }
}

#[test]
fn a_name_is_a_value_wherever_a_value_stands() {
    let grammar = Grammar::surface().unwrap();
    admitted(
        &grammar,
        &["n := 1 [n]", "{n := 1 a = n}", "{n := 1 n => 1}", "x"],
    );
    refused(&grammar, &["{n := 1 n = }", "{N := 1 a = N}", "n := "]);
}

#[test]
fn a_misspelled_number_is_one_bad_token_in_the_surface_language_too() {
    let grammar = Grammar::surface().unwrap();
    refused(
        &grammar,
        &["[1E5]", "[01]", "n := 1e05 [n]", "[1.]", "[1.b]"],
    );
}

#[test]
fn a_dyadic_literal_is_a_number_wherever_a_number_stands() {
    let grammar = Grammar::surface().unwrap();
    admitted(
        &grammar,
        &[
            "[0b1010]",
            "[0q3201]",
            "[0o755]",
            "[0xFE8019]",
            "[0b101110100xC]",
            "[0xF35b1]",
            "[0xDEADxBEEF]",
            "[-0xFF]",
            "n := 0xFF {a = n}",
            "{0xFF => 1}",
        ],
    );
    assert_eq!(
        grammar.parse("0xFF").unwrap().shape(),
        scalar("number"),
        "a base is a spelling of a number and not a form of its own"
    );
    refused(
        &grammar,
        &[
            "[0xff]", "[0b2]", "[0q4]", "[0o8]", "[0xG]", "[0x]", "[0x1.8]",
        ],
    );
    let canonical = Grammar::canonical().unwrap();
    refused(&canonical, &["[0xFF]", "[0b1010]"]);
}

#[test]
fn a_field_is_projected_out_of_whatever_stands_to_the_left_of_the_dot() {
    let grammar = Grammar::surface().unwrap();
    admitted(
        &grammar,
        &[
            "p := {a = 1} p.a",
            "p := {a = {b = 1}} p.a.b",
            "{a = 1}.a",
            "[1 2].a",
            "{\"a\" => 1}.a",
            "p := {a = 1} {b = p.a}",
            "p := {a = 1} {p.a => 1}",
        ],
    );
    refused(
        &grammar,
        &["p := {a = 1} p.", "p := {a = 1} p.1", ".a", "p.A"],
    );
}

#[test]
fn a_key_is_parenthesised_so_that_the_dot_evaluates_it_rather_than_reading_a_name() {
    let grammar = Grammar::surface().unwrap();
    admitted(
        &grammar,
        &[
            "m := {\"a\" => 1} m.(\"a\")",
            "m := {\"a\" => 1} k := \"a\" m.(k)",
            "m := {[1 2] => 1} m.([1 2])",
            "m := {{x = 1} => 1} m.({x = 1})",
            "[1 2].(0)",
            "p := {a = {\"b\" => {c = 1}}} p.a.(\"b\").c",
            "m := {\"a\" => 1} m . ( \"a\" )",
            "m := {\"a\" => 1} [m.(\"a\")]",
            "m := {\"a\" => 1} {b = m.(\"a\")}",
        ],
    );
    refused(
        &grammar,
        &[
            "m := {\"a\" => 1} m.()",
            "m := {\"a\" => 1} m.(\"a\"",
            "m := {\"a\" => 1} m.(\"a\" \"b\")",
            "m := {\"a\" => 1} m.\"a\"",
            "m := {\"a\" => 1} m.[\"a\"]",
            "1.(a)",
            "(1)",
        ],
    );
}

#[test]
fn the_dot_takes_whitespace_the_way_every_other_operator_does() {
    let grammar = Grammar::surface().unwrap();
    admitted(
        &grammar,
        &[
            "p := {a = {b = 1}} [p . a . b]",
            "p := {a = 1} [p. a]",
            "p := {a = 1} [p .a]",
            "p := {a = 1} [p # here\n.a]",
            "[1 . b]",
        ],
    );
}

#[test]
fn a_call_names_a_builtin_with_an_identifier_and_takes_exactly_one_operand() {
    let grammar = Grammar::surface().unwrap();
    admitted(
        &grammar,
        &[
            "@import \"p.nuke\"",
            "@import\"p.nuke\"",
            "@ import \"p.nuke\"",
            "@import # here\n\"p.nuke\"",
            "@import @import \"p.nuke\"",
            "@length [1 2]",
            "@merge {a = 1}",
            "p := \"p.nuke\" @import p",
        ],
    );
    refused(
        &grammar,
        &[
            "@",
            "@import",
            "@\"p.nuke\"",
            "@Import \"p.nuke\"",
            "@1 \"p.nuke\"",
            "@import \"a.nuke\" \"b.nuke\"",
            "\"p.nuke\"@",
            "@:= \"p.nuke\"",
        ],
    );
}

#[test]
fn a_call_stands_where_a_value_stands_and_the_dot_takes_what_it_yields() {
    let grammar = Grammar::surface().unwrap();
    admitted(
        &grammar,
        &[
            "[@import \"p.nuke\"]",
            "{a = @import \"p.nuke\"}",
            "{@import \"p.nuke\" => 1}",
            "{1 => @import \"p.nuke\"}",
            "p := @import \"p.nuke\" [p]",
            "@import \"p.nuke\".accent",
            "@import \"p.nuke\" . accent",
            "@import \"p.nuke\".a.b",
        ],
    );
}

#[test]
fn the_canonical_form_has_no_call_at_all() {
    let grammar = Grammar::canonical().unwrap();
    refused(
        &grammar,
        &[
            "@import \"p.nuke\"",
            "[@import \"p.nuke\"]",
            "{a = @import \"p.nuke\"}",
            "{@import \"p.nuke\" => 1}",
            "1 @import \"p.nuke\"",
        ],
    );
}

#[test]
fn a_projection_is_a_shape_of_its_own_until_it_is_reduced() {
    let grammar = Grammar::surface().unwrap();
    let cases = [
        ("p := {a = 1} p.a", scalar("access")),
        ("p := {a = 1} p", scalar("reference")),
        (
            "p := {a = 1} {b = p.a}",
            Shape::Tuple(vec![("b".to_owned(), scalar("access"))]),
        ),
        ("[{a = 1}.a]", Shape::List(vec![scalar("access")])),
        ("@import \"p.nuke\"", scalar("call")),
        ("@import \"p.nuke\".accent", scalar("access")),
        ("m := {\"a\" => 1} m.(\"a\")", scalar("access")),
        (
            "m := {\"a\" => 1} [m.(\"a\")]",
            Shape::List(vec![scalar("access")]),
        ),
        (r#"$"a{b}""#, scalar("interpolation")),
        (
            r#"{a = $"x"}"#,
            Shape::Tuple(vec![("a".to_owned(), scalar("interpolation"))]),
        ),
        (
            "{a = @import \"p.nuke\"}",
            Shape::Tuple(vec![("a".to_owned(), scalar("call"))]),
        ),
    ];
    for (source, expected) in cases {
        let parse = grammar
            .parse(source)
            .unwrap_or_else(|error| panic!("{source} should parse, but: {error}"));
        assert_eq!(parse.shape(), expected, "for {source}");
    }
}

#[test]
fn an_interpolated_string_holds_text_holes_and_doubled_braces() {
    let grammar = Grammar::surface().unwrap();
    admitted(
        &grammar,
        &[
            r#"$"a""#,
            r#"$"""#,
            r##"$"#{accent}""##,
            r#"$"{a}{b}""#,
            r#"$"{ a }""#,
            r#"$"{p.a}""#,
            r#"$"{@import "./p.nuke"}""#,
            r#"$"a{{b}}c""#,
            r#"$"{ {a = 1}.a }""#,
            r#"$"{$"{a}"}""#,
            r#"$"tab\there \u{2713}""#,
            r#"[$"a" $"b"]"#,
            r#"{a = $"x{y}"}"#,
            r#"$"a" . b"#,
        ],
    );
}

#[test]
fn a_hole_carries_a_format_specifier_spelled_the_way_rust_spells_one() {
    let grammar = Grammar::surface().unwrap();
    admitted(
        &grammar,
        &[
            r#"$"{a:6}""#,
            r#"$"{a:<6}""#,
            r#"$"{a:*^6}""#,
            r#"$"{a:>>6}""#,
            r#"$"{a: >6}""#,
            r#"$"{a:#^6}""#,
            r#"$"{a:+}""#,
            r#"$"{a:#x}""#,
            r#"$"{a:#010X}""#,
            r#"$"{a:.2}""#,
            r#"$"{a:.3e}""#,
            r#"$"{a:>8.2}""#,
            r#"$"{p.a:6}""#,
            r#"$"{a :6}""#,
        ],
    );
    refused(
        &grammar,
        &[
            r#"$"{a:>8 }""#,
            r#"$"{a:z}""#,
            r#"$"{a:.}""#,
            r#"$"{a:007}""#,
            r#"$"{a:{b}}""#,
            r#"$"{a:>>>}""#,
        ],
    );
}

#[test]
fn a_plain_string_is_all_text_however_many_braces_it_holds() {
    let grammar = Grammar::surface().unwrap();
    admitted(
        &grammar,
        &[r#""a{b}""#, r#""{icon} {percentage}%""#, r#""}""#],
    );
}

#[test]
fn a_hole_holds_one_value_and_a_lone_brace_closes_nothing() {
    let grammar = Grammar::surface().unwrap();
    refused(
        &grammar,
        &[
            r#"$"a}""#,
            r#"$"{}""#,
            r#"$"{a b}""#,
            r#"$"{a := 1}""#,
            r#"$"a"#,
            r#"$"{a""#,
            r#"$ "a""#,
            r#"$a"#,
            r#"$"#,
        ],
    );
}

#[test]
fn the_canonical_form_has_no_interpolation_at_all() {
    let grammar = Grammar::canonical().unwrap();
    refused(
        &grammar,
        &[
            r#"$"a""#,
            r#"$"{a}""#,
            r#"[$"a"]"#,
            r#"{a = $"b"}"#,
            r#"1 $"a""#,
        ],
    );
}
