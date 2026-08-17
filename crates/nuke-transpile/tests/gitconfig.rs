use gix_config::File;
use nuke_syntax::{Ident, Tuple, Value, parse};
use nuke_transpile::gitconfig::{Error, ErrorKind, to_string};

fn gitconfig(source: &str) -> String {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect("the value should transpile")
}

fn value(source: &str) -> String {
    let written = gitconfig(&format!("{{s = {{v = {source}}}}}"));
    written
        .strip_prefix("[s]\n\tv = ")
        .expect("a scalar is written as the one variable of one section")
        .to_owned()
}

fn refused(source: &str) -> Error {
    let value = parse(source).expect("the source should parse");
    to_string(&value).expect_err("the value should be refused")
}

#[test]
fn a_document_that_is_not_a_table_is_refused() {
    for (source, form) in [
        ("[1 2]", "list"),
        ("True", "atom"),
        (r#""text""#, "string"),
        ("42", "integer"),
        ("1.5", "float"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableRoot(form),
            "{error}"
        );
        assert_eq!(error.path().to_string(), "the document");
        assert!(error.path().is_root(), "for {source}");
    }
    assert_eq!(gitconfig("{}"), "");
}

#[test]
fn a_variable_outside_a_section_is_refused_because_git_can_name_no_such_key() {
    for (source, name, path) in [
        ("{a = 1}", "a", "a"),
        (r#"{a = "text"}"#, "a", "a"),
        ("{a = [1 2]}", "a", "a"),
        ("{a = {b = 1} c = 2}", "c", "c"),
        (r#"{"x y" => 1}"#, "x y", "#1"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::SectionlessKey(name.to_owned()),
            "{error}"
        );
        assert_eq!(error.path().to_string(), path, "for {source}");
    }
}

#[test]
fn every_variable_stands_under_a_header_and_a_variable_after_a_subsection_reopens_one() {
    assert_eq!(gitconfig("{a = {b = 1}}"), "[a]\n\tb = 1");
    assert_eq!(gitconfig("{a = {b = 1 c = 2}}"), "[a]\n\tb = 1\n\tc = 2");
    assert_eq!(gitconfig("{a = {}}"), "[a]");
    assert_eq!(
        gitconfig("{a = {b = 1} c = {d = 2}}"),
        "[a]\n\tb = 1\n\n[c]\n\td = 2"
    );
    assert_eq!(gitconfig("{a = {s = {c = 2}}}"), "[a \"s\"]\n\tc = 2");
    assert_eq!(gitconfig("{a = {s = {}}}"), "[a \"s\"]");
    assert_eq!(
        gitconfig("{a = {b = 1 s = {c = 2} d = 3}}"),
        "[a]\n\tb = 1\n\n[a \"s\"]\n\tc = 2\n\n[a]\n\td = 3"
    );
    assert_eq!(
        gitconfig("{a = {s = {c = 1} t = {d = 2}}}"),
        "[a \"s\"]\n\tc = 1\n\n[a \"t\"]\n\td = 2"
    );
}

#[test]
fn a_table_below_a_subsection_is_refused_because_git_is_three_levels_deep() {
    let error = refused("{a = {s = {b = {c = 1}}}}");
    assert_eq!(
        error.kind(),
        &ErrorKind::UnrepresentableValue("tuple"),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "a.s.b");

    let error = refused(r#"{a = {s = {b = {"c" => 1}}}}"#);
    assert_eq!(
        error.kind(),
        &ErrorKind::UnrepresentableValue("map"),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "a.s.b");
}

#[test]
fn a_list_is_the_variable_repeated_and_holds_only_scalars() {
    assert_eq!(
        gitconfig(r#"{include = {path = ["one" "two" "three"]}}"#),
        "[include]\n\tpath = one\n\tpath = two\n\tpath = three"
    );
    assert_eq!(gitconfig("{a = {b = [1]}}"), gitconfig("{a = {b = 1}}"));

    let error = refused("{a = {b = []}}");
    assert_eq!(error.kind(), &ErrorKind::EmptyList, "{error}");
    assert_eq!(error.path().to_string(), "a.b");

    for (source, form) in [
        ("{a = {b = [[1]]}}", "list"),
        ("{a = {b = [{c = 1}]}}", "tuple"),
        (r#"{a = {b = [{"c" => 1}]}}"#, "map"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableValue(form),
            "{error}"
        );
        assert_eq!(error.path().to_string(), "a.b[0]", "for {source}");
    }
}

#[test]
fn a_name_is_three_alphabets_and_a_subsection_takes_what_the_other_two_refuse() {
    assert_eq!(gitconfig("{a1 = {b2 = 1}}"), "[a1]\n\tb2 = 1");
    assert_eq!(gitconfig(r#"{"1a" => {b = 1}}"#), "[1a]\n\tb = 1");
    assert_eq!(
        gitconfig(r#"{"diff-highlight" => {"line-numbers" => True}}"#),
        "[diff-highlight]\n\tline-numbers = True"
    );
    assert_eq!(
        gitconfig("{Relative => {Atom99 => 1}}"),
        "[Relative]\n\tAtom99 = 1"
    );

    for (source, name, path) in [
        ("{a_b = {c = 1}}", "a_b", "a_b"),
        ("{a = {b_c = 1}}", "b_c", "a.b_c"),
        (r#"{"a.b" => {c = 1}}"#, "a.b", "#1"),
        (r#"{a = {"b.c" => 1}}"#, "b.c", "a#1"),
        (r#"{"" => {c = 1}}"#, "", "#1"),
        (r#"{a = {"" => 1}}"#, "", "a#1"),
        (r#"{a = {"1b" => 1}}"#, "1b", "a#1"),
        (r#"{"a b" => {c = 1}}"#, "a b", "#1"),
        (r#"{a = {"b c" => 1}}"#, "b c", "a#1"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::UnspellableName(name.to_owned()),
            "{error}"
        );
        assert_eq!(error.path().to_string(), path, "for {source}");
    }

    assert_eq!(
        gitconfig(r#"{a = {"b_c.d e" => {f = 1}}}"#),
        "[a \"b_c.d e\"]\n\tf = 1"
    );
    assert_eq!(gitconfig(r#"{a = {"" => {f = 1}}}"#), "[a \"\"]\n\tf = 1");
    assert_eq!(
        gitconfig(r#"{a = {"he \"q\" c:\\t" => {f = 1}}}"#),
        "[a \"he \\\"q\\\" c:\\\\t\"]\n\tf = 1"
    );
}

#[test]
fn a_key_git_cannot_name_at_all_is_refused() {
    for (source, form) in [
        ("{42 => {a = 1}}", "integer"),
        ("{-1.5 => {a = 1}}", "float"),
        ("{[1 2] => {a = 1}}", "list"),
        ("{{x = 1} => {a = 1}}", "tuple"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableKey(form),
            "{error}"
        );
        assert_eq!(error.path().to_string(), "#1", "for {source}");
    }
}

#[test]
fn two_names_git_folds_into_one_are_refused_and_the_ones_it_keeps_apart_are_not() {
    for (source, name, path) in [
        (r#"{Core => {a = 1} "core" => {b = 2}}"#, "core", "#2"),
        (r#"{a = {Theme => 1 "theme" => 2}}"#, "theme", "a#2"),
        (r#"{a = {s = {Theme => 1 "theme" => 2}}}"#, "theme", "a.s#2"),
    ] {
        let error = refused(source);
        assert_eq!(
            error.kind(),
            &ErrorKind::DuplicateKey(name.to_owned()),
            "{error}"
        );
        assert_eq!(error.path().to_string(), path, "for {source}");
    }

    assert_eq!(
        gitconfig(r#"{a = {"S" => {c = 1} "s" => {d = 2}}}"#),
        "[a \"S\"]\n\tc = 1\n\n[a \"s\"]\n\td = 2"
    );
    assert_eq!(
        gitconfig(r#"{a = {"x" => 1 "X" => {c = 2}}}"#),
        "[a]\n\tx = 1\n\n[a \"X\"]\n\tc = 2"
    );
}

#[test]
fn every_scalar_goes_out_as_its_own_text_and_an_atom_keeps_its_spelling() {
    assert_eq!(value("True"), "True");
    assert_eq!(value("False"), "False");
    assert_eq!(value("Null"), "Null");
    assert_eq!(value("Relative"), "Relative");
    assert_eq!(value(r#""text""#), "text");
    assert_eq!(value("0"), "0");
    assert_eq!(value("-0"), "0");
    assert_eq!(value("-7"), "-7");
    assert_eq!(
        value("170141183460469231731687303715884105728"),
        "170141183460469231731687303715884105728"
    );
    assert_eq!(value("1.5"), "1.5");
    assert_eq!(value("-0.0"), "-0.0");
    assert_eq!(value("1e5"), "100000.0");
    assert_eq!(value("1e300"), "1e300");

    assert_eq!(value("42"), value(r#""42""#));
    assert_eq!(value("True"), value(r#""True""#));
    assert_eq!(value("1.0"), value(r#""1.0""#));
}

#[test]
fn a_value_is_bare_where_the_bare_form_gives_it_back_and_quoted_where_it_does_not() {
    for bare in [
        "hx",
        "-7",
        "50%",
        "a=b",
        "a:b",
        "a[b]",
        "~/.gitignore",
        "é✓",
    ] {
        assert_eq!(value(&format!(r#""{bare}""#)), bare, "for {bare}");
    }
    for (source, quoted) in [
        (r#""""#, r#""""#),
        (r#""a b""#, r#""a b""#),
        (r#"" x""#, r#"" x""#),
        (r#""x ""#, r#""x ""#),
        (r##""#ffffff""##, r##""#ffffff""##),
        (r#""a;b""#, r#""a;b""#),
        (r#""a\"b""#, r#""a\"b""#),
        (r#""c:\\tmp""#, r#""c:\\tmp""#),
        (r#""a\nb""#, r#""a\nb""#),
        (r#""a\tb""#, r#""a\tb""#),
        (r#""\u{A0}""#, "\"\u{A0}\""),
    ] {
        assert_eq!(value(source), quoted, "for {source}");
    }
}

#[test]
fn a_character_git_has_no_escape_for_is_refused_by_the_character_itself() {
    for (source, character) in [
        (r#""\u{0}""#, '\u{0}'),
        (r#""\r""#, '\r'),
        (r#""\u{8}""#, '\u{8}'),
        (r#""\u{1}""#, '\u{1}'),
        (r#""\u{B}""#, '\u{B}'),
        (r#""\u{C}""#, '\u{C}'),
        (r#""\u{1F}""#, '\u{1F}'),
    ] {
        let error = refused(&format!("{{s = {{v = {source}}}}}"));
        assert_eq!(
            error.kind(),
            &ErrorKind::UnrepresentableCharacter(character),
            "{error}"
        );
        assert_eq!(error.path().to_string(), "s.v", "for {source}");
    }

    for crosses in [
        r#""\u{7F}""#,
        r#""\u{85}""#,
        r#""\u{2028}""#,
        r#""\u{2029}""#,
        r#""\u{FEFF}""#,
        r#""\u{10FFFF}""#,
    ] {
        value(crosses);
    }

    for source in [r#""a\nb""#, r#""a\tb""#, r#""a\u{0}b""#, r#""a\rb""#] {
        let written = to_string(&parse(&format!("{{a = {{{source} => {{f = 1}}}}}}")).unwrap());
        assert!(
            matches!(
                written.as_ref().err().map(Error::kind),
                Some(ErrorKind::UnrepresentableCharacter(_))
            ),
            "{source} should not name a subsection: {written:?}"
        );
    }
}

#[test]
fn the_backspace_git_documents_is_read_two_ways_so_the_character_it_spells_is_refused() {
    let file = File::try_from("[a]\n\tv = \"X\\bY\"").expect("the oracle should parse it");
    let read: Vec<String> = file
        .sections()
        .flat_map(|section| {
            section
                .body()
                .into_iter()
                .map(|(_, text)| text.to_string())
                .collect::<Vec<String>>()
        })
        .collect();
    assert_eq!(
        read,
        vec!["Y".to_owned()],
        "gix-config applies the backspace and drops the character before it, where git 2.54 writes U+0008 itself, which is why the backend refuses the character rather than the escape"
    );
}

#[test]
fn a_dot_file_cannot_be_a_git_config_file_because_a_field_name_is_not_a_variable_name() {
    let source = include_str!("../../../fixtures/valid/dotfile.nuke");
    let error = refused(source);
    assert_eq!(
        error.kind(),
        &ErrorKind::UnspellableName("tab_width".to_owned()),
        "{error}"
    );
    assert_eq!(error.path().to_string(), "editor.tab_width");
}

#[test]
fn a_git_config_file_lays_out_the_way_a_hand_written_one_would() {
    let source = r#"
{
  user = {name = "Vahan" email = "vahan@example.com"}

  core = {
    editor = "hx"
    excludesfile = "~/.gitignore"
    pager = "delta --dark"
  }

  remote = {
    "origin" => {
      url = "git@example.com:me/repo.git"
      fetch = "+refs/heads/*:refs/remotes/origin/*"
    }
  }

  include = {path = ["~/.gitconfig.local" "~/.gitconfig.work"]}

  color = {ui = True diff = {meta = "yellow bold"} branch = Auto}
}
"#;
    assert_eq!(
        gitconfig(source),
        "\
[user]
\tname = Vahan
\temail = vahan@example.com

[core]
\teditor = hx
\texcludesfile = ~/.gitignore
\tpager = \"delta --dark\"

[remote \"origin\"]
\turl = git@example.com:me/repo.git
\tfetch = +refs/heads/*:refs/remotes/origin/*

[include]
\tpath = ~/.gitconfig.local
\tpath = ~/.gitconfig.work

[color]
\tui = True

[color \"diff\"]
\tmeta = \"yellow bold\"

[color]
\tbranch = Auto"
    );
}

#[test]
fn what_the_backend_writes_is_the_document_a_real_git_config_parser_reads_back() {
    for source in [
        "{}",
        "{a = {}}",
        "{a = {b = 1}}",
        "{a = {b = 1 c = 2} d = {e = 3}}",
        "{a = {b = 1 s = {c = 2} d = 3}}",
        "{a = {s = {c = 1} t = {d = 2}}}",
        r#"{remote = {"origin" => {url = "git@example.com:me/repo.git"}}}"#,
        r#"{include = {path = ["one" "two" "three"]}}"#,
        r#"{a = {v = ""}}"#,
        r#"{a = {v = " pad " w = "a b"}}"#,
        r##"{a = {v = "#ffffff" w = "x ; y" x = "50%" y = "a=b"}}"##,
        r#"{a = {v = "a\"b" w = "c:\\tmp"}}"#,
        r#"{a = {v = "one\ntwo" w = "a\tb"}}"#,
        r#"{a = {"sub with spaces" => {v = 1}}}"#,
        r#"{a = {"he \"q\" and c:\\t" => {v = 1}}}"#,
        r#"{a = {"" => {v = 1}}}"#,
        "{a = {v = True w = False x = Null y = Relative}}",
        "{a = {n = -0.0 m = 1e300 k = 9007199254740993 w = 170141183460469231731687303715884105728}}",
        r#"{a = {v = "é ✓ 😀" w = "\u{7F}\u{85}\u{2028}\u{FEFF}"}}"#,
        r#"{Core => {ABBREV => 12}}"#,
    ] {
        let value = parse(source).expect("the source should parse");
        let written = to_string(&value).expect("the value should transpile");
        reads(&value, &written, source);
    }
}

#[test]
fn a_value_nested_past_what_git_config_holds_stops_at_the_shape_rather_than_the_depth() {
    let mut deep = Value::List(Vec::new());
    for _ in 1..nuke_syntax::MAX_DEPTH + 2 {
        deep = Value::List(vec![deep]);
    }
    let error = to_string(&Value::Tuple(named("a", Value::Tuple(named("b", deep)))))
        .expect_err("the writer should refuse it");
    assert_eq!(error.kind(), &ErrorKind::UnrepresentableValue("list"));
    assert_eq!(
        error.path().to_string(),
        "a.b[0]",
        "git config is three levels deep, so no depth limit can be reached"
    );
}

fn named(name: &str, value: Value) -> Tuple {
    let mut tuple = Tuple::new();
    tuple.insert(
        Ident::parse(name).expect("the name is an identifier"),
        value,
    );
    tuple
}

fn reads(value: &Value, written: &str, source: &str) {
    let file = File::try_from(written).unwrap_or_else(|error| {
        panic!("{source} wrote a git config file that does not parse: {error}\n{written}")
    });
    let mut read = Vec::new();
    for section in file.sections() {
        let header = section.header();
        let name = header.name().to_string().to_ascii_lowercase();
        let subsection = header.subsection_name().map(|sub| sub.to_string());
        for (variable, text) in section.body() {
            read.push((
                name.clone(),
                subsection.clone(),
                variable.to_ascii_lowercase(),
                text.to_string(),
            ));
        }
    }
    let want = spread(value);
    assert_eq!(read.len(), want.len(), "{source} lost a line\n{written}");
    for ((section, subsection, variable, text), (at, item)) in read.iter().zip(&want) {
        let (wanted_section, wanted_subsection, wanted_variable) = at;
        assert_eq!(section, wanted_section, "{source} renamed a section");
        assert_eq!(
            subsection, wanted_subsection,
            "{source} renamed a subsection"
        );
        assert_eq!(variable, wanted_variable, "{source} renamed a variable");
        means(item, text, &format!("{source}: {section}.{variable}"));
    }
}

type Line<'a> = ((String, Option<String>, String), &'a Value);

fn spread(value: &Value) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    for (section, body) in table(value) {
        for (name, item) in table(body) {
            if matches!(item, Value::Tuple(_) | Value::Map(_)) {
                for (variable, scalar) in table(item) {
                    push(&mut lines, &section, Some(&name), &variable, scalar);
                }
            } else {
                push(&mut lines, &section, None, &name, item);
            }
        }
    }
    lines
}

fn push<'a>(
    lines: &mut Vec<Line<'a>>,
    section: &str,
    subsection: Option<&str>,
    variable: &str,
    value: &'a Value,
) {
    let at = (
        section.to_ascii_lowercase(),
        subsection.map(str::to_owned),
        variable.to_ascii_lowercase(),
    );
    match value {
        Value::List(items) => lines.extend(items.iter().map(|item| (at.clone(), item))),
        scalar => lines.push((at, scalar)),
    }
}

fn table(value: &Value) -> Vec<(String, &Value)> {
    match value {
        Value::Tuple(tuple) => tuple
            .iter()
            .map(|(name, item)| (name.as_str().to_owned(), item))
            .collect(),
        Value::Map(map) => map.iter().map(|(key, item)| (key_of(key), item)).collect(),
        other => panic!("{other:?} is not a table"),
    }
}

fn key_of(key: &Value) -> String {
    match key {
        Value::String(text) => text.clone(),
        Value::Atom(atom) => atom.as_str().to_owned(),
        other => panic!("git cannot name {other:?}"),
    }
}

fn means(value: &Value, text: &str, at: &str) {
    match value {
        Value::Atom(atom) => assert_eq!(text, atom.as_str(), "at {at}"),
        Value::String(item) => assert_eq!(text, item, "at {at}"),
        Value::Integer(integer) => assert_eq!(text, integer.as_str(), "at {at}"),
        Value::Float(number) => assert_eq!(
            text.parse::<f64>().map(f64::to_bits),
            Ok(number.get().to_bits()),
            "at {at}"
        ),
        other => panic!("{at} is not a scalar: {other:?}"),
    }
}
