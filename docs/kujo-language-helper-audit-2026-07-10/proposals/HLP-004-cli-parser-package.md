# HLP-004 — declarative CLI parser package

## Problem and evidence

PatchBrief, Muzzle, Lens, ShipCheck, and Kennel implement overlapping
`parse_subcommand`, `has_flag`, and `flag_value` functions. ChangeBucket,
RunLedger, PackWrite, Tribunal, and Fence implement related `parse_args`
variants. The common token scan is simple, but missing-value, boolean, repeated
flag, `--key=value`, short-flag, and positional semantics vary. Core
`arg_parser()` currently returns an empty `ArgParser` struct.

## Root cause and ownership

Token parsing is reusable; CLI policy and help text are not universal language
semantics. Put a small declarative parser in an official first-party package,
not in the language core. It should return structured errors and leave rendering
and exit-code policy to the caller.

## Proposed API

Proposed syntax:

```kujo
spec := cli.spec([
    {'name': 'json', 'kind': 'bool'},
    {'name': 'output', 'kind': 'value', 'required': false}
])
parsed := cli.parse(args(), spec)
```

Recommended signature: `cli.parse(argv: array, spec: array|dict) ->
Result<dict, CliError>`. Success includes `command`, `positionals`, `options`,
and `occurrences`. Error includes `index`, `token`, `code`, and `message`.
Specify no implicit shell parsing, no environment reads, no process exits, and
no mutation. Support long flags first; add short aliases only when the spec
declares them. Reject a flag that requires a value when the next token is a
flag, unless `allow_empty` is explicit.

## Alternatives considered

- Expand core `arg_parser()` into a full framework: too much policy for core.
- Keep local helpers: preserves drift and makes agent-generated CLIs inconsistent.
- Add only `parse_args`: misses typed validation and precise errors.
- Adopt a third-party parser: conflicts with local-first dependency goals.

## Migration

Migrate Muzzle or PatchBrief first because their parser contracts are small and
well documented. Compare Help/JSON/exit output byte-for-byte. Then migrate
ShipCheck, Lens, Kennel, ChangeBucket, and RunLedger only where the package
supports their existing semantics. Do not force a migration of complex Eval or
RAG script parsing until the spec grows deliberately.

## Tests and performance

Test empty argv, command-first, flag-first, `--key=value`, separated values,
boolean flags, negative numeric values, repeated flags, unknown flags, missing
values, `--` terminator, Unicode, deterministic ordering, and error indexes.
Parsing is O(n) and should not need benchmarking beyond large argv regression.

## Agent benefit and risks

The package gives agents one obvious parser and makes ambiguous tokens fail
early. It must not generate help text or silently coerce values; those behaviors
would hide application policy and make review harder.

## Status

Implemented as the first-party modules/cli.kujo package spike. The initial
compatibility fixture covers long and short flags, equals and separated
values, repeated occurrences, defaults, required options, negative values,
unknown/missing errors, and the -- terminator on both runtime paths. Consumer
migration and a broader compatibility matrix remain follow-up work.
