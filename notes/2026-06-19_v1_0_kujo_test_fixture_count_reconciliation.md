# V1.0 Kujo Test Fixture Count Reconciliation

Date: 2026-06-19
Checklist item: V1RR-P0-006
Status: complete

## Summary

Reconciled the `kujo test` fixture-count story by making the runner summary explicit and by removing host-dependent output from `tests/test_stdlib_system.kujo`.

Current `kujo test` output now distinguishes:

- `passed`
- `failed`
- `skipped`
- `expected_fail`
- `runnable`
- `discovered`

The current release-candidate fixture corpus has `150` discovered `.kujo` files under `tests/`: `144` runnable snapshot fixtures and `6` skipped framework fixtures that declare `Run with: kujo test-run`.

## Current Results

Command/status manifest:

- `notes/release_evidence/2026-06-19_p0-006/status.tsv`

Final `kujo test` summaries:

- `cargo run -- test --runtime vm`: `Passed 144/144 tests`; `Fixture outcomes: passed=144, failed=0, skipped=6, expected_fail=0, runnable=144, discovered=150`.
- `cargo run -- test --runtime dual`: `Passed 144/144 tests`; `Fixture outcomes: passed=144, failed=0, skipped=6, expected_fail=0, runnable=144, discovered=150`; `Runtime strategy: dual (vm_primary=144, interpreter_fallback=0)`.

## Policy Reconciliation

The `tests/` runner does not currently use expected-fail fixtures; its `expected_fail` counter is `0`.

Expected-fail policy remains scoped to example and documentation smoke tests:

- `examples/README_examples.md` lists legacy or expected-fail examples.
- `tests/docs_examples.rs` mirrors that list with `23` expected-fail examples and reasons.
- `cargo test --test docs_examples` passed, confirming the expected-fail examples still fail as expected and doc snippet expected-fail coverage remains empty.

## Fixture Stabilization

`tests/test_stdlib_system.kujo` previously printed host-dependent values:

- `args()` length, which can vary by runtime invocation path.
- sleep elapsed time derived from coarse `now()` seconds, which could render as either `~0ms` or `~1000ms`.

The fixture now checks `args()` shape and `sleep()` execution without printing variable values. This stabilized both `--runtime vm` and `--runtime dual` sweeps.

## Generated Evidence

Regenerated `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` and `.csv` after the fixture cleanup.

Current mismatch inventory totals:

- `P0 runtime-parity-bug: 6`
- `P1 stale-snapshot-expectation: 5`
- `P2 harness-debt: 0`

`tests/test_stdlib_system.kujo` now classifies as `both_match_snapshot`.

## Validation

All commands passed:

- `cargo fmt --check`
- `cargo test --test cli_contracts`
- `cargo test --test docs_examples`
- `cargo run -- test --runtime vm`
- `cargo run -- test --runtime dual`
- `bash scripts/generate_vm_runtime_mismatch_inventory.sh`
- `cargo test --test vm_runtime_mismatch_inventory_contract`
- `cargo test --test generated_artifact_freshness_contract`

