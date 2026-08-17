use std::collections::HashSet;

use abnf::types::{Kind, Node, Rule};
use nuke_grammar::{CANONICAL, CANONICAL_ABNF, Layer, TOKENS_ABNF};

fn rules(name: &str, abnf: &str) -> Vec<Rule> {
    abnf::rulelist(abnf).unwrap_or_else(|error| panic!("{name} is not valid ABNF: {error}"))
}

fn defined(abnf: &str) -> HashSet<String> {
    rules("a fragment", abnf)
        .iter()
        .map(|rule| rule.name().to_owned())
        .collect()
}

fn mentions(node: &Node, found: &mut Vec<String>) {
    match node {
        Node::Rulename(name) => found.push(name.clone()),
        Node::Alternatives(nodes) | Node::Concatenation(nodes) => {
            nodes.iter().for_each(|node| mentions(node, found));
        }
        Node::Repetition { node, .. } | Node::Group(node) | Node::Optional(node) => {
            mentions(node, found);
        }
        Node::String(_) | Node::TerminalValues(_) | Node::Prose(_) => {}
    }
}

fn reachable(rules: &[Rule], roots: &[&str]) -> HashSet<String> {
    let mut found: HashSet<String> = HashSet::new();
    let mut pending: Vec<String> = roots.iter().map(|root| (*root).to_owned()).collect();
    while let Some(name) = pending.pop() {
        if !found.insert(name.clone()) {
            continue;
        }
        for rule in rules.iter().filter(|rule| rule.name() == name) {
            mentions(rule.node(), &mut pending);
        }
    }
    found
}

fn roots(layer: Layer) -> Vec<&'static str> {
    let mut roots = vec![layer.start];
    roots.extend(layer.refined.iter().map(|(_, refining)| *refining));
    roots
}

fn syntax_layers() -> Vec<(&'static str, &'static str)> {
    vec![("canonical.abnf", CANONICAL_ABNF)]
}

fn assemblies() -> Vec<(&'static str, Layer)> {
    vec![("the canonical form", CANONICAL)]
}

#[test]
fn every_grammar_file_is_valid_abnf_on_its_own() {
    for (name, abnf) in [("tokens.abnf", TOKENS_ABNF)]
        .into_iter()
        .chain(syntax_layers())
    {
        rules(name, abnf);
    }
}

#[test]
fn no_syntax_layer_redefines_a_rule_the_token_layer_defines() {
    let tokens = defined(TOKENS_ABNF);
    for (name, abnf) in syntax_layers() {
        for rule in rules(name, abnf) {
            match rule.kind() {
                Kind::Basic => assert!(
                    !tokens.contains(rule.name()),
                    "{name} redefines `{}`, and a later definition wins in silence",
                    rule.name()
                ),
                Kind::Incremental => assert!(
                    tokens.contains(rule.name()),
                    "{name} extends `{}`, which the token layer does not define",
                    rule.name()
                ),
            }
        }
    }
}

#[test]
fn every_rule_an_assembly_defines_is_reachable_from_where_that_language_starts() {
    for (language, layer) in assemblies() {
        let rules = rules(language, &layer.abnf());
        let reachable = reachable(&rules, &roots(layer));
        for rule in &rules {
            assert!(
                reachable.contains(rule.name()),
                "{language} carries `{}`, which nothing in it reaches",
                rule.name()
            );
        }
    }
}

#[test]
fn every_rule_an_assembly_mentions_is_one_it_defines() {
    for (language, layer) in assemblies() {
        let rules = rules(language, &layer.abnf());
        let defined = defined(&layer.abnf());
        let mut mentioned = Vec::new();
        for rule in &rules {
            mentions(rule.node(), &mut mentioned);
        }
        for name in roots(layer)
            .into_iter()
            .map(ToOwned::to_owned)
            .chain(mentioned)
        {
            assert!(
                defined.contains(&name),
                "{language} mentions `{name}`, which nothing in it defines"
            );
        }
    }
}
