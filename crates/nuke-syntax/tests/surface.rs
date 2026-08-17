use nuke_grammar::Grammar;
use nuke_syntax::{ErrorKind, ExprKind, parse, surface};

fn faults() -> Vec<(&'static str, ErrorKind)> {
    vec![
        (
            "binding-a-name-that-is-not-one.nuke",
            ErrorKind::ExpectedBindingName,
        ),
        ("binding-after-a-field.nuke", ErrorKind::MisplacedBinding),
        ("binding-after-an-entry.nuke", ErrorKind::MisplacedBinding),
        ("binding-as-a-value.nuke", ErrorKind::MisplacedBinding),
        ("binding-in-a-list.nuke", ErrorKind::MisplacedBinding),
        ("binding-without-a-document.nuke", ErrorKind::OnlyBindings),
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
    for source in [
        "{n := 1 n := 2 a = n}",
        "{a = 1 a = 2}",
        "[\"\\u{D800}\"]",
        "[1e400]",
    ] {
        assert!(grammar.accepts(source), "the grammar should admit {source}");
        assert!(
            surface::parse(source).is_err(),
            "the parser should reject {source}"
        );
    }
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
