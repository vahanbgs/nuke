use nuke_fixtures::Fixture;
use nuke_resolve::Resolution;
use nuke_syntax::{Span, surface};

#[test]
fn the_corpus_resolves_in_source_order() {
    for fixture in every() {
        let resolution = Resolution::of(&surface::parse(&fixture.source).expect("parses"));
        assert!(
            increasing(resolution.bounds().iter().map(|bound| bound.span)),
            "{}: bindings arrive out of order",
            fixture.display()
        );
        assert!(
            increasing(resolution.reads().iter().map(|read| read.span)),
            "{}: reads arrive out of order",
            fixture.display()
        );
    }
}

#[test]
fn a_cursor_on_a_read_answers_what_that_read_named() {
    for fixture in every() {
        let resolution = Resolution::of(&surface::parse(&fixture.source).expect("parses"));
        for read in resolution.reads() {
            assert_eq!(
                resolution.at(read.span.start),
                read.bound,
                "{}: `{}` at {} resolves twice over",
                fixture.display(),
                read.ident.as_str(),
                read.span.location(&fixture.source)
            );
        }
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

fn increasing(spans: impl Iterator<Item = Span>) -> bool {
    let all: Vec<Span> = spans.collect();
    all.windows(2).all(|pair| pair[0].start < pair[1].start)
}
