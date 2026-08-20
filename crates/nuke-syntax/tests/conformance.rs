use nuke_grammar::Grammar;
use nuke_syntax::{ErrorKind, parse};

fn spelling(token: &str) -> ErrorKind {
    ErrorKind::NumberSpelling(token.to_owned())
}

fn faults() -> Vec<(&'static str, ErrorKind)> {
    vec![
        ("bare-carriage-return.nuke", ErrorKind::CarriageReturn),
        ("bare-ident.nuke", ErrorKind::IdentAsValue("x".to_owned())),
        (
            "braceless-fields.nuke",
            ErrorKind::IdentAsValue("bg".to_owned()),
        ),
        ("byte-order-mark.nuke", ErrorKind::ByteOrderMark),
        ("chained-dots.nuke", ErrorKind::SurfaceDot),
        (
            "colon-instead-of-equals.nuke",
            ErrorKind::UnexpectedCharacter(':'),
        ),
        ("comma-separator.nuke", ErrorKind::UnexpectedCharacter(',')),
        ("comment-only.nuke", ErrorKind::EmptyDocument),
        (
            "control-character-in-string.nuke",
            ErrorKind::ControlCharacter('\u{1}'),
        ),
        ("dropped-escape.nuke", ErrorKind::UnknownEscape('b')),
        ("empty.nuke", ErrorKind::EmptyDocument),
        ("exponent-leading-zero.nuke", spelling("1e05")),
        ("exponent-plus.nuke", spelling("1e+5")),
        ("hex-literal.nuke", ErrorKind::SurfaceDyadic),
        ("interpolated-string.nuke", ErrorKind::SurfaceInterpolation),
        (
            "ident-as-map-key.nuke",
            ErrorKind::IdentAsValue("a".to_owned()),
        ),
        ("leading-plus.nuke", ErrorKind::UnexpectedCharacter('+')),
        ("list-double-zero.nuke", spelling("00.5")),
        ("list-leading-zero.nuke", spelling("01")),
        ("list-uppercase-exponent.nuke", spelling("1E5")),
        (
            "lowercase-escape-hex.nuke",
            ErrorKind::MalformedUnicodeEscape,
        ),
        ("mixed-pair-operators.nuke", ErrorKind::MixedPairOperators),
        ("solidus-escape.nuke", ErrorKind::UnknownEscape('/')),
        ("trailing-dot.nuke", spelling("1.")),
        ("two-values.nuke", ErrorKind::TrailingInput),
        ("unterminated-list.nuke", ErrorKind::UnterminatedList),
        ("unterminated-string.nuke", ErrorKind::UnterminatedString),
        ("uppercase-escape.nuke", ErrorKind::UnknownEscape('N')),
        ("uppercase-exponent.nuke", spelling("1E5")),
        ("uppercase-field-name.nuke", ErrorKind::ExpectedArrow),
    ]
}

#[test]
fn valid_fixtures_parse() {
    for fixture in nuke_fixtures::valid() {
        if let Err(error) = parse(&fixture.source) {
            panic!(
                "{} should parse, but at {}: {error}",
                fixture.display(),
                error.location(&fixture.source)
            );
        }
    }
}

#[test]
fn every_invalid_fixture_is_rejected_by_the_error_that_names_its_fault() {
    let faults = faults();
    let fixtures = nuke_fixtures::invalid();
    assert_eq!(
        faults.len(),
        fixtures.len(),
        "every invalid fixture needs an expected error"
    );
    for fixture in fixtures {
        let (_, expected) = faults
            .iter()
            .find(|(name, _)| *name == fixture.name())
            .unwrap_or_else(|| panic!("{} has no expected error", fixture.name()));
        let error = parse(&fixture.source)
            .err()
            .unwrap_or_else(|| panic!("{} should have been rejected", fixture.display()));
        assert_eq!(error.kind(), expected, "for {}", fixture.name());
    }
}

#[test]
fn the_parser_and_the_grammar_agree_on_every_fixture() {
    let grammar = Grammar::canonical().expect("the canonical ABNF should translate and compile");
    for fixture in nuke_fixtures::valid()
        .into_iter()
        .chain(nuke_fixtures::invalid())
    {
        assert_eq!(
            parse(&fixture.source).is_ok(),
            grammar.accepts(&fixture.source),
            "the parser and the grammar disagree about {}",
            fixture.display()
        );
    }
}

#[test]
fn the_parser_is_stricter_than_the_grammar_where_the_spec_says_so() {
    let grammar = Grammar::canonical().unwrap();
    for source in [
        "{a = 1 a = 2}",
        "{[1 2] => 1 [1 2] => 2}",
        "[\"\\u{D800}\"]",
        "[1e400]",
    ] {
        assert!(grammar.accepts(source), "the grammar should admit {source}");
        assert!(parse(source).is_err(), "the parser should reject {source}");
    }
}
