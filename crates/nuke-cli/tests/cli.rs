use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use lsp_server::{Message, Notification, Request, RequestId};

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

fn piped(arguments: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nuke"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary should run");
    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(input.as_bytes())
        .expect("the document should be written");
    child.wait_with_output().expect("the binary should finish")
}

#[test]
fn a_document_is_formatted_from_stdin() {
    let output = piped(&["fmt", "-"], "{a:=1 b:=\"x\" c=[a a]}");
    assert!(output.status.success(), "{}", err(&output));
    assert_eq!(out(&output), "{a := 1 b := \"x\" c = [a a]}\n");
}

#[test]
fn formatting_a_file_writes_it_to_stdout_and_leaves_it_alone() {
    let file = fixtures().join("surface/valid/dotfile.nuke");
    let before = std::fs::read_to_string(&file).expect("the fixture should be readable");
    let output = nuke(&["fmt", file.to_str().unwrap()]);
    assert!(output.status.success(), "{}", err(&output));
    assert_eq!(out(&output), before);
    assert_eq!(
        std::fs::read_to_string(&file).expect("the fixture should still be readable"),
        before
    );
}

#[test]
fn a_document_that_cannot_be_parsed_is_reported_by_position() {
    let output = piped(&["fmt", "-"], "{a = }");
    assert!(!output.status.success());
    let message = err(&output);
    assert!(message.starts_with("<stdin>:1:6:"), "{message}");
}

#[test]
fn a_document_in_the_style_it_asks_for_is_silent() {
    let file = fixtures().join("surface/valid/dotfile.nuke");
    let output = nuke(&["lint", file.to_str().unwrap()]);
    assert!(output.status.success(), "{}", err(&output));
    assert_eq!(out(&output), "");
}

#[test]
fn a_document_out_of_style_names_the_rule_by_position() {
    let output = piped(&["lint", "-"], "{a__b = HTTPServer c = @import \"./p\"}");
    assert!(!output.status.success());
    let stdout = out(&output);
    let reported: Vec<&str> = stdout.lines().collect();
    assert_eq!(reported.len(), 3, "{reported:?}");
    assert!(
        reported[0].starts_with("<stdin>:1:2: ident-case:"),
        "{reported:?}"
    );
    assert!(
        reported[1].starts_with("<stdin>:1:9: atom-case:"),
        "{reported:?}"
    );
    assert!(
        reported[2].starts_with("<stdin>:1:32: import-extension:"),
        "{reported:?}"
    );
}

#[test]
fn a_document_that_does_not_parse_is_a_fault_and_not_a_finding() {
    let output = piped(&["lint", "-"], "{a = }");
    assert!(!output.status.success());
    assert_eq!(out(&output), "");
    assert!(err(&output).starts_with("<stdin>:1:6:"), "{}", err(&output));
}

#[test]
fn linting_a_file_leaves_it_alone() {
    let directory = TempDir::new().expect("a temporary directory");
    let file = directory.path().join("out-of-style.nuke");
    let before = "{a__b = 1}\n";
    std::fs::write(&file, before).expect("the document should be written");
    let output = nuke(&["lint", file.to_str().unwrap()]);
    assert!(!output.status.success());
    assert_eq!(
        std::fs::read_to_string(&file).expect("the document should still be readable"),
        before
    );
}

#[test]
fn the_server_speaks_the_protocol_over_a_pipe() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nuke"))
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("the binary should run");
    let mut stdin = child.stdin.take().expect("stdin should be a pipe");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout should be a pipe"));

    said(
        &mut stdin,
        Message::Request(Request::new(
            RequestId::from(0),
            "initialize".to_owned(),
            serde_json::json!({ "capabilities": {} }),
        )),
    );
    let Some(Message::Response(response)) = Message::read(&mut stdout).expect("a reply") else {
        panic!("the server answered initialize with something else");
    };
    let result = response.result.expect("a result");
    assert!(
        result["capabilities"]["definitionProvider"] == serde_json::json!(true),
        "{result}"
    );
    said(
        &mut stdin,
        Message::Notification(Notification::new(
            "initialized".to_owned(),
            serde_json::json!({}),
        )),
    );
    said(
        &mut stdin,
        Message::Request(Request::new(
            RequestId::from(1),
            "shutdown".to_owned(),
            serde_json::Value::Null,
        )),
    );
    let Some(Message::Response(response)) = Message::read(&mut stdout).expect("a reply") else {
        panic!("the server answered shutdown with something else");
    };
    assert_eq!(response.id, RequestId::from(1));
    said(
        &mut stdin,
        Message::Notification(Notification::new(
            "exit".to_owned(),
            serde_json::Value::Null,
        )),
    );
    drop(stdin);

    let status = child.wait().expect("the server should exit");
    assert!(status.success(), "{status}");
}

fn said(pipe: &mut impl Write, message: Message) {
    message.write(pipe).expect("the message should be written");
}

fn generated(name: &str) -> String {
    let file = Path::new(env!("OUT_DIR")).join(name);
    std::fs::read_to_string(&file)
        .unwrap_or_else(|error| panic!("{} should have been generated: {error}", file.display()))
}

const COMPLETIONS: [&str; 6] = [
    "nuke.bash",
    "nuke.elv",
    "nuke.fish",
    "_nuke.ps1",
    "_nuke",
    "nuke.nu",
];

const INSTALLED: [&str; 4] = ["nuke.bash", "nuke.fish", "_nuke", "nuke.nu"];

const MANUALS: [&str; 6] = [
    "nuke.1",
    "nuke-render.1",
    "nuke-deps.1",
    "nuke-fmt.1",
    "nuke-lint.1",
    "nuke-lsp.1",
];

#[test]
fn the_build_writes_a_completion_for_every_shell_and_a_page_for_every_verb() {
    for name in COMPLETIONS.into_iter().chain(MANUALS) {
        assert!(!generated(name).trim().is_empty(), "{name} is empty");
    }
}

#[test]
fn a_completion_a_shell_installs_names_every_verb_and_every_target() {
    for name in INSTALLED {
        let script = generated(name);
        for verb in ["render", "deps", "fmt", "lint", "lsp"] {
            assert!(script.contains(verb), "{name} does not offer {verb}");
        }
        for target in Target::ALL {
            assert!(
                script.contains(target.name()),
                "{name} does not offer {}",
                target.name()
            );
        }
    }
}

#[test]
fn the_help_for_render_lists_the_targets_that_can_be_named() {
    let output = nuke(&["render", "--help"]);
    assert!(output.status.success(), "{}", err(&output));
    let help = out(&output);
    for target in Target::ALL {
        assert!(help.contains(target.name()), "{help}");
    }
}
