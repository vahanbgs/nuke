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
fn a_projection_is_the_field_it_names_and_nothing_is_built_to_make_it() {
    assert_eq!(reduced("p := {a = 1} p.a"), reduced("1"));
    assert_eq!(reduced("{a = 1}.a"), reduced("1"));
    assert_eq!(
        reduced("p := {a = {b = {c = 1}}} p.a.b"),
        reduced("{c = 1}")
    );
    assert_eq!(reduced("p := {a = {b = 1}} p.a.b"), reduced("1"));
    assert_eq!(reduced("p := {a = 1 b = 2} [p.b p.a]"), reduced("[2 1]"));
}

#[test]
fn only_a_tuple_has_fields_and_only_the_ones_it_holds() {
    for source in [
        "m := {\"a\" => 1} m.a",
        "[1 2].a",
        "1 . a",
        "\"text\" . a",
        "True . a",
    ] {
        assert_eq!(
            refused(source).kind(),
            &ErrorKind::NotATuple,
            "for {source}"
        );
    }
    for (source, name) in [
        ("p := {a = 1} p.b", "b"),
        ("{}.a", "a"),
        ("{n := 1}.a", "a"),
    ] {
        assert_eq!(
            refused(source).kind(),
            &ErrorKind::NoSuchField(name.to_owned()),
            "for {source}"
        );
    }
}

#[test]
fn a_projection_costs_what_reaching_its_operand_costs() {
    let mut source = String::from("a0 := [1 1]\n");
    for line in 1..30 {
        source.push_str(&format!("a{line} := [a{} a{}]\n", line - 1, line - 1));
    }
    source.push_str("p := {big = a29}\n[p.big]");
    assert_eq!(
        refused(&source).kind(),
        &ErrorKind::TooLarge,
        "narrowing a value does not make reaching it cheaper"
    );

    let deep = format!("p := {{a = 1}} p{}", ".a".repeat(200));
    assert!(
        matches!(refused(&deep).kind(), ErrorKind::Syntax(_)),
        "a chain long enough to overflow the evaluator is stopped by the parser first"
    );
}

#[test]
fn every_fault_carries_the_span_of_what_raised_it() {
    let source = "{\n  a = 1\n  b = missing\n}";
    let error = refused(source);
    assert_eq!(error.location(source).to_string(), "3:7");

    let error = refused("{x: 1}");
    assert!(matches!(error.kind(), ErrorKind::Syntax(_)));
    assert_eq!(error.span(), nuke_syntax::Span::new(2, 3));

    let source = "p := {a = 1}\np.b";
    let error = refused(source);
    assert_eq!(
        error.span(),
        nuke_syntax::Span::new(15, 16),
        "a missing field is named at the name, not at what was projected"
    );

    let source = "p := [1 2]\np.a";
    let error = refused(source);
    assert_eq!(
        error.span(),
        nuke_syntax::Span::new(11, 12),
        "a value that has no fields is named at itself, not at the field asked for"
    );
}
