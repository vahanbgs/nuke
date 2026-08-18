use nuke_fixtures::Fixture;
use nuke_lint::lint;
use nuke_syntax::surface;

#[test]
fn the_corpus_is_already_in_the_style_it_asks_for() {
    for fixture in every() {
        let found =
            lint(&fixture.source).unwrap_or_else(|error| panic!("{}: {error}", fixture.display()));
        assert!(
            found.is_empty(),
            "{} is a fixture in the wrong style: {}",
            fixture.display(),
            report(&fixture, &found)
        );
    }
}

#[test]
fn a_document_that_does_not_parse_has_no_lints() {
    for fixture in nuke_fixtures::surface_invalid() {
        assert!(
            lint(&fixture.source).is_err(),
            "{} parses, so it is not an invalid fixture",
            fixture.display()
        );
    }
}

fn every() -> Vec<Fixture> {
    let mut all: Vec<Fixture> = nuke_fixtures::reductions()
        .into_iter()
        .flat_map(|reduction| [reduction.source, reduction.reduced])
        .collect();
    all.extend(nuke_fixtures::valid());
    all.extend(nuke_fixtures::surface_modules());
    all.extend(nuke_fixtures::surface_refused());
    all.retain(|fixture| surface::parse(&fixture.source).is_ok());
    all
}

fn report(fixture: &Fixture, found: &[nuke_lint::Diagnostic]) -> String {
    found
        .iter()
        .map(|diagnostic| {
            format!(
                "\n\t{}: {}: {diagnostic}",
                diagnostic.location(&fixture.source),
                diagnostic.rule()
            )
        })
        .collect()
}
