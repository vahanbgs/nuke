use nuke_syntax::lexer::{Lexer, TokenKind};
use nuke_syntax::printer::format;
use nuke_syntax::surface;

fn sources() -> Vec<nuke_fixtures::Fixture> {
    let mut all = nuke_fixtures::reductions()
        .into_iter()
        .map(|reduction| reduction.source)
        .collect::<Vec<_>>();
    all.extend(nuke_fixtures::valid());
    all.extend(nuke_fixtures::surface_modules());
    all.retain(|fixture| surface::parse(&fixture.source).is_ok());
    all
}

fn comments(source: &str) -> Vec<String> {
    Lexer::new(source)
        .filter_map(Result::ok)
        .filter(|token| token.kind == TokenKind::Comment)
        .map(|token| token.text.trim_end().to_owned())
        .collect()
}

#[test]
fn formatted_output_parses() {
    for fixture in sources() {
        let formatted = format(&fixture.source)
            .unwrap_or_else(|error| panic!("{}: {error}", fixture.display()));
        surface::parse(&formatted).unwrap_or_else(|error| {
            panic!(
                "{} does not parse after formatting: {error}\n{formatted}",
                fixture.display()
            )
        });
    }
}

#[test]
fn formatting_is_idempotent() {
    for fixture in sources() {
        let once = format(&fixture.source).expect("formats");
        let twice = format(&once).expect("formats again");
        assert_eq!(once, twice, "{} is not stable", fixture.display());
    }
}

#[test]
fn every_comment_survives() {
    for fixture in sources() {
        let formatted = format(&fixture.source).expect("formats");
        assert_eq!(
            comments(&fixture.source),
            comments(&formatted),
            "{} lost or changed a comment",
            fixture.display()
        );
    }
}

const SPARSE: [&str; 2] = ["access-whitespace.nuke", "whitespace.nuke"];

#[test]
fn the_corpus_is_already_formatted() {
    let mut drifted = Vec::new();
    for fixture in sources() {
        if SPARSE.contains(&fixture.name()) {
            continue;
        }
        let formatted = format(&fixture.source).expect("formats");
        if formatted != fixture.source {
            drifted.push(fixture.name().to_owned());
        }
    }
    assert!(
        drifted.is_empty(),
        "fixtures the formatter rewrites: {drifted:?}"
    );
}

#[test]
fn whitespace_a_document_omits_is_put_back() {
    for fixture in sources().iter().filter(|f| SPARSE.contains(&f.name())) {
        let formatted = format(&fixture.source).expect("formats");
        assert_ne!(
            formatted,
            fixture.source,
            "{} demonstrates omitted whitespace, so the formatter must add it",
            fixture.display()
        );
        assert_eq!(
            formatted.split_whitespace().collect::<Vec<_>>().join(" "),
            format(&formatted)
                .expect("formats")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
            "{} is not stable once spaced",
            fixture.display()
        );
    }
}

fn formatted(source: &str) -> String {
    format(source).unwrap_or_else(|error| panic!("{source} should format, but: {error}"))
}

#[test]
fn a_block_written_on_one_line_stays_on_one_line() {
    assert_eq!(formatted("{a = 1 b = 2}"), "{a = 1 b = 2}\n");
    assert_eq!(formatted("[1 2 3]"), "[1 2 3]\n");
    assert_eq!(formatted("{}"), "{}\n");
}

#[test]
fn a_block_the_author_broke_stays_broken_and_indents_by_two() {
    assert_eq!(formatted("{a = 1\nb = 2}"), "{\n  a = 1\n  b = 2\n}\n");
    assert_eq!(
        formatted("{a = {b = 1\nc = 2}}"),
        "{\n  a = {\n    b = 1\n    c = 2\n  }\n}\n"
    );
}

#[test]
fn a_block_too_wide_for_one_line_takes_one_item_per_line() {
    let wide = format!("{{a = \"{}\" b = \"{}\"}}", "x".repeat(60), "y".repeat(60));
    let formatted = formatted(&wide);
    assert_eq!(formatted.lines().count(), 4, "{formatted}");
    assert!(
        formatted.lines().all(|line| line.len() <= 70),
        "{formatted}"
    );
}

#[test]
fn items_the_author_grouped_on_a_line_stay_grouped() {
    assert_eq!(
        formatted("[\n  True False\n\n  1 2\n]"),
        "[\n  True False\n\n  1 2\n]\n"
    );
}

#[test]
fn a_run_of_blank_lines_becomes_one() {
    assert_eq!(
        formatted("{a = 1\n\n\n\nb = 2}"),
        "{\n  a = 1\n\n  b = 2\n}\n"
    );
}

#[test]
fn a_literal_keeps_the_spelling_it_was_written_in() {
    for literal in [
        "0xFE8019", "0b1010", "0q3201", "0o755", "1e-5", "-2.5e-3", "0.0",
    ] {
        assert_eq!(formatted(&format!("[{literal}]")), format!("[{literal}]\n"));
    }
}

#[test]
fn an_interpolation_and_its_specifier_are_left_alone() {
    let source = "accent := 0xFE8019\n\n[$\"#{accent:06X}\" $\"{accent:#x}\"]\n";
    assert_eq!(formatted(source), source);
}

#[test]
fn a_comment_after_the_document_survives() {
    assert_eq!(formatted("{a = 1}\n# trailing"), "{a = 1}\n# trailing\n");
}

#[test]
fn whitespace_around_a_dot_is_closed_up() {
    assert_eq!(
        formatted("p := {a = {b = 1}}\n[p . a . b]"),
        "p := {a = {b = 1}}\n[p.a.b]\n"
    );
}
