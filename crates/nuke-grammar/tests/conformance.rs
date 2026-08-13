use nuke_grammar::{Grammar, Rejection, Shape};

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
    for fixture in nuke_fixtures::valid() {
        if let Err(error) = grammar.parse(&fixture.source) {
            panic!("{} should parse, but: {error}", fixture.display());
        }
    }
}

#[test]
fn invalid_fixtures_are_rejected() {
    let grammar = Grammar::canonical().unwrap();
    for fixture in nuke_fixtures::invalid() {
        assert!(
            grammar.parse(&fixture.source).is_err(),
            "{} should have been rejected",
            fixture.display()
        );
    }
}

#[test]
fn tokens_are_matched_greedily() {
    let grammar = Grammar::canonical().unwrap();
    let cases = [
        ("[12]", Shape::List(vec![scalar("number")])),
        ("[1 2]", Shape::List(vec![scalar("number"); 2])),
        ("[10]", Shape::List(vec![scalar("number")])),
        ("[1 0]", Shape::List(vec![scalar("number"); 2])),
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
fn a_misspelled_number_is_one_bad_token() {
    let grammar = Grammar::canonical().unwrap();
    for source in ["[01]", "[00.5]", "[1E5]", "[1e05]", "[1e+5]", "[1.]"] {
        let rejection = grammar
            .parse(source)
            .err()
            .unwrap_or_else(|| panic!("{source} should have been rejected"));
        let Rejection::Number(token) = rejection else {
            panic!("{source} should fail the number check, not the parse: {rejection}");
        };
        assert_eq!(token, source.trim_matches(['[', ']']), "for {source}");
    }
}

#[test]
fn well_spelled_numbers_survive_the_check() {
    let grammar = Grammar::canonical().unwrap();
    for source in [
        "[0]",
        "[-0]",
        "[42]",
        "[0.0]",
        "[-1.5]",
        "[1e5]",
        "[-2.5e-3]",
    ] {
        assert!(grammar.accepts(source), "{source} should be accepted");
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
