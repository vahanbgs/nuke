use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use nuke_transpile::{Target, toml};
use tempfile::TempDir;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .canonicalize()
        .expect("the fixture tree should exist")
}

fn nuke(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nuke"))
        .args(arguments)
        .output()
        .expect("the binary should run")
}

fn out(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be text")
}

fn err(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be text")
}

#[test]
fn a_named_target_writes_what_the_backend_writes() {
    let file = fixtures().join("surface/valid/dotfile.nuke");
    let output = nuke(&["render", "-f", "toml", file.to_str().unwrap()]);
    assert!(output.status.success(), "{}", err(&output));

    let source = std::fs::read_to_string(&file).expect("the fixture should be readable");
    let value = nuke_eval::eval_at(&source, &file).expect("the fixture should reduce");
    let expected = toml::to_string(&value).expect("the fixture should cross into TOML");
    assert_eq!(out(&output), format!("{expected}\n"));
}

#[test]
fn a_file_name_can_say_the_target_instead() {
    let directory = TempDir::new().expect("a temporary directory");
    let named = directory.path().join("config.toml.nuke");
    std::fs::copy(fixtures().join("surface/valid/dotfile.nuke"), &named)
        .expect("the fixture should copy");

    let inferred = nuke(&["render", named.to_str().unwrap()]);
    let told = nuke(&["render", "-f", "toml", named.to_str().unwrap()]);
    assert!(inferred.status.success(), "{}", err(&inferred));
    assert_eq!(out(&inferred), out(&told));
}

#[test]
fn a_name_that_says_nothing_asks_for_a_format() {
    let file = fixtures().join("surface/valid/dotfile.nuke");
    let output = nuke(&["render", file.to_str().unwrap()]);
    assert!(!output.status.success());
    let message = err(&output);
    assert!(message.contains("--format"), "{message}");
    for target in Target::ALL {
        assert!(message.contains(target.name()), "{message}");
    }
}

#[test]
fn a_target_nothing_writes_is_refused_before_the_file_is_read() {
    let output = nuke(&["render", "-f", "fish", "no-such-file.nuke"]);
    assert!(!output.status.success());
    assert!(err(&output).contains("fish"), "{}", err(&output));
}

#[test]
fn a_fault_names_the_file_and_where_in_it_it_stands() {
    let directory = TempDir::new().expect("a temporary directory");
    let file = directory.path().join("broken.json.nuke");
    std::fs::write(&file, "{editor = {theme = missing}}").expect("the document should be written");

    let output = nuke(&["render", file.to_str().unwrap()]);
    assert!(!output.status.success());
    let message = err(&output);
    assert!(message.contains("1:20"), "{message}");
    assert!(message.contains("broken.json.nuke"), "{message}");
}

#[test]
fn deps_names_the_entry_file_and_every_file_it_read() {
    let file = fixtures().join("surface/valid/a-diamond-of-imports.nuke");
    let output = nuke(&["deps", file.to_str().unwrap()]);
    assert!(output.status.success(), "{}", err(&output));
    let stdout = out(&output);
    let listed: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        listed,
        vec![
            file.to_str().unwrap(),
            fixtures()
                .join("surface/modules/palette.nuke")
                .to_str()
                .unwrap(),
            fixtures()
                .join("surface/modules/theme.nuke")
                .to_str()
                .unwrap(),
        ]
    );
}
