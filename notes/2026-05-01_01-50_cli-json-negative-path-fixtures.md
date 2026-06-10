# CLI JSON Negative-Path Fixture Expansion (v1.0.0 P0)

Date: 2026-05-01

## Summary

Executed the v1.0.0 P0 negative-path contract fixture cycle and closed the roadmap item.

Completed in this cycle:

- Added fixture-locked negative-path CLI JSON contract tests for malformed params, unknown-symbol failures, and missing-file IO failures.
- Removed panic-based `format`/`lint` file-read/write paths that previously surfaced as process panic exits (`101`) instead of deterministic runtime failure exits.
- Updated contract docs/roadmap/changelog/notes to reflect locked negative-path guarantees.

## Code Changes

### CLI failure-path hardening

- File: `src/main.rs`
- Changes:
  - `Commands::Format` now handles read/write errors with deterministic `stderr` messages and `exit(1)` (no panic).
  - `Commands::Lint` now handles read/write errors with deterministic `stderr` messages and `exit(1)` (no panic).

### Negative-path contract fixtures

- File: `tests/cli_json_contracts.rs`
- Added test: `cli_json_negative_paths_have_stable_failure_signals`
- Coverage includes:
  - missing-file `format --json` and `lint --json` IO failures (exit `1`, empty `stdout`, deterministic `stderr`)
  - malformed `--line` parameter for `lsp-definition --json` (exit `2`, empty `stdout`, Clap diagnostic on `stderr`)
  - unknown-symbol `lsp-rename --json` failure (exit `1`, empty `stdout`, deterministic `stderr`)

## Documentation Updates

- Updated `docs/CLI_MACHINE_READABLE_CONTRACTS.md` with a dedicated negative-path fixture guarantees section.
- Marked roadmap P0 item complete in `ROADMAP.md` for negative-path fixture expansion.
- Updated `CHANGELOG.md` and `notes/README.md` with this cycle results.

## Verification

Commands run:

1. `cargo test --test cli_json_contracts`
- Result: PASS
- Passed tests: 5
- Failed tests: 0

## Follow-Through Context

The previously open parity P0 remains open only for the tag-style match-binding capability gap. Negative-path contract fixture P0 is now complete.
