use nuke_resolve::Resolution;
use nuke_syntax::{Span, surface};

#[test]
fn a_read_finds_the_binding_it_names() {
    let resolution = of("p := 1\n[p]");
    assert_eq!(
        spans(resolution.bounds().iter().map(|bound| bound.span)),
        [Span::new(0, 1)]
    );
    let read = &resolution.reads()[0];
    assert_eq!(read.span, Span::new(8, 9));
    assert_eq!(
        resolution.bound(read.bound.expect("bound")).span,
        Span::new(0, 1)
    );
    assert_eq!(resolution.unread().count(), 0);
}

#[test]
fn a_binding_reads_the_one_above_it_rather_than_itself() {
    let resolution = of("n := 1\n{a = {n := n b = n}}");
    let outer = resolution.at(0).expect("the outer name");
    let inner = resolution.at(13).expect("the inner name");
    assert_ne!(outer, inner);
    assert_eq!(resolution.at(18), Some(outer), "the binding's own value");
    assert_eq!(resolution.at(24), Some(inner), "the field below it");
}

#[test]
fn a_document_level_name_has_nothing_above_it() {
    let resolution = of("n := n\n[n]");
    assert_eq!(resolution.reads()[0].bound, None);
    assert_eq!(resolution.unread().count(), 0);
}

#[test]
fn a_nested_block_shadows_and_what_it_covers_goes_unread() {
    let resolution = of("p := 1\n{a = {p := 2 b = p}}");
    assert_eq!(
        spans(resolution.unread().map(|bound| bound.span)),
        [Span::new(0, 1)]
    );
}

#[test]
fn a_read_of_nothing_is_bound_to_nothing() {
    let resolution = of("[missing]");
    assert!(resolution.bounds().is_empty());
    assert_eq!(resolution.reads()[0].bound, None);
    assert_eq!(resolution.at(1), None);
}

#[test]
fn only_a_reference_reads_a_name() {
    let resolution = of("p := {a = 1}\np.a");
    assert_eq!(
        spans(resolution.reads().iter().map(|read| read.span)),
        [Span::new(13, 14)]
    );
    let resolution = of("p := \"./x.nuke\"\n@import <| p");
    assert_eq!(
        spans(resolution.reads().iter().map(|read| read.span)),
        [Span::new(27, 28)]
    );
}

#[test]
fn a_hole_is_walked() {
    let resolution = of("p := 1\n[$\"a{p}\"]");
    assert_eq!(resolution.reads().len(), 1);
    assert_eq!(resolution.unread().count(), 0);
}

#[test]
fn a_name_under_a_cursor_is_found_from_either_end() {
    let resolution = of("p := 1\n[p]");
    let id = resolution.at(0).expect("the binding's own name");
    assert_eq!(resolution.at(8), Some(id));
    assert_eq!(resolution.at(7), None, "the `[` names nothing");
    assert_eq!(resolution.reads_of(id).count(), 1);
}

#[test]
fn everything_is_in_source_order() {
    let resolution = of("a := 1\nb := {c := a d = c}\n[a b]");
    assert!(increasing(
        resolution.bounds().iter().map(|bound| bound.span)
    ));
    assert!(increasing(resolution.reads().iter().map(|read| read.span)));
    assert_eq!(resolution.bounds().len(), 3);
    assert_eq!(resolution.reads().len(), 4);
}

fn of(source: &str) -> Resolution {
    Resolution::of(&surface::parse(source).expect("parses"))
}

fn spans(spans: impl Iterator<Item = Span>) -> Vec<Span> {
    spans.collect()
}

fn increasing(spans: impl Iterator<Item = Span>) -> bool {
    let all: Vec<Span> = spans.collect();
    all.windows(2).all(|pair| pair[0].start < pair[1].start)
}
