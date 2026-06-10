module.exports = grammar({
  name: 'kujo',

  extras: $ => [
    /\s/,
    $.comment,
  ],

  word: $ => $.identifier,

  rules: {
    source_file: $ => repeat($._statement),

    _statement: $ => choice(
      $.function_definition,
      $.variable_declaration,
      $.expression_statement,
      $.return_statement,
      $.if_statement,
      $.for_statement,
      $.while_statement,
      $.block
    ),

    block: $ => seq('{', repeat($._statement), '}'),

    function_definition: $ => seq(
      'func',
      field('name', $.identifier),
      '(',
      optional($.parameter_list),
      ')',
      $.block
    ),

    parameter_list: $ => seq(
      $.identifier,
      repeat(seq(',', $.identifier))
    ),

    variable_declaration: $ => seq(
      choice('let', 'const', 'mut'),
      field('name', $.identifier),
      ':=',
      field('value', $.expression)
    ),

    return_statement: $ => seq('return', optional($.expression)),

    if_statement: $ => seq(
      'if',
      field('condition', $.expression),
      $.block,
      optional(seq('else', choice($.block, $.if_statement)))
    ),

    for_statement: $ => seq(
      'for',
      field('iterator', $.identifier),
      'in',
      field('iterable', $.expression),
      $.block
    ),

    while_statement: $ => seq(
      'while',
      field('condition', $.expression),
      $.block
    ),

    expression_statement: $ => $.expression,

    expression: $ => choice(
      $.call_expression,
      $.binary_expression,
      $.array,
      $.dictionary,
      $.string,
      $.number,
      $.identifier
    ),

    call_expression: $ => seq(
      field('function', $.identifier),
      '(',
      optional($.argument_list),
      ')'
    ),

    argument_list: $ => seq(
      $.expression,
      repeat(seq(',', $.expression))
    ),

    binary_expression: $ => prec.left(seq(
      $.expression,
      choice('+', '-', '*', '/', '==', '!=', '<', '<=', '>', '>='),
      $.expression
    )),

    array: $ => seq(
      '[',
      optional(seq($.expression, repeat(seq(',', $.expression)))),
      ']'
    ),

    dictionary: $ => seq(
      '{',
      optional(seq($.dictionary_pair, repeat(seq(',', $.dictionary_pair)))),
      '}'
    ),

    dictionary_pair: $ => seq(choice($.string, $.identifier), ':', $.expression),

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,
    number: $ => /[0-9]+(\.[0-9]+)?/,
    string: $ => /"([^"\\]|\\.)*"/,
    comment: $ => token(choice(
      seq('//', /[^\n]*/),
      seq('#', /[^\n]*/),
      seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/')
    )),
  },
});
