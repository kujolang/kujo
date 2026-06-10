# V1U-RUN-003 — `kujo test` Runtime Strategy (VM-first + Bounded Fallback)

Date: 2026-05-20

## Scope

Implement a VM-first or dual-engine `kujo test` strategy that reduces blanket interpreter
reliance while preserving deterministic fixture behavior.

## What Changed

1. Added `kujo test --runtime dual|vm|interpreter` (default: `dual`).
2. Updated `src/parser.rs::run_all_tests` to support explicit runtime strategy.
3. Implemented bounded fallback policy in `dual` mode:
   - run VM first,
   - if VM output matches snapshot, pass as VM-primary,
   - if not, run interpreter once,
   - pass only if interpreter output matches snapshot,
   - otherwise fail and print both outputs for triage.
4. Added runtime-strategy summary counters in command output:
   - `vm_primary`,
   - `interpreter_fallback` (dual),
   - `interpreter_primary` (interpreter mode).

## Fallback Boundaries

- Fallback exists only in `dual` mode and only on VM snapshot mismatch.
- `vm` mode never falls back.
- `interpreter` mode never runs VM.

## Test Coverage Added

- `cli_test_runtime_vm_mode_reports_mismatch_for_vm_drift_fixture`
- `cli_test_runtime_dual_mode_falls_back_to_interpreter_for_vm_drift_fixture`

Both tests use a deterministic drift fixture where VM and interpreter stdout differ, proving:

- failure path in VM-only mode,
- success path via bounded interpreter fallback in dual mode.

## Validation Commands

```bash
cargo test --test cli_contracts
cargo test --test interpreter_flag_dependency_map_contract
cargo test --test readme_contracts
cargo test --test vm_interpreter_parity_surfaces
```

All commands passed in this loop.
