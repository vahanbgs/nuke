use std::str::FromStr;

use nuke_syntax::{Value, parse};
use nuke_transpile::{Target, ghostty, gitconfig, ini, json, kdl, lua, nix, toml, xml, yaml};

fn value_of(source: &str) -> Value {
    parse(source).expect("the source should parse")
}

fn by_module(target: Target, value: &Value) -> Result<String, String> {
    match target {
        Target::Json => json::to_string(value).map_err(|error| error.to_string()),
        Target::Yaml => yaml::to_string(value).map_err(|error| error.to_string()),
        Target::Toml => toml::to_string(value).map_err(|error| error.to_string()),
        Target::Xml => xml::to_string(value).map_err(|error| error.to_string()),
        Target::Kdl => kdl::to_string(value).map_err(|error| error.to_string()),
        Target::Lua => lua::to_string(value).map_err(|error| error.to_string()),
        Target::Ini => ini::to_string(value).map_err(|error| error.to_string()),
        Target::Gitconfig => gitconfig::to_string(value).map_err(|error| error.to_string()),
        Target::Nix => nix::to_string(value).map_err(|error| error.to_string()),
        Target::Ghostty => ghostty::to_string(value).map_err(|error| error.to_string()),
    }
}

#[test]
fn a_name_round_trips_through_its_spelling() {
    for target in Target::ALL {
        assert_eq!(Target::from_str(&target.to_string()), Ok(target));
        assert_eq!(target.to_string(), target.name());
    }
}

#[test]
fn every_target_is_named_once() {
    let mut names: Vec<&str> = Target::ALL.iter().map(|target| target.name()).collect();
    names.sort_unstable();
    let count = names.len();
    names.dedup();
    assert_eq!(names.len(), count, "two targets answer to one name");
}

#[test]
fn a_name_no_target_answers_to_is_refused() {
    for name in ["", "JSON", "cfg", "fish", "plist", "yml"] {
        let refused = Target::from_str(name).expect_err("no target should answer");
        assert_eq!(refused.name(), name);
    }
}

#[test]
fn an_extension_names_the_target_that_writes_it() {
    assert_eq!(Target::from_extension("json"), Some(Target::Json));
    assert_eq!(Target::from_extension("toml"), Some(Target::Toml));
    assert_eq!(Target::from_extension("yaml"), Some(Target::Yaml));
    assert_eq!(Target::from_extension("yml"), Some(Target::Yaml));
    assert_eq!(Target::from_extension("nix"), Some(Target::Nix));
}

#[test]
fn an_extension_no_target_claims_is_nobodys() {
    for extension in [
        "",
        "nuke",
        "conf",
        "config",
        "gitconfig",
        "ghostty",
        "JSON",
        "tielpmet",
    ] {
        assert_eq!(Target::from_extension(extension), None);
    }
}

#[test]
fn a_target_claims_no_extension_another_does() {
    for target in Target::ALL {
        for extension in target.extensions() {
            assert_eq!(Target::from_extension(extension), Some(target));
        }
    }
    assert!(
        Target::Gitconfig.extensions().is_empty(),
        "a git config file is named rather than extended"
    );
}

#[test]
fn naming_a_target_writes_what_calling_its_module_writes() {
    for fixture in nuke_fixtures::valid() {
        let value = parse(&fixture.source)
            .unwrap_or_else(|error| panic!("{} should parse: {error}", fixture.display()));
        for target in Target::ALL {
            let named = target.render(&value).map_err(|error| error.to_string());
            assert_eq!(
                named,
                by_module(target, &value),
                "{} disagrees with {target} called by name",
                fixture.display()
            );
        }
    }
}

#[test]
fn a_refusal_carries_the_target_and_the_path_it_stopped_at() {
    let value = value_of("{a = {b = [1 2]}}");
    let refusal = Target::Ini.render(&value).expect_err("INI should refuse");
    assert_eq!(refusal.target(), Target::Ini);
    assert_eq!(refusal.path().to_string(), "a.b");
    assert_eq!(
        refusal.to_string(),
        ini::to_string(&value)
            .expect_err("INI should refuse")
            .to_string()
    );
}
