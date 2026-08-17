use nuke_eval::{ErrorKind, eval};
use nuke_syntax::{Value, surface};

enum Standing {
    Refused,
    Reduced(ErrorKind),
    Reserved(&'static str),
}

fn unbound(name: &str) -> Standing {
    Standing::Reduced(ErrorKind::Unbound(name.to_owned()))
}

fn standing() -> Vec<(&'static str, Standing)> {
    vec![
        ("bare-carriage-return.nuke", Standing::Refused),
        ("bare-ident.nuke", unbound("x")),
        ("byte-order-mark.nuke", Standing::Refused),
        ("chained-dots.nuke", Standing::Refused),
        ("colon-instead-of-equals.nuke", Standing::Refused),
        ("comma-separator.nuke", Standing::Refused),
        ("comment-only.nuke", Standing::Refused),
        ("control-character-in-string.nuke", Standing::Refused),
        ("dropped-escape.nuke", Standing::Refused),
        ("empty.nuke", Standing::Refused),
        ("exponent-leading-zero.nuke", Standing::Refused),
        ("exponent-plus.nuke", Standing::Refused),
        (
            "hex-literal.nuke",
            Standing::Reserved(
                "hex, octal and binary are surface syntax that reduces to a decimal integer",
            ),
        ),
        ("ident-as-map-key.nuke", unbound("a")),
        ("leading-plus.nuke", Standing::Refused),
        ("list-double-zero.nuke", Standing::Refused),
        ("list-leading-zero.nuke", Standing::Refused),
        ("list-uppercase-exponent.nuke", Standing::Refused),
        ("lowercase-escape-hex.nuke", Standing::Refused),
        ("mixed-pair-operators.nuke", Standing::Refused),
        ("solidus-escape.nuke", Standing::Refused),
        ("trailing-dot.nuke", Standing::Refused),
        ("two-values.nuke", Standing::Refused),
        ("unterminated-list.nuke", Standing::Refused),
        ("unterminated-string.nuke", Standing::Refused),
        ("uppercase-escape.nuke", Standing::Refused),
        ("uppercase-exponent.nuke", Standing::Refused),
        ("uppercase-field-name.nuke", Standing::Refused),
    ]
}

fn refusals() -> Vec<(&'static str, ErrorKind)> {
    vec![
        ("access-after-a-number.nuke", ErrorKind::NotATuple),
        ("access-on-a-map.nuke", ErrorKind::NotATuple),
        (
            "field-is-not-a-binding.nuke",
            ErrorKind::Unbound("a".to_owned()),
        ),
        (
            "forward-reference.nuke",
            ErrorKind::Unbound("later".to_owned()),
        ),
        ("no-such-field.nuke", ErrorKind::NoSuchField("b".to_owned())),
        ("out-of-scope-name.nuke", ErrorKind::Unbound("n".to_owned())),
        ("self-reference.nuke", ErrorKind::Unbound("n".to_owned())),
        (
            "unbound-name.nuke",
            ErrorKind::Unbound("missing".to_owned()),
        ),
    ]
}

fn same(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Tuple(one), Value::Tuple(other)) => {
            one.len() == other.len()
                && one
                    .iter()
                    .zip(other.iter())
                    .all(|((name, value), (other_name, other_value))| {
                        name == other_name && same(value, other_value)
                    })
        }
        (Value::Map(one), Value::Map(other)) => {
            one.len() == other.len()
                && one
                    .iter()
                    .zip(other.iter())
                    .all(|((key, value), (other_key, other_value))| {
                        same(key, other_key) && same(value, other_value)
                    })
        }
        (Value::List(one), Value::List(other)) => {
            one.len() == other.len()
                && one
                    .iter()
                    .zip(other.iter())
                    .all(|(value, other_value)| same(value, other_value))
        }
        _ => left == right,
    }
}

fn reduced(source: &str) -> Value {
    eval(source).unwrap_or_else(|error| panic!("{source} should reduce, but: {error}"))
}

#[test]
fn entry_order_survives_reduction_and_value_equality_does_not_see_it() {
    let one = reduced("{1 => 2 3 => 4}");
    let other = reduced("{3 => 4 1 => 2}");
    assert_eq!(
        one, other,
        "map equality is order-independent, because a map can itself be a key"
    );
    assert!(
        !same(&one, &other),
        "so a reduction is checked by a walk that is not, since the spec preserves entry order"
    );
}

#[test]
fn evaluating_a_canonical_document_is_the_identity() {
    for fixture in nuke_fixtures::valid().into_iter().chain(
        nuke_fixtures::reductions()
            .into_iter()
            .map(|pair| pair.reduced),
    ) {
        let parsed = nuke_syntax::parse(&fixture.source)
            .unwrap_or_else(|error| panic!("{} should parse: {error}", fixture.display()));
        let reduced = eval(&fixture.source)
            .unwrap_or_else(|error| panic!("{} should reduce: {error}", fixture.display()));
        assert!(
            same(&reduced, &parsed),
            "evaluating {} changed it",
            fixture.display()
        );
    }
}

#[test]
fn every_surface_fixture_reduces_to_its_canonical_counterpart() {
    for pair in nuke_fixtures::reductions() {
        let expected = nuke_syntax::parse(&pair.reduced.source)
            .unwrap_or_else(|error| panic!("{} should parse: {error}", pair.reduced.display()));
        let reduced = eval(&pair.source.source).unwrap_or_else(|error| {
            panic!(
                "{} should reduce, but at {}: {error}",
                pair.display(),
                error.location(&pair.source.source)
            )
        });
        assert!(same(&reduced, &expected), "for {}", pair.name());
    }
}

#[test]
fn the_surface_dot_file_reduces_to_the_canonical_dot_file() {
    let surface = include_str!("../../../fixtures/surface/valid/dotfile.nuke");
    let canonical = include_str!("../../../fixtures/valid/dotfile.nuke");
    assert!(same(
        &reduced(surface),
        &nuke_syntax::parse(canonical).expect("the dot file should parse")
    ));
}

#[test]
fn every_surface_fixture_is_a_document_the_canonical_form_cannot_spell() {
    for pair in nuke_fixtures::reductions() {
        assert!(
            nuke_syntax::parse(&pair.source.source).is_err(),
            "{} is canonical already, so it tests nothing the identity does not",
            pair.display()
        );
    }
}

#[test]
fn every_fixture_the_reduction_refuses_is_refused_by_the_fault_that_names_it() {
    let refusals = refusals();
    let fixtures = nuke_fixtures::surface_refused();
    assert_eq!(
        refusals.len(),
        fixtures.len(),
        "every refused fixture needs an expected fault"
    );
    for fixture in fixtures {
        let (_, expected) = refusals
            .iter()
            .find(|(name, _)| *name == fixture.name())
            .unwrap_or_else(|| panic!("{} has no expected fault", fixture.name()));
        let error = eval(&fixture.source)
            .err()
            .unwrap_or_else(|| panic!("{} should have been refused", fixture.display()));
        assert_eq!(error.kind(), expected, "for {}", fixture.name());
    }
}

#[test]
fn every_canonically_invalid_fixture_declares_where_it_stands_in_the_surface_language() {
    let standing = standing();
    let fixtures = nuke_fixtures::invalid();
    assert_eq!(
        standing.len(),
        fixtures.len(),
        "every invalid fixture needs a standing in the surface language"
    );
    for fixture in fixtures {
        let (_, expected) = standing
            .iter()
            .find(|(name, _)| *name == fixture.name())
            .unwrap_or_else(|| panic!("{} has no standing", fixture.name()));
        match expected {
            Standing::Refused => assert!(
                surface::parse(&fixture.source).is_err(),
                "{} is a surface document now, so its standing has changed",
                fixture.display()
            ),
            Standing::Reserved(note) => assert!(
                surface::parse(&fixture.source).is_err(),
                "{} is a surface document now, because {note}; \
                 say so here rather than leaving this row to lie",
                fixture.display()
            ),
            Standing::Reduced(kind) => {
                surface::parse(&fixture.source).unwrap_or_else(|error| {
                    panic!(
                        "{} should parse in the surface language: {error}",
                        fixture.display()
                    )
                });
                let error = eval(&fixture.source)
                    .err()
                    .unwrap_or_else(|| panic!("{} should have been refused", fixture.display()));
                assert_eq!(error.kind(), kind, "for {}", fixture.name());
            }
        }
    }
}
