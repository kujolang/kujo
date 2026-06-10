# Snapshot Parity Zero-Mismatch Follow-Through

Date: 2026-05-25
Owner: runtime/docs

## Goal

Close remaining `stale-snapshot-expectation` drift after runtime-parity-bug closure and verify mismatch contracts still hold when mismatch count reaches zero.

## Changes

- Refreshed fixture snapshots via VM runtime harness update flow:
  - `cargo run -- test --runtime vm --update`
- Hardened deterministic fixture output for OS/path surface:
  - `tests/stdlib_os_path_test.kujo` now avoids environment-specific output text in snapshot assertions.
- Updated inventory contract tests to allow zero-mismatch baselines without false failures:
  - `tests/vm_runtime_mismatch_inventory_contract.rs`
  - `tests/vm_runtime_mismatch_baseline_contract.rs`

## Commands and Results

- `bash scripts/generate_vm_runtime_mismatch_inventory.sh` -> success
- `cargo test --test vm_runtime_mismatch_inventory_contract` -> `2 passed`
- `cargo test --test vm_runtime_mismatch_baseline_contract` -> `4 passed`
- `cargo test --test vm_interpreter_parity_surfaces` -> `96 passed`
- `cargo run -- test --runtime vm` -> `Passed 137/150`
- `cargo run -- test --runtime dual` -> `Passed 137/150` (`vm_primary=137`, `interpreter_fallback=0`)

## Outcome

Generated inventory now reports zero unexplained and zero stale mismatch buckets:

- `both mismatch: 0`
- `P0 runtime-parity-bug: 0`
- `P1 stale-snapshot-expectation: 0`
- `P2 harness-debt: 0`

Remaining fixture failures in `kujo test` are parser/language-coverage fixtures already classified as matching baseline snapshots.
