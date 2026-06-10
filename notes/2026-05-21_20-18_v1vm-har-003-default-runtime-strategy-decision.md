# V1VM-HAR-003 — Default `kujo test` Runtime Strategy Reassessment

Date: 2026-05-21
Owner: codex agent

## Decision

Keep the default `kujo test` runtime at `dual` for now.

## Why

- VM-only test runtime still trails dual pass-rate (`107/150` vs `121/150`).
- Dual mode remains deterministic and bounded after HAR-001 (`[dual fallback: interpreter]` markers + fallback counters).
- Remaining unresolved mismatch categories (`runtime-parity-bug`, `harness-debt`) indicate VM-only default would create avoidable default-user failures before parity closure.

## Evidence

- `cargo run -- test --runtime vm` -> `Passed 107/150`
- `cargo run -- test --runtime dual` -> `Passed 121/150` (`vm_primary=107`, `interpreter_fallback=14`)
- `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` still contains unresolved non-intentional buckets.

## Contracts updated

- `tests/cli_contracts.rs`: default `kujo test` now explicitly asserts dual runtime summary and zero fallback on VM-clean fixture.
- `tests/runtime_path_matrix_contract.rs`: verifies decision section remains documented in parity matrix.

## Risk posture

- Short-term: dual default prevents user-visible regressions while preserving strict VM option (`--runtime vm`).
- Exit criterion for future default flip: close unresolved mismatch buckets and satisfy intentional-divergence-only requirement (`V1VM-PAR-004`).
