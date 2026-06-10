# 2026-05-26 — V1X-DRY-001 legacy runtime mirror removal

## Summary

Closed `V1X-DRY-001` by removing `src/interpreter/legacy_full.rs` from the active source tree and updating triage generation so mirror-runtime debt is no longer tracked as an active production-path ambiguity.

## Commands run

1. `bash scripts/generate_v1_code_todo_triage.sh --strict`
2. `cargo test --test v1_code_todo_triage_contract`
3. `cargo test --test vm_interpreter_parity_surfaces`
4. `cargo test`

## Results

- `generate_v1_code_todo_triage.sh --strict`: pass; regenerated markdown/csv artifacts.
- `cargo test --test v1_code_todo_triage_contract`: pass (`3 passed`).
- `cargo test --test vm_interpreter_parity_surfaces`: pass (`100 passed`).
- `cargo test`: pass.
  - One run observed a transient `docgen_universal` external-validation test failure.
  - Isolated rerun of the same test passed, and a subsequent full `cargo test` run passed fully.

## Artifact deltas

- `docs/generated/V1_CODE_TODO_TRIAGE.md`
- `docs/generated/V1_CODE_TODO_TRIAGE.csv`

Current summary: `30` markers triaged, `0` unclassified.

## Residual risk

- Existing large runtime files (`src/jit.rs`, `src/vm.rs`, `src/interpreter/mod.rs`) remain complexity hotspots and are tracked by separate checklist items.
