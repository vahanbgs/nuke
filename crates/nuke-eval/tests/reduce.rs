use nuke_eval::{Error, ErrorKind, eval};
use nuke_syntax::{Value, surface};

fn reduced(source: &str) -> Value {
    eval(source).unwrap_or_else(|error| panic!("{source} should reduce, but: {error}"))
}

fn refused(source: &str) -> Error {
    eval(source).expect_err("the document should be refused")
}

#[test]
fn a_binding_contributes_nothing_and_a_block_of_bindings_is_empty() {
    assert_eq!(reduced("{n := 1}"), reduced("{}"));
    assert_eq!(reduced("{n := 1 a = n}"), reduced("{a = 1}"));
    assert_eq!(reduced("n := 1 [n n]"), reduced("[1 1]"));
    assert_eq!(
        reduced("{port := 8080 port = port}"),
        reduced("{port = 8080}")
    );
}

#[test]
fn an_inner_block_shadows_a_name_and_a_binding_reads_the_one_above() {
    assert_eq!(
        reduced("n := 1 {a = n b = {n := 2 c = n} d = n}"),
        reduced("{a = 1 b = {c = 2} d = 1}")
    );
    assert_eq!(
        reduced("n := 1 {a = {n := n b = n}}"),
        reduced("{a = {b = 1}}")
    );
}

#[test]
fn a_reference_cycle_cannot_be_written() {
    for source in ["n := n [n]", "a := b b := a [a]", "{n := n a = n}"] {
        assert!(
            matches!(refused(source).kind(), ErrorKind::Unbound(_)),
            "for {source}"
        );
    }
}

#[test]
fn a_fault_in_a_binding_nothing_reads_is_still_a_fault() {
    assert!(matches!(
        refused("unused := missing [1]").kind(),
        ErrorKind::Unbound(_)
    ));
}

#[test]
fn a_repeated_key_is_found_only_after_reduction() {
    let source = "{n := 1 n => \"a\" 1 => \"b\"}";
    surface::parse(source).expect("no parser can see this collision");
    assert_eq!(refused(source).kind(), &ErrorKind::DuplicateKey);
}

#[test]
fn a_document_that_doubles_a_value_each_line_stops_at_the_budget() {
    let mut source = String::from("a0 := [1 1]\n");
    for level in 1..30 {
        source.push_str(&format!("a{level} := [a{} a{}]\n", level - 1, level - 1));
    }
    source.push_str("[a29]");
    assert_eq!(refused(&source).kind(), &ErrorKind::TooLarge);
}

#[test]
fn a_binding_used_where_it_would_nest_past_the_parsers_own_limit_is_refused() {
    let source = format!(
        "n := {}{}\n{}n{}",
        "[".repeat(100),
        "]".repeat(100),
        "[".repeat(50),
        "]".repeat(50)
    );
    assert_eq!(refused(&source).kind(), &ErrorKind::TooDeep);
}

#[test]
fn every_fault_carries_the_span_of_what_raised_it() {
    let source = "{\n  a = 1\n  b = missing\n}";
    let error = refused(source);
    assert_eq!(error.location(source).to_string(), "3:7");

    let error = refused("{x: 1}");
    assert!(matches!(error.kind(), ErrorKind::Syntax(_)));
    assert_eq!(error.span(), nuke_syntax::Span::new(2, 3));
}
