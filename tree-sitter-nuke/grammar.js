/// <reference types="tree-sitter-cli/dsl" />

// Nuke — the surface language, for an editor.
//
// This grammar is not normative. `grammar/tokens.abnf` followed by `grammar/surface.abnf` is,
// and `crates/nuke-syntax` is the implementation both are held to. This one exists because an
// editor needs a tree for text that is not finished, which neither of those can give it.
//
// `docs/highlighting.md` argues where it is allowed to disagree with them, and
// `test/verdicts` names every fixture where it does.

module.exports = grammar({
  name: 'nuke',

  extras: $ => [/[ \t\n]/, $.comment],

  rules: {
    // A file stands in for the braces of the tuple it holds, so what follows the bindings is
    // either one value or a run of fields. `repeat1` and not `repeat`, or an empty file would
    // be a document. The fields are the document's own children rather than a node of their
    // own, which is what makes `(document) @local.scope` the scope `(tuple)` would have been.
    document: $ => seq(repeat($.binding), choice(repeat1($.field), $._value)),

    binding: $ => seq(field('name', $.ident), ':=', field('value', $._value)),

    _value: $ => choice($._operand, $.access, $.apply, $.pipe),

    // `<|` is the loosest operator and right-associative, `|>` is looser than the dot and
    // left-associative, and the dot binds tightest of the three. `grammar/surface.abnf`
    // spells the same ladder as four rules, a PEG having no precedence of its own.
    apply: $ => prec.right(1, seq(
      field('function', $._value),
      '<|',
      field('argument', $._value),
    )),

    pipe: $ => prec.left(2, seq(
      field('argument', $._value),
      '|>',
      field('function', $._value),
    )),

    access: $ => prec.left(3, seq(
      field('operand', $._value),
      '.',
      field('key', choice($.ident, $.group)),
    )),

    group: $ => seq('(', $._value, ')'),

    _operand: $ => choice(
      $.group,
      $.builtin,
      $.interpolation,
      $.tuple,
      $.map,
      $.list,
      $.reference,
      $.atom,
      $.string,
      $.number,
      $.malformed_number,
    ),

    builtin: $ => seq('@', field('name', $.ident)),

    tuple: $ => seq('{', repeat($.binding), repeat($.field), '}'),
    field: $ => seq(field('name', $.ident), '=', field('value', $._value)),

    map: $ => seq('{', repeat($.binding), repeat1($.entry), '}'),
    entry: $ => seq(field('key', $._value), '=>', field('value', $._value)),

    list: $ => seq('[', repeat($._value), ']'),

    reference: $ => $.ident,

    string: $ => seq(
      '"',
      repeat(choice($._string_content, $.escape_sequence)),
      token.immediate('"'),
    ),

    _string_content: _ => token.immediate(prec(1, /[^"\\\x00-\x1F]+/)),

    escape_sequence: _ => token.immediate(/\\(["\\nrt]|u\{[0-9A-F]{1,6}\})/),

    interpolation: $ => seq(
      '$"',
      repeat(choice($._text, $.escape_sequence, $.doubled_brace, $.hole)),
      token.immediate('"'),
    ),

    _text: _ => token.immediate(prec(1, /[^"\\{}\x00-\x1F]+/)),

    doubled_brace: _ => token.immediate(choice('{{', '}}')),

    hole: $ => seq(
      token.immediate('{'),
      field('value', $._value),
      optional(seq(':', field('spec', $.format_spec))),
      '}',
    ),

    format_spec: _ => token.immediate(prec(1, /[^"{}\x00-\x1F]+/)),

    atom: _ => /[A-Z][A-Za-z0-9]*/,

    ident: _ => /[a-z][a-z0-9_]*/,

    number: _ => token(choice(
      /-?0(b[01]+|q[0-3]+|o[0-7]+|x[0-9A-F]+)+/,
      /-?(0|[1-9][0-9]*)(\.[0-9]+)?(e-?(0|[1-9][0-9]*))?/,
    )),

    malformed_number: _ => token(
      /-?(0[bqox][A-Za-z0-9]*(\.[0-9]*)?|[0-9]+(\.[0-9]*)?([eE][-+]?[0-9]*)?)/,
    ),

    comment: _ => token(/#[^\x00-\x08\x0A-\x1F]*/),
  },
});
