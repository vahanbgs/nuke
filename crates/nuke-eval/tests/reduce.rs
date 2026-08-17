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

fn tree(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a temporary directory");
    for (name, source) in files {
        let path = root.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("a directory for the file");
        }
        std::fs::write(&path, source).expect("a file to write");
    }
    root
}

fn reduced_in(root: &tempfile::TempDir, name: &str) -> Value {
    let path = root.path().join(name);
    let source = std::fs::read_to_string(&path).expect("a file to read");
    nuke_eval::eval_at(&source, &path)
        .unwrap_or_else(|error| panic!("{name} should reduce, but: {error}"))
}

fn innermost(error: Error) -> Error {
    let mut error = error;
    while let ErrorKind::Import { cause, .. } = error.kind().clone() {
        error = *cause;
    }
    error
}

fn refused_in(root: &tempfile::TempDir, name: &str) -> Error {
    let path = root.path().join(name);
    let source = std::fs::read_to_string(&path).expect("a file to read");
    nuke_eval::eval_at(&source, &path).expect_err("the document should be refused")
}

#[test]
fn an_import_is_the_reduced_value_of_the_file_it_names_wherever_a_value_stands() {
    let root = tree(&[
        ("p.nuke", "{accent = \"#FE8019\"}"),
        ("bound.nuke", "p := @import \"./p.nuke\" [p p.accent]"),
        ("field.nuke", "{a = @import \"./p.nuke\"}"),
        ("key.nuke", "{@import \"./p.nuke\" => 1}"),
        ("naked.nuke", "@import \"./p.nuke\".accent"),
        ("bare.nuke", "@import \"p.nuke\""),
    ]);
    assert_eq!(
        reduced_in(&root, "bound.nuke"),
        reduced("[{accent = \"#FE8019\"} \"#FE8019\"]")
    );
    assert_eq!(
        reduced_in(&root, "field.nuke"),
        reduced("{a = {accent = \"#FE8019\"}}")
    );
    assert_eq!(
        reduced_in(&root, "key.nuke"),
        reduced("{{accent = \"#FE8019\"} => 1}")
    );
    assert_eq!(reduced_in(&root, "naked.nuke"), reduced("\"#FE8019\""));
    assert_eq!(
        reduced_in(&root, "bare.nuke"),
        reduced("{accent = \"#FE8019\"}"),
        "a path with no `./` in front of it names the same file"
    );
}

#[test]
fn an_import_resolves_against_the_file_that_spells_it() {
    let root = tree(&[
        ("sub/p.nuke", "{accent = \"#FE8019\"}"),
        ("sub/theme.nuke", "{fg = @import \"./p.nuke\".accent}"),
        ("main.nuke", "@import \"./sub/theme.nuke\""),
        ("up/deep/leaf.nuke", "@import \"../../sub/p.nuke\""),
    ]);
    assert_eq!(
        reduced_in(&root, "main.nuke"),
        reduced("{fg = \"#FE8019\"}"),
        "`./p.nuke` inside sub/theme.nuke is sub/p.nuke, not a sibling of the entry"
    );
    assert_eq!(
        reduced_in(&root, "up/deep/leaf.nuke"),
        reduced("{accent = \"#FE8019\"}"),
        "`..` is an ordinary component with no ceiling"
    );
}

#[test]
fn an_imported_file_hands_over_its_value_and_not_its_names() {
    let root = tree(&[
        ("keeps.nuke", "secret := 1 {a = secret}"),
        ("needs.nuke", "[secret]"),
        ("reads.nuke", "p := @import \"./keeps.nuke\" [p secret]"),
        ("lends.nuke", "secret := 1 @import \"./needs.nuke\""),
    ]);
    assert_eq!(reduced_in(&root, "keeps.nuke"), reduced("{a = 1}"));
    assert!(
        matches!(refused_in(&root, "reads.nuke").kind(), ErrorKind::Unbound(name) if name == "secret"),
        "an importer cannot see the names the file it imports keeps"
    );
    let ErrorKind::Import { cause, .. } = refused_in(&root, "lends.nuke").kind().clone() else {
        panic!("a fault inside an imported file is wrapped in one that names the file");
    };
    assert!(
        matches!(cause.kind(), ErrorKind::Unbound(name) if name == "secret"),
        "and an imported file cannot see the names of the file that imported it"
    );
}

#[test]
fn a_diamond_is_not_a_cycle_and_one_file_imported_twice_is_reuse() {
    let root = tree(&[
        ("p.nuke", "{accent = \"#FE8019\"}"),
        ("left.nuke", "@import \"./p.nuke\""),
        ("right.nuke", "@import \"./p.nuke\""),
        (
            "top.nuke",
            "{a = @import \"./left.nuke\" b = @import \"./right.nuke\"}",
        ),
        (
            "twice.nuke",
            "{a = @import \"./p.nuke\".accent b = @import \"./p.nuke\".accent}",
        ),
    ]);
    assert_eq!(
        reduced_in(&root, "top.nuke"),
        reduced("{a = {accent = \"#FE8019\"} b = {accent = \"#FE8019\"}}")
    );
    assert_eq!(
        reduced_in(&root, "twice.nuke"),
        reduced("{a = \"#FE8019\" b = \"#FE8019\"}")
    );
}

#[test]
fn a_cycle_between_files_is_refused_from_whichever_end_it_is_entered() {
    let root = tree(&[
        ("a.nuke", "@import \"./b.nuke\""),
        ("b.nuke", "@import \"./c.nuke\""),
        ("c.nuke", "@import \"./a.nuke\""),
        ("self.nuke", "@import \"./self.nuke\""),
        ("spelled.nuke", "@import \"././spelled.nuke\""),
    ]);
    for name in ["a.nuke", "b.nuke", "c.nuke"] {
        let error = innermost(refused_in(&root, name));
        assert!(
            matches!(error.kind(), ErrorKind::ImportCycle(_)),
            "entering the loop at {name} should close it, but: {error}"
        );
    }
    assert!(matches!(
        refused_in(&root, "self.nuke").kind(),
        ErrorKind::ImportCycle(_)
    ));
    assert!(
        matches!(
            refused_in(&root, "spelled.nuke").kind(),
            ErrorKind::ImportCycle(_)
        ),
        "a file is identified by where it is, not by how the path was spelled"
    );
}

#[test]
fn a_file_that_cannot_be_read_is_refused_where_the_import_stands() {
    let root = tree(&[
        ("gone.nuke", "{a = @import \"./nowhere.nuke\"}"),
        ("dir.nuke", "{a = @import \"./sub\"}"),
        ("sub/p.nuke", "{}"),
    ]);
    assert_eq!(
        refused_in(&root, "gone.nuke").kind(),
        &ErrorKind::Unreadable {
            path: "./nowhere.nuke".to_owned(),
            cause: std::io::ErrorKind::NotFound,
        },
        "the fault names the path as it was written, which is what an author can act on"
    );
    assert!(
        matches!(
            refused_in(&root, "dir.nuke").kind(),
            ErrorKind::Unreadable { .. }
        ),
        "there is no index.nuke, because that would be a second spelling of one import"
    );
}

#[test]
fn a_fault_inside_an_imported_file_is_located_in_that_file() {
    let root = tree(&[
        ("broken.nuke", "{a = 1\n b = missing}"),
        ("torn.nuke", "{a = ,}"),
        ("needs.nuke", "\n{a = @import \"./broken.nuke\"}"),
        ("reads.nuke", "{a = @import \"./torn.nuke\"}"),
    ]);
    let error = refused_in(&root, "needs.nuke");
    let source = std::fs::read_to_string(root.path().join("needs.nuke")).unwrap();
    assert_eq!(
        error.location(&source).to_string(),
        "2:6",
        "the outer span indexes the file the caller handed in"
    );
    let ErrorKind::Import { path, at, cause } = error.kind() else {
        panic!("a fault below a file boundary is wrapped in one that names the file");
    };
    assert!(path.ends_with("broken.nuke"));
    assert_eq!(
        at.to_string(),
        "2:6",
        "and the location inside the imported file is resolved where its source was in hand"
    );
    assert!(matches!(cause.kind(), ErrorKind::Unbound(name) if name == "missing"));

    let ErrorKind::Import { cause, .. } = refused_in(&root, "reads.nuke").kind().clone() else {
        panic!("a file that is not a document is not importable");
    };
    assert!(matches!(cause.kind(), ErrorKind::Syntax(_)));
}

#[test]
fn a_builtin_is_named_at_reduction_and_import_takes_a_literal() {
    let root = tree(&[
        ("p.nuke", "{}"),
        ("nope.nuke", "{a = @nope \"p.nuke\"}"),
        ("computed.nuke", "n := \"./p.nuke\" {a = @import n}"),
        ("joined.nuke", "{a = @import {b = \"x\"}.b}"),
    ]);
    assert_eq!(
        refused_in(&root, "nope.nuke").kind(),
        &ErrorKind::NoSuchBuiltin("nope".to_owned())
    );
    for name in ["computed.nuke", "joined.nuke"] {
        assert_eq!(
            refused_in(&root, name).kind(),
            &ErrorKind::ExpectedImportPath,
            "for {name}"
        );
    }
}

#[test]
fn a_document_with_no_file_of_its_own_resolves_no_import() {
    assert_eq!(
        refused("@import \"./p.nuke\"").kind(),
        &ErrorKind::NoOrigin,
        "the working directory is the process's, not the document's"
    );
    assert_eq!(
        reduced("{a = 1}"),
        eval("{a = 1}").unwrap(),
        "and a document that imports nothing needs no file"
    );
}

#[test]
fn a_chain_of_files_deeper_than_the_bound_is_refused_rather_than_overflowing() {
    let deep = nuke_eval::MAX_IMPORTS + 2;
    let mut files: Vec<(String, String)> = (0..deep)
        .map(|at| {
            (
                format!("f{at}.nuke"),
                format!("@import \"./f{}.nuke\"", at + 1),
            )
        })
        .collect();
    files.push((format!("f{deep}.nuke"), "{}".to_owned()));
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();
    let root = tree(&borrowed);
    let error = innermost(refused_in(&root, "f0.nuke"));
    assert_eq!(
        error.kind(),
        &ErrorKind::ImportsTooDeep,
        "a chain is finite and acyclic, so only a bound of its own stops it"
    );
}

fn list_of(values: usize) -> String {
    let mut source = String::with_capacity(values * 2 + 2);
    source.push('[');
    for _ in 1..values {
        source.push_str("1 ");
    }
    source.push(']');
    source
}

#[test]
fn a_file_is_read_once_however_often_a_document_imports_it() {
    let size = nuke_eval::MAX_VALUES * 3 / 10;
    let root = tree(&[
        ("m.nuke", &list_of(size)),
        (
            "twice.nuke",
            "{a = @import \"./m.nuke\" b = @import \"./m.nuke\"}",
        ),
    ]);
    reduced_in(&root, "twice.nuke");
}

#[test]
fn an_import_costs_its_size_at_every_site_that_names_it() {
    let size = nuke_eval::MAX_VALUES * 4 / 10;
    let root = tree(&[
        ("m.nuke", &list_of(size)),
        ("once.nuke", "{a = @import \"./m.nuke\"}"),
        (
            "twice.nuke",
            "{a = @import \"./m.nuke\" b = @import \"./m.nuke\"}",
        ),
    ]);
    reduced_in(&root, "once.nuke");
    assert_eq!(
        refused_in(&root, "twice.nuke").kind(),
        &ErrorKind::TooLarge,
        "a file costs building it once, plus its size at every site that names it"
    );
}

#[test]
fn the_budget_is_one_document_wide_however_many_files_it_reads() {
    let size = nuke_eval::MAX_VALUES * 4 / 10;
    let root = tree(&[
        ("one.nuke", &list_of(size)),
        ("other.nuke", &list_of(size)),
        ("alone.nuke", "@import \"./one.nuke\""),
        (
            "both.nuke",
            "{a = @import \"./one.nuke\" b = @import \"./other.nuke\"}",
        ),
    ]);
    reduced_in(&root, "alone.nuke");
    assert_eq!(
        innermost(refused_in(&root, "both.nuke")).kind(),
        &ErrorKind::TooLarge,
        "two files that each fit need not fit together, and the second runs out \
         while it is still being built, so the fault stands inside it"
    );
}

#[test]
fn an_import_nested_past_what_the_canonical_parser_would_read_back_is_refused() {
    let deep = format!(
        "{}1{}",
        "[".repeat(nuke_syntax::MAX_DEPTH - 1),
        "]".repeat(nuke_syntax::MAX_DEPTH - 1)
    );
    let root = tree(&[
        ("deep.nuke", &deep),
        ("flat.nuke", "@import \"./deep.nuke\""),
        ("wrapped.nuke", "[@import \"./deep.nuke\"]"),
    ]);
    reduced_in(&root, "flat.nuke");
    assert_eq!(
        refused_in(&root, "wrapped.nuke").kind(),
        &ErrorKind::TooDeep,
        "an import charges its measured depth at the use site, the way a bound name does"
    );
}

#[test]
fn a_fault_below_a_file_reads_as_one_line_naming_every_file_it_passed_through() {
    let root = tree(&[
        ("a.nuke", "{x = @import \"./b.nuke\"}"),
        ("b.nuke", "[@import \"./c.nuke\"]"),
        ("c.nuke", "{y = missing}"),
    ]);
    let b = root.path().join("b.nuke").canonicalize().unwrap();
    let c = root.path().join("c.nuke").canonicalize().unwrap();
    assert_eq!(
        refused_in(&root, "a.nuke").to_string(),
        format!(
            "importing `{}` failed at 1:2: importing `{}` failed at 1:6: \
             `missing` is not bound here; a name is visible below its own binding, and only \
             inside the block that makes it",
            b.display(),
            c.display()
        )
    );
}
