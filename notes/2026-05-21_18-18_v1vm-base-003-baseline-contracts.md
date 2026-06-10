# Kujo Field Notes — V1VM-BASE-003 Baseline Contracts

**Date:** 2026-05-21
**Session:** 18:18 local
**Branch/Commit:** main / 172007e
**Scope:** Added strict classification validation and baseline artifact contract coverage for VM runtime mismatch inventory outputs.

---

## What I Changed
- Added `--strict` mode to `scripts/generate_vm_runtime_mismatch_inventory.sh`.
- Implemented strict failure checks for missing mismatch classification fields.
- Added `tests/vm_runtime_mismatch_baseline_contract.rs` to enforce:
  - required markdown markers
  - required CSV columns
  - valid bucket/priority domain
  - non-empty classification fields for every mismatch row
  - strict-mode smoke success
- Updated `tests/vm_runtime_mismatch_inventory_contract.rs` to run script in strict mode.

## Gotchas (Read This Next Time)
- **Gotcha:** Artifact contracts should validate both structure and semantic invariants.
  - **Symptom:** A file can have the right header but still carry unclassified mismatch rows.
  - **Root cause:** Header-only checks miss semantic drift.
  - **Fix:** Assert mismatch-row invariants (`bucket != none`, `owner != n/a`, `priority != P4`).
  - **Prevention:** Keep strict mode in generator and test it directly in CI-focused contract suites.

## Things I Learned
- Strict-mode generators are useful guardrails before deeper parity work begins.
- Current baseline artifacts are now validated for both schema and mismatch-classification semantics.

## Debug Notes (Only if applicable)
- **Failing test / error:** N/A.
- **Repro steps:** N/A.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** N/A.

## Follow-ups / TODO (For Future Agents)
- [ ] Keep `--strict` enabled during future baseline regenerations.
- [ ] Extend semantic checks if new mismatch buckets are introduced.

## Links / References
- Files touched:
  - `scripts/generate_vm_runtime_mismatch_inventory.sh`
  - `tests/vm_runtime_mismatch_baseline_contract.rs`
  - `tests/vm_runtime_mismatch_inventory_contract.rs`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv`
- Related docs:
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`

## Command Evidence
- `bash scripts/generate_vm_runtime_mismatch_inventory.sh --strict`
  - Result: success
- `cargo test --test vm_runtime_mismatch_inventory_contract --test vm_runtime_mismatch_baseline_contract`
  - Result: `5 passed; 0 failed`
