use std::collections::HashMap;
use std::path::PathBuf;

use nuke_syntax::from_str;
use nuke_transpile::Target;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Manifest {
    targets: HashMap<PathBuf, Target>,
}

fn atom_of(target: Target) -> String {
    format!("{target:?}")
}

#[test]
fn every_target_binds_from_the_atom_that_spells_it() {
    for target in Target::ALL {
        let atom = atom_of(target);
        assert_eq!(
            from_str::<Target>(&atom),
            Ok(target),
            "`{atom}` should name {target}"
        );
    }
}

#[test]
fn the_atom_a_document_spells_is_not_the_name_the_command_line_takes() {
    for target in Target::ALL {
        assert_ne!(
            atom_of(target),
            target.name(),
            "a target that spells the same both ways teaches nothing"
        );
        from_str::<Target>(&format!("\"{}\"", target.name()))
            .expect_err("the lowercase name belongs to a flag and not to a document");
    }
}

#[test]
fn an_atom_no_target_answers_to_is_refused() {
    for atom in ["Ghosty", "Json5", "Cfg", "Fish", "Nuke"] {
        from_str::<Target>(atom).expect_err("no target should answer");
    }
}

#[test]
fn a_table_binds_a_target_to_each_path_that_names_one() {
    let manifest: Manifest = from_str(
        "{targets = {\".config/ghostty/config\" => Ghostty \".config/git/config\" => Gitconfig}}",
    )
    .expect("a manifest of named targets should bind");

    assert_eq!(
        manifest,
        Manifest {
            targets: HashMap::from([
                (PathBuf::from(".config/ghostty/config"), Target::Ghostty),
                (PathBuf::from(".config/git/config"), Target::Gitconfig),
            ]),
        }
    );
}

#[test]
fn a_table_naming_a_target_nothing_writes_is_refused() {
    from_str::<Manifest>("{targets = {\".config/ghostty/config\" => Ghosty}}")
        .expect_err("a misspelled target should not reach a deployment");
}
