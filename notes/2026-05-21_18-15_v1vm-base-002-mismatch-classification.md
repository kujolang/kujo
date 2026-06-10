# Kujo Field Notes — V1VM-BASE-002 Mismatch Classification

**Date:** 2026-05-21
**Session:** 18:15 local
**Branch/Commit:** main / bde6f2e
**Scope:** Added deterministic mismatch-cause classification (bucket + owner + priority + rationale) for VM-vs-interpreter fixture baseline outputs.

---

## What I Changed
- Extended `scripts/generate_vm_runtime_mismatch_inventory.sh` to classify each fixture row with:
  - `mismatch_bucket`
  - `bucket_owner`
  - `priority`
  - `rationale`
- Added ordered mismatch classification summary totals in markdown output.
- Updated `tests/vm_runtime_mismatch_inventory_contract.rs` to enforce new output columns and classified-mismatch presence.
- Regenerated:
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv`

## Gotchas (Read This Next Time)
- **Gotcha:** Mismatch classification should be based on deterministic observable output shape, not assumptions about fixture intent.
  - **Symptom:** Easy to over-assign causes without stable runtime signals.
  - **Root cause:** Fixtures can fail for different reasons while sharing similar names or domains.
  - **Fix:** Classify from explicit delta type + exit-code patterns first, then apply narrow named-surface overrides.
  - **Prevention:** Keep bucket logic deterministic and test-verified via generated output contracts.

## Things I Learned
- Current top bucket is `stale-snapshot-expectation`, followed by `runtime-parity-bug`.
- The classification table is enough to prioritize next loops by measurable impact.

## Debug Notes (Only if applicable)
- **Failing test / error:** N/A after loop-1 script fix.
- **Repro steps:** N/A.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** N/A.

## Follow-ups / TODO (For Future Agents)
- [ ] Use `runtime-parity-bug` fixtures as first target set for `V1VM-PAR-*` loops.
- [ ] Revisit `parser-invalid-fixture` only if future scans produce non-zero counts.

## Links / References
- Files touched:
  - `scripts/generate_vm_runtime_mismatch_inventory.sh`
  - `tests/vm_runtime_mismatch_inventory_contract.rs`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv`
- Related docs:
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`
  - `docs/VM_INTERPRETER_PARITY_MATRIX.md`

## Command Evidence
- `bash scripts/generate_vm_runtime_mismatch_inventory.sh`
  - Result: success
  - Mismatch classification totals:
    - P0 `runtime-parity-bug`: `30`
    - P1 `stale-snapshot-expectation`: `48`
    - P1 `parser-invalid-fixture`: `0`
    - P2 `harness-debt`: `16`
    - P2 `intentional-divergence`: `0`
- `cargo test --test vm_runtime_mismatch_inventory_contract`
  - Result: `2 passed; 0 failed`
