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
fn the_surface_language_admits_two_documents_the_canonical_form_refuses_and_no_others() {
    let canonical = Grammar::canonical().unwrap();
    let surface = Grammar::surface().unwrap();
    let admits = ["bare-ident.nuke", "ident-as-map-key.nuke"];
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
    refused(&grammar, &["[1E5]", "[01]", "n := 1e05 [n]", "[1.]"]);
}
