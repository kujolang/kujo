# Documented helper candidate verification

Verified against the Kujo implementation and tests on 2026-07-11. These four
candidates remain documentation-only dispositions. No new runtime API or
alias was added.

## HLP-007 — canonical ISO/time, slug, and padding helpers

- Canonical calls: `slugify`, `pad_left`/`pad_right`, `now_utc`, `now_unix`,
  `format_date`; existing `pad_start`/`pad_end` aliases are also documented.
- Verified semantics: `format_date` consumes Unix seconds, while
  `current_timestamp` is Unix milliseconds; slugification and Unicode-character
  padding edge cases are documented.
- Runtime parity: the existing padding aliases were added to the type-checker
  signature table because they were registered and dispatched but a CLI run
  could otherwise report them as undefined.
- Runnable proof: `examples/helper_hlp_007_text_time.kujo` passes on the VM
  path and the interpreter example smoke lane.

## HLP-011 — typed environment configuration

- Canonical calls: `env_or`, `env_int`, `env_float`, `env_bool`, and
  `env_required`, with `env` documented for the explicit empty-string fallback.
- Verified semantics: integer/float/bool return typed values; bool spellings,
  missing-variable behavior, required empty values, error objects, and
  `env-read`/`env-write` capability boundaries are documented.
- Runnable proof: `examples/helper_hlp_011_env_config.kujo` passes on the VM
  path and the interpreter example smoke lane.

## HLP-013 — structured ProcessResult access

- Canonical calls: `spawn_process(argv, options?)` for explicit argv and
  `execute_status(command, options?)` for the shell compatibility boundary.
- Verified semantics: field spelling, lossy UTF-8 output, timeout and success
  interaction, per-stream truncation, option defaults/limits, environment
  policy options, and capability requirements are documented. `execute` is
  explicitly described as stdout-only exception-style shell execution.
- Runnable proof: `examples/helper_hlp_013_process_result.kujo` passes on the VM
  path and the interpreter example smoke lane. Timeout and output-truncation
  integration tests pass.

## HLP-015 — deterministic canonical JSON serialization

- Canonical calls: `to_json`, `to_json_pretty`, and `parse_json`; no separate
  canonicalization alias is justified.
- Verified semantics: map-key ordering, fixed/dense ordering, whitespace-only
  pretty output differences, supported/unsupported values, secret redaction,
  non-finite float rejection, JSON root coverage, and input/nesting limits are
  documented. Runtime structs must be projected into dictionaries before JSON
  serialization.
- Runnable proof: `examples/helper_hlp_015_canonical_json.kujo` passes on the VM
  path and the interpreter example smoke lane. Native JSON contract tests pass.

## Verification commands

- `cargo fmt --check`
- `cargo test --test stdlib_reference_contract`
- `cargo test --test docs_examples`
- `cargo test --test native_json`
- `cargo test --test native_api_security_boundaries process_timeout_kills_long_running_process`
- `cargo test --test native_api_security_boundaries process_output_limit_sets_truncation_flags`
- `cargo run --quiet -- run examples/helper_hlp_007_text_time.kujo`
- `cargo run --quiet -- run examples/helper_hlp_011_env_config.kujo`
- `cargo run --quiet -- run examples/helper_hlp_013_process_result.kujo`
- `cargo run --quiet -- run examples/helper_hlp_015_canonical_json.kujo`

All commands above passed during this verification pass. Full repository and
VM/interpreter dual-runtime gates remain the final completion checks.
