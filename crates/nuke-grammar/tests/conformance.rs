use std::fs;
use std::path::{Path, PathBuf};

use nuke_grammar::{Grammar, Shape};

fn fixtures(kind: &str) -> Vec<(PathBuf, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(kind);
    let mut found: Vec<(PathBuf, String)> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", dir.display()))
        .map(|entry| entry.expect("cannot read directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "nuke")
        })
        .map(|path| {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            (path, source)
        })
        .collect();
    found.sort();
    assert!(!found.is_empty(), "no fixtures found in {}", dir.display());
    found
}

fn scalar(kind: &str) -> Shape {
    Shape::Scalar(kind.to_owned())
}

#[test]
fn the_grammar_translates_to_a_usable_parser() {
    Grammar::canonical().expect("the canonical ABNF should translate and compile");
}

#[test]
fn valid_fixtures_parse_completely() {
    let grammar = Grammar::canonical().unwrap();
    for (path, source) in fixtures("valid") {
        if let Err(error) = grammar.parse(&source) {
            panic!("{} should parse, but: {error}", path.display());
        }
    }
}

#[test]
fn invalid_fixtures_are_rejected() {
    let grammar = Grammar::canonical().unwrap();
    for (path, source) in fixtures("invalid") {
        assert!(
            grammar.parse(&source).is_err(),
            "{} should have been rejected",
            path.display()
        );
    }
}

#[test]
fn tokens_are_matched_greedily() {
    let grammar = Grammar::canonical().unwrap();
    let cases = [
        ("[12]", Shape::List(vec![scalar("number")])),
        ("[1 2]", Shape::List(vec![scalar("number"); 2])),
        ("[01]", Shape::List(vec![scalar("number"); 2])),
        (
            "[[1][2]]",
            Shape::List(vec![Shape::List(vec![scalar("number")]); 2]),
        ),
        ("[TrueFalse]", Shape::List(vec![scalar("atom")])),
        ("[True False]", Shape::List(vec![scalar("atom"); 2])),
    ];
    for (source, expected) in cases {
        let parse = grammar
            .parse(source)
            .unwrap_or_else(|error| panic!("{source} should parse, but: {error}"));
        assert_eq!(parse.shape(), expected, "for {source}");
    }
}

#[test]
fn braces_separate_on_their_pair_operator() {
    let grammar = Grammar::canonical().unwrap();
    let cases = [
        ("{}", Shape::Tuple(vec![])),
        (
            "{x = 1}",
            Shape::Tuple(vec![("x".to_owned(), scalar("number"))]),
        ),
        (
            "{\"a\" => 1}",
            Shape::Map(vec![(scalar("string"), scalar("number"))]),
        ),
        (
            "{{} => 1}",
            Shape::Map(vec![(Shape::Tuple(vec![]), scalar("number"))]),
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
fn field_order_is_preserved() {
    let grammar = Grammar::canonical().unwrap();
    let parse = grammar.parse("{b = 1 a = 2 c = 3}").unwrap();
    let Shape::Tuple(fields) = parse.shape() else {
        panic!("expected a tuple");
    };
    let names: Vec<String> = fields.into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["b", "a", "c"]);
}
