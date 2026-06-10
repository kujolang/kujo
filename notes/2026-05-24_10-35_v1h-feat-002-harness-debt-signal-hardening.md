# V1H-FEAT-002 — Harness-Debt Signal Hardening

Date: 2026-05-24
Owner: Codex agent loop

## Summary

Completed `V1H-FEAT-002` by hardening mismatch triage classification so rows where VM/interpreter outputs diverge from each other are treated as `runtime-parity-bug` instead of `harness-debt`.

This keeps the inventory actionable: runtime divergence is tracked as runtime work, while harness debt is reserved for true fixture-contract noise.

## What changed

- Updated `scripts/generate_vm_runtime_mismatch_inventory.sh`:
  - `both_mismatch_different_output` now maps to:
    - `runtime-parity-bug|runtime-owner|P0|both runtimes diverge from snapshot and from each other, indicating runtime-path parity drift rather than stale fixture expectations`
- Added regression contract test in `tests/vm_runtime_mismatch_baseline_contract.rs`:
  - `vm_runtime_mismatch_baseline_does_not_bucket_runtime_divergence_as_harness_debt`

## Evidence before/after

Before regeneration (prior baseline):
- `P0 runtime-parity-bug`: `21`
- `P2 harness-debt`: `16`

After regeneration:
- `P0 runtime-parity-bug`: `40`
- `P2 harness-debt`: `0`

Interpretation:
- The 16 former `harness-debt` rows were not stale-contract-only noise; they represented runtime divergence classes (env/stdlib/image and method/self/struct/diagnostic families) and are now surfaced in the correct parity bucket.

## Commands run

- `bash scripts/generate_vm_runtime_mismatch_inventory.sh`
- `cargo test --test vm_runtime_mismatch_inventory_contract`
- `cargo test --test vm_runtime_mismatch_baseline_contract`
- `cargo test --test vm_interpreter_parity_surfaces`
- `cargo run -- test --runtime vm`
- `cargo run -- test --runtime dual`

## Results

- Inventory regenerated successfully with updated bucket totals.
- Contract tests passed for inventory generation and baseline classification checks.
- VM/interpreter parity surface test passed.
- VM/dual runtime sweeps completed with known pre-existing non-green baseline fixtures; no new harness-classification regressions observed.

## Residual risk and follow-up

- Risk: runtime-parity backlog is now more explicit (`P0: 40`), so burn-down pressure shifts to runtime parity loops.
- Follow-up: drive `V1H-FEAT-001`/`V1VM-PAR-*` parity closures fixture-family-by-family using the newly clarified inventory signal.
