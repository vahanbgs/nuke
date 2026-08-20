use nuke_grammar::Grammar;
use nuke_syntax::{Align, ErrorKind, ExprKind, Form, Notation, Piece, Spec, parse, surface};

fn faults() -> Vec<(&'static str, ErrorKind)> {
    vec![
        (
            "access-into-a-number.nuke",
            ErrorKind::NumberSpelling("1.".to_owned()),
        ),
        (
            "access-without-a-field-name.nuke",
            ErrorKind::ExpectedAccessName,
        ),
        (
            "binding-a-name-that-is-not-one.nuke",
            ErrorKind::ExpectedBindingName,
        ),
        ("a-map-that-needs-its-braces.nuke", ErrorKind::TrailingInput),
        (
            "a-value-beside-a-field-run.nuke",
            ErrorKind::ExpectedFieldName,
        ),
        ("binding-after-a-field.nuke", ErrorKind::MisplacedBinding),
        ("binding-after-an-entry.nuke", ErrorKind::MisplacedBinding),
        ("binding-as-a-value.nuke", ErrorKind::MisplacedBinding),
        ("binding-in-a-list.nuke", ErrorKind::MisplacedBinding),
        ("binding-without-a-document.nuke", ErrorKind::OnlyBindings),
        (
            "a-call-without-a-builtin-name.nuke",
            ErrorKind::ExpectedBuiltinName,
        ),
        ("a-call-without-an-operand.nuke", ErrorKind::ExpectedValue),
        (
            "a-lone-brace-in-an-interpolation.nuke",
            ErrorKind::UnmatchedBrace,
        ),
        ("a-hole-with-no-value.nuke", ErrorKind::ExpectedValue),
        (
            "a-hole-that-holds-two-values.nuke",
            ErrorKind::ExpectedHoleClose,
        ),
        (
            "a-key-that-is-never-closed.nuke",
            ErrorKind::UnterminatedKey,
        ),
        (
            "a-key-that-holds-two-values.nuke",
            ErrorKind::ExpectedKeyClose,
        ),
        ("a-projection-with-no-key.nuke", ErrorKind::ExpectedValue),
        (
            "an-interpolation-that-is-never-closed.nuke",
            ErrorKind::UnterminatedString,
        ),
        ("a-specifier-that-is-not-one.nuke", ErrorKind::MalformedSpec),
        (
            "a-hex-digit-in-lower-case.nuke",
            ErrorKind::NumberSpelling("0xff".to_owned()),
        ),
        (
            "a-digit-a-base-has-not-got.nuke",
            ErrorKind::NumberSpelling("0b2".to_owned()),
        ),
        (
            "a-marker-with-no-digits.nuke",
            ErrorKind::NumberSpelling("0x".to_owned()),
        ),
        (
            "a-dyadic-literal-with-a-point.nuke",
            ErrorKind::NumberSpelling("0x1.8".to_owned()),
        ),
    ]
}

fn fault(source: &str) -> ErrorKind {
    surface::parse(source)
        .err()
        .unwrap_or_else(|| panic!("{source} should have been rejected"))
        .kind()
        .clone()
}

fn document(source: &str) -> nuke_syntax::Document {
    surface::parse(source).unwrap_or_else(|error| panic!("{source} should parse, but: {error}"))
}

fn integer(source: &str) -> String {
    let ExprKind::Integer(integer) = document(source).value.kind else {
        panic!("{source} should be an integer");
    };
    integer.as_str().to_owned()
}

#[test]
fn every_surface_fixture_parses() {
    for reduction in nuke_fixtures::reductions() {
        if let Err(error) = surface::parse(&reduction.source.source) {
            panic!(
                "{} should parse, but at {}: {error}",
                reduction.display(),
                error.location(&reduction.source.source)
            );
        }
    }
}

#[test]
fn every_canonical_document_is_a_surface_document() {
    for fixture in nuke_fixtures::valid().into_iter().chain(
        nuke_fixtures::reductions()
            .into_iter()
            .map(|pair| pair.reduced),
    ) {
        if let Err(error) = surface::parse(&fixture.source) {
            panic!("{} should parse, but: {error}", fixture.display());
        }
    }
}

#[test]
fn every_fixture_the_reduction_refuses_is_one_the_parser_reads() {
    for fixture in nuke_fixtures::surface_refused() {
        if let Err(error) = surface::parse(&fixture.source) {
            panic!(
                "{} is refused by the parser, so it belongs in surface/invalid: {error}",
                fixture.display()
            );
        }
    }
}

#[test]
fn every_invalid_surface_fixture_is_rejected_by_the_error_that_names_its_fault() {
    let faults = faults();
    let fixtures = nuke_fixtures::surface_invalid();
    assert_eq!(
        faults.len(),
        fixtures.len(),
        "every invalid surface fixture needs an expected error"
    );
    for fixture in fixtures {
        let (_, expected) = faults
            .iter()
            .find(|(name, _)| *name == fixture.name())
            .unwrap_or_else(|| panic!("{} has no expected error", fixture.name()));
        let error = surface::parse(&fixture.source)
            .err()
            .unwrap_or_else(|| panic!("{} should have been rejected", fixture.display()));
        assert_eq!(error.kind(), expected, "for {}", fixture.name());
    }
}

#[test]
fn the_parser_and_the_grammar_agree_on_every_fixture() {
    let grammar = Grammar::surface().expect("the surface ABNF should translate and compile");
    let fixtures = nuke_fixtures::valid()
        .into_iter()
        .chain(nuke_fixtures::invalid())
        .chain(nuke_fixtures::surface_invalid())
        .chain(nuke_fixtures::surface_refused())
        .chain(nuke_fixtures::surface_modules())
        .chain(
            nuke_fixtures::reductions()
                .into_iter()
                .flat_map(|pair| [pair.source, pair.reduced]),
        );
    for fixture in fixtures {
        assert_eq!(
            surface::parse(&fixture.source).is_ok(),
            grammar.accepts(&fixture.source),
            "the parser and the grammar disagree about {}",
            fixture.display()
        );
    }
}

#[test]
fn the_parser_is_stricter_than_the_grammar_where_the_spec_says_so() {
    let grammar = Grammar::surface().unwrap();
    let chain = format!("p := {{a = 1}} p{}", ".a".repeat(200));
    for source in [
        "{n := 1 n := 2 a = n}",
        "{a = 1 a = 2}",
        "a = 1 a = 2",
        "[\"\\u{D800}\"]",
        "[1e400]",
        "[0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF]",
        r#"$"{a:}""#,
        &chain,
    ] {
        assert!(grammar.accepts(source), "the grammar should admit {source}");
        assert!(
            surface::parse(source).is_err(),
            "the parser should reject {source}"
        );
    }
    assert_eq!(
        fault(&chain),
        ErrorKind::TooDeep,
        "a projection is a level of the expression, so a chain spends the same budget nesting does"
    );
}

#[test]
fn a_block_binds_a_name_once_and_an_inner_block_may_shadow_it() {
    assert_eq!(
        fault("{n := 1 n := 2 a = n}"),
        ErrorKind::DuplicateBinding("n".to_owned())
    );
    assert_eq!(
        fault("n := 1 n := 2 [n]"),
        ErrorKind::DuplicateBinding("n".to_owned())
    );
    document("{n := 1 a = {n := 2 b = n}}");
    document("n := 1 {n := 2 a = n}");
}

#[test]
fn a_file_stands_in_for_the_braces_of_the_tuple_it_holds() {
    let braceless = document("accent := \"#FE8019\"\n\neditor = {theme = accent}\nterminal = 1");
    let braced = document("accent := \"#FE8019\"\n\n{editor = {theme = accent} terminal = 1}");
    assert_eq!(braceless.form, Form::Fields);
    assert_eq!(braced.form, Form::Value);
    assert_eq!(braceless.bindings, braced.bindings);

    let ExprKind::Tuple { bindings, fields } = &braceless.value.kind else {
        panic!("a file of fields is a tuple");
    };
    assert!(
        bindings.is_empty(),
        "the file's bindings stay the document's, as a brace block's stay its own"
    );
    let ExprKind::Tuple {
        fields: braced_fields,
        ..
    } = &braced.value.kind
    else {
        panic!("the braced twin is a tuple");
    };
    let named = |fields: &[nuke_syntax::Field]| -> Vec<String> {
        fields
            .iter()
            .map(|field| field.name.ident.as_str().to_owned())
            .collect()
    };
    assert_eq!(named(fields), named(braced_fields));

    let source = "editor = {theme = 1}\nterminal = 2";
    let span = document(source).value.span;
    assert_eq!(
        &source[span.start..span.end],
        source,
        "the span runs from the first field's name to the last field's value"
    );
}

#[test]
fn a_file_of_fields_raises_the_faults_a_brace_block_raises() {
    assert_eq!(fault("a = 1 2"), ErrorKind::ExpectedFieldName);
    assert_eq!(fault("a = 1 }"), ErrorKind::TrailingInput);
    assert_eq!(fault("a = 1 b"), ErrorKind::ExpectedEquals);
    assert_eq!(fault("a = 1 n := 2"), ErrorKind::MisplacedBinding);
    assert_eq!(fault("a = 1 \"b\" => 2"), ErrorKind::MixedPairOperators);
    assert_eq!(
        fault("a = 1 a = 2"),
        ErrorKind::DuplicateField("a".to_owned())
    );
    assert_eq!(fault("\"a\" => 1"), ErrorKind::TrailingInput);
    assert_eq!(fault("n := 1"), ErrorKind::OnlyBindings);
    assert_eq!(fault(""), ErrorKind::EmptyDocument);
}

#[test]
fn a_bound_name_and_a_field_name_do_not_collide() {
    let ExprKind::Tuple { bindings, fields } = document("{port := 8080 port = port}").value.kind
    else {
        panic!("the document should be a tuple");
    };
    assert_eq!(bindings.len(), 1);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name.ident.as_str(), "port");
}

#[test]
fn the_canonical_form_refuses_the_binder_by_name_where_it_reaches_it() {
    for source in ["{n := 1 a = n}", "{n := 1}", "[1 := 2]", "{\"a\" := 1}"] {
        let error = parse(source)
            .err()
            .unwrap_or_else(|| panic!("{source} should have been rejected"));
        assert_eq!(error.kind(), &ErrorKind::SurfaceBinder, "for {source}");
    }
    for (source, name) in [("[n := 1]", "n"), ("{\"a\" => n := 1}", "n")] {
        let error = parse(source)
            .err()
            .unwrap_or_else(|| panic!("{source} should have been rejected"));
        assert_eq!(
            error.kind(),
            &ErrorKind::IdentAsValue(name.to_owned()),
            "an identifier standing as a value is the earlier fault, for {source}"
        );
    }
}

#[test]
fn a_colon_is_a_token_inside_a_hole_and_nowhere_else() {
    assert_eq!(fault("{x: 1}"), ErrorKind::UnexpectedCharacter(':'));
    assert_eq!(
        parse("{x: 1}").unwrap_err().kind(),
        &ErrorKind::UnexpectedCharacter(':')
    );
    assert_eq!(
        pieces(r#"$"{a}: {b}""#).len(),
        3,
        "a colon in an interpolation's text is text, and opens nothing"
    );
    document(r#"a := 1 $"{a:>4}""#);
}

#[test]
fn a_number_takes_the_point_with_it_so_only_a_spaced_dot_projects() {
    for source in ["[1.b]", "1.b", "1."] {
        assert_eq!(
            fault(source),
            ErrorKind::NumberSpelling("1.".to_owned()),
            "for {source}"
        );
    }
    document("1 . b");
    document("[1 . b]");
    assert_eq!(
        fault("[.5]"),
        ErrorKind::ExpectedValue,
        "a digit-less fraction is no number, and a dot is no value"
    );
}

#[test]
fn a_projection_reads_left_to_right_and_takes_any_value_on_its_left() {
    let ExprKind::Access { operand, field } = document("p := {a = {b = 1}} p.a.b").value.kind
    else {
        panic!("the document should be a projection");
    };
    assert_eq!(field.ident.as_str(), "b");
    let ExprKind::Access { operand, field } = operand.kind else {
        panic!("what a projection stands on may be a projection");
    };
    assert_eq!(field.ident.as_str(), "a");
    assert!(matches!(operand.kind, ExprKind::Reference(_)));

    document("{a = 1}.a");
    document("[1 2].a");
    document("{\"a\" => 1}.a");
}

#[test]
fn parentheses_are_what_tell_the_dot_to_evaluate_a_key_rather_than_read_a_name() {
    let ExprKind::Index { operand, key } = document("m := {\"a\" => 1} m.(\"a\")").value.kind
    else {
        panic!("the document should be a projection by key");
    };
    assert!(matches!(key.kind, ExprKind::String(_)));
    assert!(matches!(operand.kind, ExprKind::Reference(_)));

    let ExprKind::Access { operand, field } =
        document("p := {a = {\"k\" => {b = 1}}} p.a.(\"k\").b")
            .value
            .kind
    else {
        panic!("a chain ends in whichever reader stands last");
    };
    assert_eq!(field.ident.as_str(), "b");
    let ExprKind::Index { operand, key } = operand.kind else {
        panic!("the two readers chain into one another");
    };
    assert!(matches!(key.kind, ExprKind::String(_)));
    assert!(matches!(operand.kind, ExprKind::Access { .. }));

    let ExprKind::Index { key, .. } = document("m := {} k := \"a\" m.(k)").value.kind else {
        panic!("a key is an expression, which is the whole point of the parentheses");
    };
    assert!(matches!(key.kind, ExprKind::Reference(_)));

    document("[1 2].(0)");
    document("m := {} m . ( \"a\" )");
}

#[test]
fn a_key_is_one_value_the_parentheses_close_around() {
    assert_eq!(fault("m := {} m.()"), ErrorKind::ExpectedValue);
    assert_eq!(
        fault("m := {} m.(\"a\" \"b\")"),
        ErrorKind::ExpectedKeyClose
    );
    assert_eq!(fault("m := {} m.(\"a\""), ErrorKind::UnterminatedKey);
    assert_eq!(fault("m := {} m.\"a\""), ErrorKind::ExpectedAccessName);
    assert_eq!(fault("m := {} m.[\"a\"]"), ErrorKind::ExpectedAccessName);
    assert_eq!(fault("(1)"), ErrorKind::ExpectedValue);
}

#[test]
fn a_key_reaches_a_hole_because_a_bracket_is_no_brace() {
    let Piece::Hole { expr, .. } = &pieces(r#"$"{m.("a")}""#)[0] else {
        panic!("the piece should be a hole");
    };
    assert!(matches!(expr.kind, ExprKind::Index { .. }));
}

#[test]
fn a_field_is_named_rather_than_pathed() {
    assert_eq!(fault("{a.b = 1}"), ErrorKind::ExpectedArrow);
    assert_eq!(fault("{x = 1 a.b = 2}"), ErrorKind::ExpectedEquals);
    document("p := {a = 1} {p.a => 1}");
}

#[test]
fn the_canonical_form_refuses_the_dot_by_name_where_it_reaches_it() {
    for source in [
        "{a = 1}.a",
        "[.5]",
        "[1.2.3]",
        "{a = 1 b.c = 2}",
        "{\"a\".b => 1}",
    ] {
        let error = parse(source)
            .err()
            .unwrap_or_else(|| panic!("{source} should have been rejected"));
        assert_eq!(error.kind(), &ErrorKind::SurfaceDot, "for {source}");
    }
    let error = parse("{a.b = 1}").expect_err("a path is not a field name");
    assert_eq!(
        error.kind(),
        &ErrorKind::IdentAsValue("a".to_owned()),
        "an identifier standing as a value is the earlier fault"
    );
}

#[test]
fn a_call_names_a_builtin_with_an_identifier_and_takes_one_operand() {
    let ExprKind::Call { name, operand } = document("@import \"p.nuke\"").value.kind else {
        panic!("the document should be a call");
    };
    assert_eq!(name.ident.as_str(), "import");
    assert!(matches!(operand.kind, ExprKind::String(text) if text == "p.nuke"));

    document("@ import \"p.nuke\"");
    document("@import # here\n\"p.nuke\"");
    document("@import @import \"p.nuke\"");
    document("p := \"p.nuke\" @import p");

    for source in ["@\"p.nuke\"", "@Import \"p.nuke\"", "@1", "@", "[@]"] {
        assert_eq!(
            fault(source),
            ErrorKind::ExpectedBuiltinName,
            "a builtin is named by an identifier, for {source}"
        );
    }
    assert_eq!(
        fault("@:= 1"),
        ErrorKind::ExpectedBindingName,
        "a block dispatches on `ident :=` before it reads a value, so that is the earlier fault"
    );
    assert_eq!(
        fault("@import \"a.nuke\" \"b.nuke\""),
        ErrorKind::TrailingInput,
        "a call takes one operand, because a collection carries no separators"
    );
}

#[test]
fn the_dot_takes_what_a_call_yields_rather_than_what_it_was_given() {
    let ExprKind::Access { operand, field } = document("@import \"p.nuke\".accent").value.kind
    else {
        panic!("the dot should take the call, not the path");
    };
    assert_eq!(field.ident.as_str(), "accent");
    assert!(matches!(operand.kind, ExprKind::Call { .. }));

    document("[@import \"p.nuke\"]");
    document("{a = @import \"p.nuke\"}");
    document("{@import \"p.nuke\" => 1}");
    document("p := @import \"p.nuke\" [p]");
    document("@import \"p.nuke\" . accent");
}

#[test]
fn the_canonical_form_refuses_the_call_by_name_where_it_reaches_it() {
    for source in [
        "@import \"p.nuke\"",
        "[@import \"p.nuke\"]",
        "{a = 1 b@import \"p\" = 2}",
        "{\"a\" @import \"p\"}",
        "1 @import \"p.nuke\"",
    ] {
        let error = parse(source)
            .err()
            .unwrap_or_else(|| panic!("{source} should have been rejected"));
        assert_eq!(error.kind(), &ErrorKind::SurfaceCall, "for {source}");
    }
}

fn pieces(source: &str) -> Vec<Piece> {
    let ExprKind::Interpolation(pieces) = document(source).value.kind else {
        panic!("{source} should be an interpolation");
    };
    pieces
}

fn text(piece: &Piece) -> &str {
    match piece {
        Piece::Text(text) => text,
        Piece::Hole { .. } => panic!("this piece is a hole"),
    }
}

#[test]
fn an_interpolation_alternates_text_and_holes() {
    let parts = pieces(r#"$"a{b}c""#);
    assert_eq!(text(&parts[0]), "a");
    assert!(matches!(&parts[1], Piece::Hole { expr, .. }
        if matches!(&expr.kind, ExprKind::Reference(name) if name.as_str() == "b")));
    assert_eq!(text(&parts[2]), "c");

    assert!(
        pieces(r#"$"""#).is_empty(),
        "no pieces make the empty string"
    );
    assert_eq!(pieces(r#"$"only text""#).len(), 1);
    assert_eq!(
        pieces(r#"$"{a}{b}""#).len(),
        2,
        "two holes touch with no text between them"
    );
    assert!(
        matches!(pieces(r#"$"{p.a}""#).first(), Some(Piece::Hole { expr, .. })
            if matches!(expr.kind, ExprKind::Access { .. })),
        "a hole takes a value, so a projection needs no parentheses it has not got"
    );
}

#[test]
fn a_doubled_brace_is_one_brace_and_a_lone_one_is_a_fault() {
    assert_eq!(text(&pieces(r#"$"{{}}""#)[0]), "{}");
    assert_eq!(text(&pieces(r#"$"a{{b}}c""#)[0]), "a{b}c");
    assert_eq!(
        text(&pieces(r#"$"body {{ font-size: 12px; }}""#)[0]),
        "body { font-size: 12px; }"
    );
    assert_eq!(fault(r#"$"a}b""#), ErrorKind::UnmatchedBrace);
    assert_eq!(
        fault(r#"$"\{""#),
        ErrorKind::UnknownEscape('{'),
        "the escape set belongs to the shared token layer, so this one is not widened"
    );
    assert!(
        matches!(document(r#""a{b} }""#).value.kind, ExprKind::String(text) if text == "a{b} }"),
        "a plain string keeps every brace it holds, which is what the prefix buys"
    );
}

#[test]
fn a_hole_holds_one_value_and_is_a_level_of_the_expression() {
    assert_eq!(fault(r#"$"{}""#), ErrorKind::ExpectedValue);
    assert_eq!(fault(r#"$"{a b}""#), ErrorKind::ExpectedHoleClose);
    assert_eq!(fault(r#"$"{a := 1}""#), ErrorKind::MisplacedBinding);
    assert_eq!(fault(r#"$"{a"#), ErrorKind::UnterminatedHole);
    assert_eq!(fault(r#"$"a"#), ErrorKind::UnterminatedString);
    assert_eq!(fault(r#"$"{a}"#), ErrorKind::UnterminatedString);
    assert_eq!(
        fault(r#"$"{a""#),
        ErrorKind::UnterminatedString,
        "a hole may hold a string, so a quote inside one opens rather than closes"
    );
    assert_eq!(fault("$"), ErrorKind::UnexpectedCharacter('$'));
    assert_eq!(fault(r#"$ "a""#), ErrorKind::UnexpectedCharacter('$'));

    document(r#"$"{ {a = 1}.a }""#);
    document(r#"$"{$"{a}"}""#);
    document(r#"$"{@import "./p.nuke".accent}""#);
    assert_eq!(
        fault(&format!(r#"$"{{{}}}""#, "[".repeat(200))),
        ErrorKind::TooDeep,
        "a hole nests the tree the evaluator walks, so it spends the depth budget"
    );
}

#[test]
fn the_canonical_form_refuses_an_interpolation_by_name_where_it_reaches_it() {
    for source in [r#"$"a""#, r#"[$"a"]"#, r#"{a = $"b"}"#, r#"{$"a" => 1}"#] {
        let error = parse(source)
            .err()
            .unwrap_or_else(|| panic!("{source} should have been rejected"));
        assert_eq!(
            error.kind(),
            &ErrorKind::SurfaceInterpolation,
            "for {source}"
        );
    }
    assert!(
        parse(r#""a{b}""#).is_ok(),
        "the canonical form reads the braces as the text they are"
    );
}

fn spec(source: &str) -> Spec {
    match pieces(source).pop() {
        Some(Piece::Hole {
            spec: Some(spec), ..
        }) => spec,
        _ => panic!("{source} should end in a hole carrying a specifier"),
    }
}

#[test]
fn a_specifier_is_read_the_way_rust_reads_one() {
    assert_eq!(
        spec(r#"$"{a:6}""#),
        Spec {
            width: Some(6),
            ..Spec::default()
        }
    );
    assert_eq!(
        spec(r#"$"{a:*^6}""#),
        Spec {
            fill: Some('*'),
            align: Some(Align::Centre),
            width: Some(6),
            ..Spec::default()
        },
        "a fill is a fill only when an alignment follows it"
    );
    assert_eq!(
        spec(r#"$"{a:>6}""#),
        Spec {
            align: Some(Align::Right),
            width: Some(6),
            ..Spec::default()
        },
        "and `>` alone is the alignment rather than a fill with nothing to align"
    );
    assert_eq!(
        spec(r#"$"{a: >6}""#).fill,
        Some(' '),
        "a space is a fill, which is why no whitespace is allowed behind the colon"
    );
    assert_eq!(
        spec(r#"$"{a:#010X}""#),
        Spec {
            prefixed: true,
            zeroed: true,
            width: Some(10),
            notation: Some(Notation::UpperHex),
            ..Spec::default()
        }
    );
    assert_eq!(
        spec(r#"$"{a:+.3e}""#),
        Spec {
            sign: true,
            precision: Some(3),
            notation: Some(Notation::Exponent),
            ..Spec::default()
        }
    );
    assert!(
        matches!(
            pieces(r#"$"{a}""#).pop(),
            Some(Piece::Hole { spec: None, .. })
        ),
        "a hole that asks for nothing carries no specifier at all"
    );
}

#[test]
fn a_specifier_is_refused_where_it_is_not_one() {
    for source in [
        r#"$"{a:}""#,
        r#"$"{a:z}""#,
        r#"$"{a:.}""#,
        r#"$"{a:>>>}""#,
        r#"$"{a:{b}}""#,
        r#"$"{a:>8 }""#,
    ] {
        assert_eq!(fault(source), ErrorKind::MalformedSpec, "for {source}");
    }
    assert_eq!(
        fault(r#"$"{a:007}""#),
        ErrorKind::MalformedSpec,
        "a width is a `uint`, so a leading zero is refused for the reason `[01]` is"
    );
    document(r#"$"{a :6}""#);
}

#[test]
fn a_marker_says_how_many_bits_each_digit_after_it_carries() {
    for (source, value) in [
        ("0b1010", "10"),
        ("0q3201", "225"),
        ("0o755", "493"),
        ("0xFE8019", "16678937"),
    ] {
        assert_eq!(integer(source), value, "for {source}");
    }
}

#[test]
fn a_marker_stands_anywhere_so_one_literal_carries_more_than_one_base() {
    assert_eq!(
        integer("0b101110100xC"),
        "5964",
        "nine bits of binary and then four of hex"
    );
    assert_eq!(
        integer("0xF35b1"),
        "7787",
        "a hex digit is uppercase, so `b` marks a base rather than being one"
    );
    assert_eq!(
        integer("0xDEADxBEEF"),
        integer("0xDEADBEEF"),
        "a marker repeated is the digit separator there is no other spelling for"
    );
}

#[test]
fn a_dyadic_literal_is_a_spelling_and_what_it_spells_is_a_decimal_integer() {
    assert_eq!(
        integer("0x00FF"),
        "255",
        "a leading zero carries bits and no value"
    );
    assert_eq!(
        integer("-0xFF"),
        "-255",
        "the sign belongs to the number and not to the pattern"
    );
    assert_eq!(
        integer("-0b0"),
        "0",
        "and `-0` settles as `0` however it is spelled"
    );
}

#[test]
fn a_marker_is_what_makes_a_literal_dyadic_so_a_decimal_number_is_untouched() {
    assert_eq!(integer("0"), "0", "`0` alone is the decimal number");
    assert_eq!(integer("42"), "42");
    assert!(
        matches!(document("0e5").value.kind, ExprKind::Float(_)),
        "`e` is no marker, so `0e5` is the float it always was"
    );
    assert_eq!(
        fault("[01]"),
        ErrorKind::NumberSpelling("01".to_owned()),
        "and a leading zero is still refused where no marker follows it"
    );
}

#[test]
fn a_hex_digit_is_uppercase_and_every_other_digit_fits_the_base_that_marks_it() {
    for source in ["0xff", "0b2", "0q4", "0o8", "0xG", "0x"] {
        assert_eq!(
            fault(source),
            ErrorKind::NumberSpelling(source.to_owned()),
            "for {source}"
        );
    }
}

#[test]
fn a_dyadic_literal_takes_the_point_with_it_the_way_a_decimal_one_does() {
    assert_eq!(
        fault("[0x1.8]"),
        ErrorKind::NumberSpelling("0x1.8".to_owned()),
        "there is no hex float, and the token takes the point anyway"
    );
    assert_eq!(
        fault("[0xFF.b]"),
        ErrorKind::NumberSpelling("0xFF.".to_owned()),
        "so only a spaced dot projects, as with `1.b`"
    );
    document("[0xFF . b]");
}

#[test]
fn a_literal_past_128_bits_is_refused_because_every_base_but_ten_is_arithmetic() {
    let widest = format!("0x{}", "F".repeat(32));
    assert_eq!(
        integer(&widest),
        u128::MAX.to_string(),
        "128 bits is what the arithmetic holds"
    );
    assert_eq!(
        fault(&format!("0x{}", "F".repeat(33))),
        ErrorKind::DyadicTooWide,
        "and one bit more is past it"
    );
    assert_eq!(
        integer(&format!("0x{}FF", "0".repeat(64))),
        "255",
        "a leading zero is no width, the ceiling being on the value"
    );
    assert_eq!(
        integer(&"9".repeat(40)),
        "9".repeat(40),
        "while a decimal integer stays arbitrary width, being text and not arithmetic"
    );
}

#[test]
fn the_canonical_form_refuses_a_dyadic_literal_by_name_where_it_reaches_it() {
    for source in ["0xFF", "[0b1010]", "{a = 0o755}", "-0q321"] {
        let error = parse(source)
            .err()
            .unwrap_or_else(|| panic!("{source} should have been rejected"));
        assert_eq!(error.kind(), &ErrorKind::SurfaceDyadic, "for {source}");
    }
    let error = parse("[0xff]").expect_err("a misspelling is no language's");
    assert_eq!(
        error.kind(),
        &ErrorKind::NumberSpelling("0xff".to_owned()),
        "a bad spelling is a bad spelling before it is anyone's syntax"
    );
}
