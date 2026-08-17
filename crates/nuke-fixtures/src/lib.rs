use std::fs;
use std::path::{Path, PathBuf};

pub struct Fixture {
    pub path: PathBuf,
    pub source: String,
}

impl Fixture {
    pub fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    }

    pub fn display(&self) -> std::path::Display<'_> {
        self.path.display()
    }
}

pub struct Reduction {
    pub source: Fixture,
    pub reduced: Fixture,
}

impl Reduction {
    pub fn name(&self) -> &str {
        self.source.name()
    }

    pub fn display(&self) -> std::path::Display<'_> {
        self.source.display()
    }
}

pub fn valid() -> Vec<Fixture> {
    read("valid")
}

pub fn invalid() -> Vec<Fixture> {
    read("invalid")
}

pub fn surface_invalid() -> Vec<Fixture> {
    read("surface/invalid")
}

pub fn surface_refused() -> Vec<Fixture> {
    read("surface/refused")
}

pub fn reductions() -> Vec<Reduction> {
    let mut reduced = read("surface/reduced");
    let paired: Vec<Reduction> = read("surface/valid")
        .into_iter()
        .map(|source| {
            let at = reduced
                .iter()
                .position(|candidate| candidate.name() == source.name())
                .unwrap_or_else(|| {
                    panic!(
                        "{} has no counterpart in fixtures/surface/reduced",
                        source.display()
                    )
                });
            let reduced = reduced.remove(at);
            Reduction { source, reduced }
        })
        .collect();
    if let Some(orphan) = reduced.first() {
        panic!(
            "{} is the reduction of nothing in fixtures/surface/valid",
            orphan.display()
        );
    }
    paired
}

fn read(kind: &str) -> Vec<Fixture> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(kind);
    let mut found: Vec<Fixture> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", dir.display()))
        .map(|entry| entry.expect("cannot read directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "nuke")
        })
        .map(|path| {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            Fixture { path, source }
        })
        .collect();
    found.sort_by(|one, other| one.path.cmp(&other.path));
    assert!(!found.is_empty(), "no fixtures found in {}", dir.display());
    found
}
