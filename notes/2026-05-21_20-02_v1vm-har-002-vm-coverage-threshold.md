# Kujo Field Notes — V1VM-HAR-002 VM Coverage Threshold

**Date:** 2026-05-21
**Session:** 20:02 local
**Branch/Commit:** main / fe1a969
**Scope:** Added machine-verifiable VM coverage gate reporting to mismatch artifacts and closed HAR-002 with threshold + trend evidence.

---

## What I Changed
- Updated `scripts/generate_vm_runtime_mismatch_inventory.sh` to append a VM coverage gate section:
  - metric: `vm_matches_snapshot / fixtures_scanned`
  - target threshold: `70.0%`
  - computed percentage and PASS/FAIL status
- Updated `tests/vm_runtime_mismatch_inventory_contract.rs` to enforce coverage-gate markers in generated markdown.
- Regenerated `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.{md,csv}` with new gate output.
- Updated `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md` and marked `V1VM-HAR-002` complete with trend + threshold evidence.

## Gotchas (Read This Next Time)
- **Gotcha:** Inventory fixture count (`163`) differs from `kujo test` top-level suite count (`150`).
  - **Symptom:** Coverage ratio in inventory and VM runtime pass totals are both valid but based on different fixture universes.
  - **Root cause:** Inventory script scans recursively (`rg --files tests -g '*.kujo'`), while `kujo test` currently iterates top-level `tests/` entries.
  - **Fix:** Report both metrics explicitly and use each for its intended purpose.
  - **Prevention:** When using trend evidence, label whether numbers are inventory-wide or runtime-suite-only.

## Things I Learned
- The coverage gate belongs in generated artifacts, not ad hoc notes, so threshold drift is machine-detectable.
- A single threshold line plus PASS/FAIL status materially simplifies readiness reviews.
- Runtime progress is easier to communicate when combining percentage gates with `vm_primary`/fallback trend deltas.

## Debug Notes (Only if applicable)
- **Failing test / error:** None; this loop was reporting/contract hardening.
- **Repro steps:** Generate inventory and inspect tail section for gate lines.
- **Breakpoints / logs used:** `tail -n 20 docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`, contract test output, vm/dual sweep summaries.
- **Final diagnosis:** Coverage threshold can be tracked reliably from generated inventory output.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue `V1VM-HAR-003` default-runtime reassessment once parity/harness buckets drop further.
- [ ] Consider converging inventory/test fixture universes if one canonical denominator is required for release sign-off.

## Links / References
- Files touched:
  - `scripts/generate_vm_runtime_mismatch_inventory.sh`
  - `tests/vm_runtime_mismatch_inventory_contract.rs`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv`
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`
  - `notes/2026-05-21_20-02_v1vm-har-002-vm-coverage-threshold.md`
- Related docs:
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
