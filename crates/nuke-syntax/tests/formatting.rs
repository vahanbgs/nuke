use nuke_syntax::lexer::{Lexer, TokenKind};
use nuke_syntax::printer::format;
use nuke_syntax::surface;

fn sources() -> Vec<nuke_fixtures::Fixture> {
    let mut all: Vec<nuke_fixtures::Fixture> = nuke_fixtures::reductions()
        .into_iter()
        .flat_map(|reduction| [reduction.source, reduction.reduced])
        .collect();
    all.extend(nuke_fixtures::valid());
    all.extend(nuke_fixtures::surface_modules());
    all.extend(nuke_fixtures::surface_refused());
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

const SPARSE: [&str; 3] = [
    "surface/valid/access-whitespace.nuke",
    "surface/valid/whitespace.nuke",
    "valid/whitespace.nuke",
];

fn sparse(fixture: &nuke_fixtures::Fixture) -> bool {
    let path = fixture.path.to_string_lossy().into_owned();
    SPARSE.iter().any(|tail| path.ends_with(tail))
}

#[test]
fn the_corpus_is_already_formatted() {
    let mut drifted = Vec::new();
    for fixture in sources() {
        if sparse(&fixture) {
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
    for fixture in sources().iter().filter(|fixture| sparse(fixture)) {
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
fn a_block_the_author_broke_stays_broken_and_indents_with_one_tab_per_level() {
    assert_eq!(formatted("{a = 1\nb = 2}"), "{\n\ta = 1\n\tb = 2\n}\n");
    assert_eq!(
        formatted("{a = {b = 1\nc = 2}}"),
        "{\n\ta = {\n\t\tb = 1\n\t\tc = 2\n\t}\n}\n"
    );
}

#[test]
fn indentation_is_never_a_space_and_alignment_is_never_a_tab() {
    for fixture in sources() {
        let formatted = formatted(&fixture.source);
        for (at, line) in formatted.lines().enumerate() {
            let lead: String = line.chars().take_while(|c| c.is_whitespace()).collect();
            assert!(
                !lead.contains(' '),
                "{}:{}: indented with a space",
                fixture.display(),
                at + 1
            );
            assert!(
                !line.trim_start().contains('\t'),
                "{}:{}: a tab stands past the indentation",
                fixture.display(),
                at + 1
            );
        }
    }
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
    assert!(formatted.contains("\n\ta = "), "{formatted}");
}

#[test]
fn items_the_author_grouped_on_a_line_stay_grouped() {
    assert_eq!(
        formatted("[\n\tTrue False\n\n\t1 2\n]"),
        "[\n\tTrue False\n\n\t1 2\n]\n"
    );
}

#[test]
fn a_run_of_blank_lines_becomes_one() {
    assert_eq!(
        formatted("{a = 1\n\n\n\nb = 2}"),
        "{\n\ta = 1\n\n\tb = 2\n}\n"
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

#[test]
fn a_dot_after_a_number_keeps_the_room_that_makes_it_an_operator() {
    for source in ["[1 . b]", "[1.5 . b]", "[1 . (0)]"] {
        let formatted = formatted(source);
        surface::parse(&formatted).unwrap_or_else(|error| {
            panic!("{source} became {formatted:?}, which does not parse: {error}")
        });
    }
    assert_eq!(formatted("[1 . b]"), "[1 .b]\n");
}
