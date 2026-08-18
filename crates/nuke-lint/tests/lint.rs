use nuke_lint::{Diagnostic, Rule, lint};
use nuke_syntax::Span;

#[test]
fn an_atom_is_spelled_in_upper_camel_case() {
    clean("[True False Null Relative Atom99 X]");
    clean("{line_numbers = Relative}");
    trips(
        "[HTTPServer]",
        Rule::AtomCase,
        "HTTPServer",
        Span::new(1, 11),
    );
    trips("[TRUE]", Rule::AtomCase, "TRUE", Span::new(1, 5));
    trips("[FooBAR]", Rule::AtomCase, "FooBAR", Span::new(1, 7));
}

#[test]
fn a_name_is_spelled_in_snake_case() {
    clean("{line_numbers = 1 a1 = 2}");
    trips("{a__b = 1}", Rule::IdentCase, "a__b", Span::new(1, 5));
    trips("{a_ = 1}", Rule::IdentCase, "a_", Span::new(1, 3));
}

#[test]
fn every_name_is_a_name() {
    trips(
        "p__q := {a = 1}\np__q",
        Rule::IdentCase,
        "p__q",
        Span::new(0, 4),
    );
    trips(
        "p := {a__b = 1}\np.a__b",
        Rule::IdentCase,
        "a__b",
        Span::new(6, 10),
    );
    trips(
        "p := 1\n[$\"{p_}\"]",
        Rule::IdentCase,
        "p_",
        Span::new(11, 13),
    );
    let found = lint("p := {a_ = 1}\np.a_").expect("parses");
    assert_eq!(found.len(), 2, "a binding and a projection each name it");
}

#[test]
fn a_hole_is_walked_and_a_string_is_not() {
    clean("[\"a__b\" \"TRUE\" \"./p\"]");
    clean("p := 1\n[$\"a__b #{p}\"]");
}

#[test]
fn an_import_is_asked_for_the_extension() {
    clean("@import \"./palette.nuke\"");
    clean("{a = @concat [\"x\" \"y\"]}");
    trips(
        "@import \"./palette\"",
        Rule::ImportExtension,
        "./palette",
        Span::new(8, 19),
    );
    trips(
        "@import \"./palette.toml\"",
        Rule::ImportExtension,
        "./palette.toml",
        Span::new(8, 24),
    );
}

#[test]
fn a_binding_nothing_reads_is_reported() {
    clean("n := 1\n[n]");
    clean("n := 1\n[$\"{n}\"]");
    clean("p := {a = 1}\n[p.a]");
    clean("m := {\"a\" => 1}\nwhich := \"a\"\n[m.(which)]");
    clean("a := 1\nb := a\n[b]");
    trips("n := 1\n[2]", Rule::UnusedBinding, "n", Span::new(0, 1));
    trips("{n := 1 a = 2}", Rule::UnusedBinding, "n", Span::new(1, 2));
}

#[test]
fn a_binding_reaches_to_the_end_of_its_own_block_and_no_further() {
    clean("{outer := 1 a = {here = outer}}");
    clean("{outer := 1 a = {\"k\" => outer}}");
    trips(
        "{a = {n := 1 x = 1}}",
        Rule::UnusedBinding,
        "n",
        Span::new(6, 7),
    );
    trips(
        "{a = {n := 1} b = n}",
        Rule::UnusedBinding,
        "n",
        Span::new(6, 7),
    );
}

#[test]
fn a_name_a_shadow_covers_before_it_is_read_is_unread() {
    clean("{n := 1 a = {n := n here = n}}");
    clean("{n := 1 above = n a = {n := 2 here = n}}");
    clean("n := n\n[n]");
    let found = lint("{n := 1 a = {n := 2 here = n}}").expect("parses");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].rule(), Rule::UnusedBinding);
    assert_eq!(
        found[0].span(),
        Span::new(1, 2),
        "the shadowed one is unread"
    );
}

#[test]
fn a_name_bound_below_the_only_reference_to_it_is_unread() {
    trips(
        "{here := later later := 1 a = here}",
        Rule::UnusedBinding,
        "later",
        Span::new(15, 20),
    );
}

#[test]
fn an_unread_import_is_still_an_import() {
    let found = lint("p := @import \"./palette\"\n[1]").expect("parses");
    let rules: Vec<Rule> = found.iter().map(Diagnostic::rule).collect();
    assert_eq!(rules, [Rule::UnusedBinding, Rule::ImportExtension]);
}

#[test]
fn findings_come_in_the_order_the_document_spells_them() {
    let found = lint("{a__b = HTTPServer c_ = @import \"./p\"}").expect("parses");
    let rules: Vec<Rule> = found.iter().map(Diagnostic::rule).collect();
    assert_eq!(
        rules,
        [
            Rule::IdentCase,
            Rule::AtomCase,
            Rule::IdentCase,
            Rule::ImportExtension
        ]
    );
    assert!(
        found
            .windows(2)
            .all(|pair| pair[0].span() <= pair[1].span()),
        "{found:?} is not in source order"
    );
}

#[test]
fn a_syntax_error_is_not_a_lint() {
    let error = lint("{a = ,}").expect_err("does not parse");
    assert_eq!(error.span(), Span::new(5, 6));
}

#[test]
fn every_rule_names_itself() {
    let names: Vec<&str> = Rule::ALL.iter().copied().map(Rule::name).collect();
    assert_eq!(
        names,
        [
            "atom-case",
            "ident-case",
            "import-extension",
            "unused-binding"
        ]
    );
    for rule in Rule::ALL {
        assert_eq!(rule.to_string(), rule.name());
    }
}

fn clean(source: &str) {
    let found = lint(source).unwrap_or_else(|error| panic!("{source}: {error}"));
    assert!(found.is_empty(), "{source} reports {found:?}");
}

fn trips(source: &str, rule: Rule, spelling: &str, span: Span) {
    let found = lint(source).unwrap_or_else(|error| panic!("{source}: {error}"));
    let at = found
        .iter()
        .position(|diagnostic| diagnostic.rule() == rule && diagnostic.spelling() == spelling)
        .unwrap_or_else(|| panic!("{source} does not report {rule} for `{spelling}`: {found:?}"));
    assert_eq!(found[at].span(), span, "{source} reports {rule} elsewhere");
}
