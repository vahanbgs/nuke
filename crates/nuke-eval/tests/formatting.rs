use nuke_eval::eval_at;
use nuke_syntax::printer::format;

#[test]
fn formatting_a_document_does_not_change_what_it_means() {
    for reduction in nuke_fixtures::reductions() {
        let fixture = reduction.source;
        let path = &fixture.path;
        let before = eval_at(&fixture.source, path)
            .unwrap_or_else(|error| panic!("{}: {error}", fixture.display()));
        let formatted = format(&fixture.source)
            .unwrap_or_else(|error| panic!("{}: {error}", fixture.display()));
        let after = eval_at(&formatted, path).unwrap_or_else(|error| {
            panic!(
                "{} does not reduce after formatting: {error}\n{formatted}",
                fixture.display()
            )
        });
        assert_eq!(
            before,
            after,
            "{} reduces differently once formatted",
            fixture.display()
        );
    }
}

#[test]
fn a_canonical_document_survives_formatting() {
    for fixture in nuke_fixtures::valid() {
        let formatted = format(&fixture.source)
            .unwrap_or_else(|error| panic!("{}: {error}", fixture.display()));
        assert_eq!(
            nuke_syntax::parse(&fixture.source).expect("canonical"),
            nuke_syntax::parse(&formatted).unwrap_or_else(|error| panic!(
                "{} left the canonical form: {error}\n{formatted}",
                fixture.display()
            )),
            "{} reduces differently once formatted",
            fixture.display()
        );
    }
}
