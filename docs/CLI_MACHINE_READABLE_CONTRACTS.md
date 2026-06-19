# CLI Machine-Readable Contracts

Status: v1.0.0 baseline draft (active)
Contract version: `1.0.0-draft`
Last updated: 2026-05-31

This document defines automation-facing contracts for Kujo CLI JSON outputs and exit behavior.

## Exit-Code Policy

Kujo user-facing commands follow this policy:

- `0`: command completed successfully
- `1`: command completed with a generic command failure or unmet gate (for example `format --check`, `lint` errors, failed `test-run` assertions, benchmark throughput gate failures)
- `2`: command-line usage/argument parse error (Clap-level usage failure)
- `3`: lexical/parser diagnostic failure
- `4`: runtime execution/semantic failure
- `5`: IO failure (for example missing input file, read/write/create failure)
- `6`: internal/tooling failure (for example unexpected runtime panic surfaces or JSON serialization failure)

Notes:

- Commands that intentionally gate behavior (for example format check mode) use `1` when the requested gate is not met.
- For automation, treat any non-zero exit as failure unless a command-specific policy explicitly documents otherwise.
- `tests/cli_contracts.rs` and `tests/cli_json_contracts.rs` lock these exit-code contracts.

## Error Shape Policy

Current v1.0.0 baseline draft for CLI `--json` mode:

- `stdout` is reserved for machine-readable JSON payloads on successful command execution.
- `stderr` carries human-readable error text for failures.
- A non-zero exit code indicates failure.

Exception for `kujo run --json-runtime-diagnostics`:

- runtime/VM execution failures emit a machine-readable JSON error payload on `stdout`
- `stderr` is suppressed for those JSON-mode runtime failures
- non-zero exit code remains authoritative (`4` for runtime failures, `6` for internal failures in JSON serialization/panic paths)

Exception for `kujo lsp-rename --json`:

- runtime rename failures emit a machine-readable JSON error payload on `stdout`
- `stderr` is suppressed for those JSON-mode rename failures
- non-zero exit code remains authoritative (`4` for runtime failures, `6` for internal failures in JSON serialization/panic paths)

Automation recommendation:

- Read JSON only from `stdout` on zero exit.
- For documented JSON-mode failure envelopes (`run --json-runtime-diagnostics`, `lsp-rename --json`), read JSON from `stdout` even on non-zero exit.
- Treat non-zero exit as authoritative failure signal.
- Capture `stderr` for troubleshooting and classification.

## Stable JSON Output Surfaces

The following command families are covered by schema-gating integration tests in `tests/cli_json_contracts.rs`.

### `kujo format --json`

Top-level object fields:

- `command` (string, constant `"format"`)
- `file` (string)
- `status` (string: `"preview" | "already_formatted" | "needs_formatting" | "written"`)
- `changed` (boolean)
- `options` (object)
  - `indent` (number)
  - `line_length` (number)
  - `sort_imports` (boolean)
  - `check` (boolean)
  - `write` (boolean)
- `formatted_source` (string or null)

### `kujo lint --json`

Top-level array of issue objects. Per item fields:

- `rule_id` (string)
- `line` (number)
- `column` (number)
- `severity` (string)
- `message` (string)
- `fix` (object or null)

### `kujo check --json`

Top-level object fields:

- `command` (string, constant `"check"`)
- `file` (string)
- `status` (string, currently `"ok"`)
- `statement_count` (number)
- `bytecode_instruction_count` (number)

### `kujo docgen --json`

Top-level object fields:

- `command` (string, constant `"docgen"`)
- `file` (string)
- `output_dir` (string)
- `module_doc_path` (string)
- `builtin_doc_path` (string or null)
- `item_count` (number)
- `languages` (array of strings)
- `project_json_path` (string)
- `gaps_json_path` (string)
- `capabilities_json_path` (string)
- `ai_tasks_path` (string or null)
- `diagnostics_count` (number)
- `undocumented_count` (number)
- `broken_link_count` (number)
- `warning_count` (number)
- `adapter_health` (object keyed by language id)
  - per-language object fields:
    - `files_scanned` (number)
    - `symbols_extracted` (number)
    - `doc_blocks_attached` (number)
    - `placeholders_emitted` (number)
- `cache_stats` (object)
  - `hits` (number)
  - `misses` (number)
- `gate_failures` (array of strings)
- `discovery_skip_counts` (object)
  - `max_file_size` (number)
  - `max_depth` (number)
  - `max_files` (number)
  - `invalid_encoding` (number)
- `discovery_limits` (object)
  - `max_file_size_bytes` (number)
  - `max_depth` (number)
  - `max_files` (number)
- `link_validation_skip_counts` (object)
  - `max_link_checks` (number)
  - `max_external_checks` (number)
  - `max_total_time` (number)
- `summary` (object)
  - `schema_version` (string, constant `"docgen-summary/v1"`)
  - includes mirrored totals plus `adapter_health`, `cache_stats`, `discovery_limits`, `discovery_skip_counts`, and `link_validation_skip_counts`

Implementation note:
- The `kujo docgen --json` payload is emitted from a typed single-source builder in `src/docgen/core.rs` (`build_cli_json_payload`) and guarded by both shape assertions and a fixture-backed snapshot test in `tests/cli_json_contracts.rs`.

### `kujo run --json-runtime-diagnostics`

Top-level object fields:

- `command` (string, constant `"run"`)
- `status` (string, constant `"error"`)
- `kind` (string, constant `"runtime_diagnostic"`)
- `contract_version` (string, currently `"1.0.0-draft"`)
- `exit_code` (number, usually `4` for runtime/VM execution failures)
- `diagnostic` (object)
  - shape matches the shared diagnostic JSON contract fields:
    - `code`, `severity`, `subsystem`, `message`, `help`, `file`, `line`, `column`
- `runtime_kind` (string, optional; present when runtime error kind metadata is available)
- `call_stack` (array of strings, optional; present for runtime errors surfaced with stack context)

### LSP CLI helper surfaces (`--json`)

Covered commands:

- `lsp-complete`
- `lsp-definition`
- `lsp-references`
- `lsp-hover`
- `lsp-diagnostics`
- `lsp-rename`
- `lsp-code-actions`

Each command has a stable top-level payload kind (array/object/null as applicable) with required field assertions in `tests/cli_json_contracts.rs`.

`kujo lsp-diagnostics --json` item fields:

- `code` (string, stable diagnostic code such as `RUFLEX001` or `RUFPARSE001`)
- `severity` (string, currently `"error"`)
- `subsystem` (string, one of `lexer`, `parser`, `lsp`)
- `message` (string)
- `line` (number)
- `column` (number)
- `file` (string or null)
- `help` (string or null)

`kujo lsp-rename --json` failure payload fields:

- `command` (string, constant `"lsp-rename"`)
- `status` (string, constant `"error"`)
- `kind` (string, currently `"runtime_error"`)
- `contract_version` (string, currently `"1.0.0-draft"`)
- `exit_code` (number, currently `4` for runtime rename failures)
- `message` (string)

## Deterministic Human-Readable Test Summary Contracts

`kujo test` is intentionally human-readable, but these summary fields are contract-tested for operator tooling:

- `Runtime strategy: <mode>` is always emitted in summary output.
- `Fixture outcomes: passed=<N>, failed=<N>, skipped=<N>, expected_fail=<N>, runnable=<N>, discovered=<N>` is always emitted in summary output. `runnable` is the denominator used by `Passed <N>/<N> tests`; `discovered` additionally includes fixtures skipped because they declare `Run with: kujo test-run`.
- `--runtime dual` summary includes split counters:
  - `vm_primary=<N>`
  - `interpreter_fallback=<N>`
- Per-fixture fallback marker (`[dual fallback: interpreter]`) is emitted only when dual mode actually falls back for that fixture.

Coverage is locked in `tests/cli_contracts.rs`:

- `cli_test_discovers_and_runs_expected_fixtures`
- `cli_test_reports_fixture_failures_with_verification_exit_code`
- `cli_test_runtime_dual_mode_keeps_vm_primary_for_vm_drift_fixture`
- `cli_test_runtime_vm_mode_executes_vm_drift_fixture_without_snapshot_mismatch`

## Dual-Mode Inventory And Human Render Expectations

- Canonical dual-mode command inventory and stability classification:
  - `docs/CLI_DUAL_MODE_OUTPUT_INVENTORY.md`
- Human-render helper migration status and intentionally low-level residual call sites:
  - `docs/OUTPUT_HELPER_MIGRATION_NOTE.md`

Human-render determinism policy:

- Snapshot-style tests should use deterministic, no-color render paths where available.
- For operator-facing colorized paths, `NO_COLOR=1` should route to deterministic plain-text output when supported.
- Snapshot/test updates for human text must only happen when output intent changes are deliberate and documented in the same change.

## Contract Change Rules

Any payload-affecting change to the documented JSON shapes requires all of the following in the same change set:

- update this document
- update `tests/cli_json_contracts.rs` contract assertions
- add/update a `CHANGELOG.md` contract-impact note

## Negative-Path Fixture Guarantees

`tests/cli_json_contracts.rs::cli_json_negative_paths_have_stable_failure_signals` locks these failure-path automation guarantees:

- missing input files for JSON-mode `format`/`lint` exit with code `5`, emit no JSON payload on `stdout`, and report a deterministic read-failure message on `stderr`
- malformed CLI parameters (for example non-numeric `--line`) exit with code `2`, emit no JSON payload on `stdout`, and return Clap usage diagnostics on `stderr`
- unknown-symbol `lsp-rename --json` requests exit with code `4`, emit deterministic JSON error payloads on `stdout`, and keep `stderr` empty
