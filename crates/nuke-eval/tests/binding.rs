use std::io;
use std::path::{Path, PathBuf};

use nuke_eval::bind::{Error, ErrorKind};
use nuke_eval::{from_path, from_source};
use nuke_syntax::Atom;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct DotFile {
    editor: Editor,
    shell: Shell,
    keybindings: Vec<Binding>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Editor {
    theme: String,
    tab_width: u8,
    line_numbers: Atom,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Shell {
    history_size: u32,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Binding {
    keys: String,
    action: Atom,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Themed {
    editor: Paint,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Paint {
    theme: String,
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/surface/valid")
        .join(name)
}

#[test]
fn a_document_the_canonical_parser_refuses_still_binds() {
    let source = "accent := \"#FE8019\"\n{editor = {theme = accent}}";
    nuke_syntax::from_str::<Themed>(source).expect_err("the canonical parser reads no binding");
    let themed: Themed =
        from_source(source, Path::new("theme.nuke")).expect("the surface language should bind");
    assert_eq!(themed.editor.theme, "#FE8019");
}

#[test]
fn a_file_binds_a_type_out_of_the_surface_language() {
    let dotfile: DotFile = from_path(&fixture("dotfile.nuke")).expect("the fixture should bind");
    assert_eq!(dotfile.editor.theme, "gruvbox-dark");
    assert_eq!(dotfile.editor.tab_width, 2);
    assert_eq!(dotfile.editor.line_numbers.as_str(), "Relative");
    assert_eq!(dotfile.shell.history_size, 10000);
    assert_eq!(dotfile.keybindings.len(), 2);
    assert_eq!(dotfile.keybindings[0].action.as_str(), "OpenTerminal");
}

#[test]
fn an_import_resolves_against_the_file_that_spells_it() {
    let themed: Themed =
        from_path(&fixture("imports-a-module.nuke")).expect("the module should be found");
    assert_eq!(themed.editor.theme, "#FE8019");
}

#[test]
fn a_file_that_is_not_there_names_itself() {
    let path = fixture("no-such-document.nuke");
    let error: Error = from_path::<Themed>(&path).expect_err("nothing should be read");
    assert_eq!(error.path(), path);
    assert_eq!(
        error.kind(),
        &ErrorKind::Unreadable(io::ErrorKind::NotFound)
    );
    assert!(error.to_string().contains("cannot be read"));
}

#[test]
fn a_fault_in_the_document_carries_where_it_stands() {
    let source = "{editor = {theme = missing}}";
    let error = from_source::<Themed>(source, Path::new("theme.nuke"))
        .expect_err("an unbound name should fault");
    let ErrorKind::Reduction { at, cause } = error.kind() else {
        panic!("expected a reduction fault, got {error}");
    };
    assert_eq!(at.to_string(), "1:20");
    assert_eq!(
        cause.kind(),
        &nuke_eval::ErrorKind::Unbound("missing".into())
    );
}

#[test]
fn a_value_the_type_cannot_hold_is_a_binding_fault() {
    let source = "{editor = {theme = 1}}";
    let error = from_source::<Themed>(source, Path::new("theme.nuke"))
        .expect_err("an integer is no string");
    assert!(
        matches!(error.kind(), ErrorKind::Binding(_)),
        "expected a binding fault, got {error}"
    );
}
