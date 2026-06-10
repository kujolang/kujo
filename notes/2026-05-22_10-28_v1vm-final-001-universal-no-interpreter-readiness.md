# V1VM-FINAL-001 — Universal No-`--interpreter` Readiness Verdict

Date: 2026-05-22
Owner: codex agent

## Verdict

- Universal no-`--interpreter` readiness: **NO-GO (for now)**.

## Why (short)

- Core dotted-module reliability and VM-first migration docs are in place.
- But `V1VM-PAR-004` remains blocked due unresolved non-intentional mismatch buckets (`runtime-parity-bug`, `harness-debt`), so intentional-divergence-only parity posture is not yet achieved.

## Evidence Table

| Check | Command | Result |
| --- | --- | --- |
| VM/interpreter parity suite | `cargo test --test vm_interpreter_parity_surfaces` | `86 passed` |
| VM fixture sweep | `cargo run -- test --runtime vm` | `Passed 104/150` (`vm_primary=104`) |
| Dual fixture sweep | `cargo run -- test --runtime dual` | `Passed 121/150` (`vm_primary=107`, `interpreter_fallback=14`) |
| Mismatch inventory contract | `cargo test --test vm_runtime_mismatch_inventory_contract` | `2 passed` |
| Perf regression comparison contract | `cargo test --test vm_import_heavy_perf_comparison_contract` | `1 passed` |
| Cache lookup perf contract | `cargo test --test vm_import_heavy_cache_lookup_contract` | `1 passed` |
| README guidance contract | `cargo test --test readme_contracts` | `1 passed` |
| Runtime matrix contract | `cargo test --test runtime_path_matrix_contract` | `1 passed` |
| Interpreter dependency map contract | `cargo test --test interpreter_flag_dependency_map_contract` | `2 passed` |
| Migration playbook contract | `cargo test --test vm_interpreter_migration_playbook_contract` | `1 passed` |

## Remaining Blockers

1. `V1VM-PAR-004` (blocked): mismatch inventory still contains unresolved non-intentional categories.
   - Current evidence: `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` shows `runtime-parity-bug: 25` and `harness-debt: 16`.

## Recommendation

- Keep VM-first guidance and migration playbook active.
- Continue parity-bucket burn-down loops until `V1VM-PAR-004` acceptance criteria are met, then re-run this final matrix for a fresh go/no-go decision.
