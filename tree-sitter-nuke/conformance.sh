#!/usr/bin/env bash
# Holds the tree-sitter grammar to the corpus the ABNF and the parser already answer to.
#
# `test/verdicts` names every fixture this grammar does not simply accept, and it is
# exhaustive: a fixture missing from it must parse clean, and a name in it that is no
# longer a fixture is a stale line. docs/highlighting.md argues each divergence.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
fixtures=../fixtures
verdicts=test/verdicts
status=0

fail() {
	printf '%s\n' "$1" >&2
	status=1
}

# What a tree says: `refused` if it holds an ERROR or a MISSING node, `named` if it
# parses but holds a malformed number, `admitted` if it parses clean.
read_of() {
	local tree
	tree=$(tree-sitter parse "$1" 2>/dev/null) || {
		printf 'refused\n'
		return
	}
	case "$tree" in
	*ERROR* | *MISSING*) printf 'refused\n' ;;
	*malformed_number*) printf 'named\n' ;;
	*) printf 'admitted\n' ;;
	esac
}

for tree in valid invalid surface/valid surface/reduced surface/invalid \
	surface/refused surface/modules; do
	for file in "$fixtures/$tree"/*.nuke; do
		name="$tree/$(basename "$file")"
		want=$(awk -v n="$name" '$1 == n {print $2}' "$verdicts")
		got=$(read_of "$file")
		[ -n "$want" ] || want=admitted
		[ "$want" = "$got" ] ||
			fail "$name reads $got, and $verdicts says $want"
	done
done

while read -r name _; do
	case "$name" in '' | '#'*) continue ;; esac
	[ -f "$fixtures/$name" ] || fail "$verdicts names $name, which is not a fixture"
done <"$verdicts"

tree-sitter test || fail 'the corpus and the grammar disagree'

# Helix compiles a query against the grammar it finds, and one unknown node type
# fails the whole file rather than the one pattern, so every .scm is compiled here.
for query in queries/*.scm; do
	tree-sitter query "$query" "$fixtures/valid/dotfile.nuke" >/dev/null 2>&1 ||
		fail "$query names something grammar.js does not"
done

# The parser an editor compiles is generated, so it must be what the grammar says.
tree-sitter generate || fail 'tree-sitter generate failed'
[ -z "$(git status --porcelain -- src)" ] ||
	fail 'src/ is not the committed parser — commit what grammar.js generates'

exit $status
