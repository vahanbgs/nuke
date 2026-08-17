use nuke_grammar::Grammar;
use nuke_syntax::{ErrorKind, ExprKind, parse, surface};

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
        "[\"\\u{D800}\"]",
        "[1e400]",
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
fn a_colon_that_binds_nothing_is_still_no_token_at_all() {
    assert_eq!(fault("{x: 1}"), ErrorKind::UnexpectedCharacter(':'));
    assert_eq!(
        parse("{x: 1}").unwrap_err().kind(),
        &ErrorKind::UnexpectedCharacter(':')
    );
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
