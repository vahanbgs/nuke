use std::fs;
use std::path::{Path, PathBuf};

use nuke_eval::eval_at_with_files;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .canonicalize()
        .expect("the fixture tree should exist")
}

fn files_of(relative: &str) -> Vec<PathBuf> {
    let path = fixtures().join(relative);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
    eval_at_with_files(&source, &path)
        .unwrap_or_else(|error| panic!("{} should reduce: {error}", path.display()))
        .files
}

#[test]
fn a_document_that_imports_nothing_names_only_itself() {
    let files = files_of("surface/valid/dotfile.nuke");
    assert_eq!(files, vec![fixtures().join("surface/valid/dotfile.nuke")]);
}

#[test]
fn the_entry_file_stands_first() {
    let files = files_of("surface/valid/imports-a-module.nuke");
    assert_eq!(
        files,
        vec![
            fixtures().join("surface/valid/imports-a-module.nuke"),
            fixtures().join("surface/modules/palette.nuke"),
        ]
    );
}

#[test]
fn a_diamond_names_the_file_it_shares_once() {
    let files = files_of("surface/valid/a-diamond-of-imports.nuke");
    assert_eq!(
        files,
        vec![
            fixtures().join("surface/valid/a-diamond-of-imports.nuke"),
            fixtures().join("surface/modules/palette.nuke"),
            fixtures().join("surface/modules/theme.nuke"),
        ]
    );
}

#[test]
fn every_file_is_named_by_its_canonical_path() {
    let entry = fixtures().join("surface/valid/./imports-a-module.nuke");
    let source = fs::read_to_string(&entry).expect("the fixture should be readable");
    let files = eval_at_with_files(&source, &entry)
        .expect("the fixture should reduce")
        .files;
    for file in &files {
        assert_eq!(
            file,
            &file.canonicalize().expect("the file should exist"),
            "{} is not canonical",
            file.display()
        );
    }
}

#[test]
fn a_reduction_that_faults_reports_no_files() {
    for name in [
        "a-cycle-between-two-files.nuke",
        "a-file-that-imports-itself.nuke",
        "an-import-of-a-file-that-is-not-there.nuke",
    ] {
        let path = fixtures().join("surface/refused").join(name);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{name} should be readable: {error}"));
        eval_at_with_files(&source, &path).expect_err("{name} should be refused");
    }
}
