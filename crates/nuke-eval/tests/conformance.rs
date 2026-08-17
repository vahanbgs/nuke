use std::ffi::OsStr;

use nuke_eval::{Error, ErrorKind, eval, eval_at};
use nuke_fixtures::Fixture;
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

#[derive(Debug)]
enum Fault {
    Kind(ErrorKind),
    Syntax,
    Cycle(&'static str),
    Inside(&'static str, Box<Fault>),
}

fn nothing_binds(name: &str) -> Fault {
    Fault::Kind(ErrorKind::Unbound(name.to_owned()))
}

fn inside(file: &'static str, fault: Fault) -> Fault {
    Fault::Inside(file, Box::new(fault))
}

fn refusals() -> Vec<(&'static str, Fault)> {
    vec![
        (
            "access-after-a-number.nuke",
            Fault::Kind(ErrorKind::NotATuple),
        ),
        ("access-on-a-map.nuke", Fault::Kind(ErrorKind::NotATuple)),
        ("field-is-not-a-binding.nuke", nothing_binds("a")),
        ("forward-reference.nuke", nothing_binds("later")),
        (
            "no-such-field.nuke",
            Fault::Kind(ErrorKind::NoSuchField("b".to_owned())),
        ),
        ("out-of-scope-name.nuke", nothing_binds("n")),
        ("self-reference.nuke", nothing_binds("n")),
        ("unbound-name.nuke", nothing_binds("missing")),
        (
            "a-builtin-nothing-defines.nuke",
            Fault::Kind(ErrorKind::NoSuchBuiltin("nope".to_owned())),
        ),
        (
            "a-cycle-between-two-files.nuke",
            inside(
                "cycles-back.nuke",
                Fault::Cycle("a-cycle-between-two-files.nuke"),
            ),
        ),
        (
            "a-fault-inside-an-imported-file.nuke",
            inside("needs-a-name.nuke", nothing_binds("secret")),
        ),
        (
            "a-field-an-imported-file-has-not-got.nuke",
            Fault::Kind(ErrorKind::NoSuchField("nope".to_owned())),
        ),
        (
            "a-file-that-imports-itself.nuke",
            Fault::Cycle("a-file-that-imports-itself.nuke"),
        ),
        (
            "a-file-that-is-not-a-document.nuke",
            inside("is-not-a-document.nuke", Fault::Syntax),
        ),
        (
            "a-name-an-imported-file-keeps-to-itself.nuke",
            nothing_binds("secret"),
        ),
        (
            "a-name-an-importer-cannot-lend.nuke",
            inside("needs-a-name.nuke", nothing_binds("secret")),
        ),
        (
            "an-import-of-a-file-that-is-not-there.nuke",
            Fault::Kind(ErrorKind::Unreadable {
                path: "./nowhere.nuke".to_owned(),
                cause: std::io::ErrorKind::NotFound,
            }),
        ),
        (
            "an-import-path-that-is-not-a-literal.nuke",
            Fault::Kind(ErrorKind::ExpectedImportPath),
        ),
        (
            "a-concat-of-one-string.nuke",
            Fault::Kind(ErrorKind::NotAList),
        ),
        (
            "a-concat-of-a-number.nuke",
            Fault::Kind(ErrorKind::NotAString),
        ),
        (
            "a-concat-of-an-atom.nuke",
            Fault::Kind(ErrorKind::NotAString),
        ),
    ]
}

fn named(path: &std::path::Path) -> Option<&str> {
    path.file_name().and_then(OsStr::to_str)
}

fn check(expected: &Fault, error: &Error, fixture: &str) {
    match (expected, error.kind()) {
        (Fault::Kind(kind), found) => assert_eq!(found, kind, "for {fixture}"),
        (Fault::Syntax, ErrorKind::Syntax(_)) => {}
        (Fault::Cycle(file), ErrorKind::ImportCycle(path)) => {
            assert_eq!(named(path), Some(*file), "for {fixture}");
        }
        (Fault::Inside(file, inner), ErrorKind::Import { path, cause, .. }) => {
            assert_eq!(named(path), Some(*file), "for {fixture}");
            check(inner, cause, fixture);
        }
        (expected, found) => {
            panic!("{fixture} should be refused by {expected:?}, but was: {found}")
        }
    }
}

fn refused(fixture: &Fixture) -> Error {
    eval_at(&fixture.source, &fixture.path)
        .err()
        .unwrap_or_else(|| panic!("{} should have been refused", fixture.display()))
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
        let reduced = eval_at(&pair.source.source, &pair.source.path).unwrap_or_else(|error| {
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
        check(expected, &refused(&fixture), fixture.name());
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

#[test]
fn every_module_is_a_file_some_other_fixture_imports() {
    let sources: Vec<(String, String)> = nuke_fixtures::surface_refused()
        .into_iter()
        .chain(nuke_fixtures::surface_modules())
        .chain(
            nuke_fixtures::reductions()
                .into_iter()
                .map(|pair| pair.source),
        )
        .map(|fixture| (fixture.name().to_owned(), fixture.source))
        .collect();
    for module in nuke_fixtures::surface_modules() {
        assert!(
            sources
                .iter()
                .any(|(name, source)| name != module.name() && source.contains(module.name())),
            "{} is a module nothing imports; a module is an input to a fixture rather than one",
            module.display()
        );
    }
}
