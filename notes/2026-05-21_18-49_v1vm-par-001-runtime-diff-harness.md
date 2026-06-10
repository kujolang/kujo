# Kujo Field Notes — V1VM-PAR-001 Runtime Diff Harness

**Date:** 2026-05-21
**Session:** 18:49 local
**Branch/Commit:** main / 3de7567
**Scope:** Implemented a deterministic VM-vs-interpreter runtime diff harness with output normalization and contract coverage. Closed `V1VM-PAR-001` with required runtime sweep evidence.

---

## What I Changed
- Added `scripts/generate_vm_runtime_diff_harness.sh`.
- Added `tests/vm_runtime_diff_harness_contract.rs`.
- Updated `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md` to mark `V1VM-PAR-001` complete with dated evidence.
- Generated harness artifacts:
  - `docs/generated/VM_RUNTIME_DIFF_HARNESS.md`
  - `docs/generated/VM_RUNTIME_DIFF_HARNESS.csv`

## Gotchas (Read This Next Time)
- **Gotcha:** VM/dual runtime sweeps in this parity phase are evidence-gathering commands, not expected to reach 150/150 yet.
  - **Symptom:** `cargo run -- test --runtime vm` and `cargo run -- test --runtime dual` report many fixture mismatches.
  - **Root cause:** Current roadmap phase is parity burn-down; mismatch buckets are intentionally tracked and reduced incrementally.
  - **Fix:** Capture exact totals and mismatch class evidence, then prioritize top buckets in `V1VM-PAR-002` and `V1VM-PAR-003`.
  - **Prevention:** Do not block `V1VM-PAR-001` closure on full parity; require deterministic harness + contract + sweep evidence instead.

## Things I Learned
- Normalizing runtime diagnostics into a shared shape is enough to classify "noise-only" vs "semantic drift" deltas without changing runtime behavior.
- A built-in self-check mode in the harness catches normalization-rule regressions before large fixture sweeps run.
- The loop is easier to audit when checklist evidence includes command totals directly (`59/150`, `78/150`).

## Debug Notes (Only if applicable)
- **Failing test / error:** Initial shell-escape bug during harness script authoring (`\`` over-escaping).
- **Repro steps:** Run `bash scripts/generate_vm_runtime_diff_harness.sh` while script includes escaped backticks in markdown table header construction.
- **Breakpoints / logs used:** Direct shell run plus minimal fixture run with `--max-fixtures 6`.
- **Final diagnosis:** Shell literal escaping was stricter than needed; plain backtick literals in double-quoted `echo` lines resolved generation correctly.

## Follow-ups / TODO (For Future Agents)
- [ ] Execute `V1VM-PAR-002` against highest-volume mismatch class from the new harness artifacts.
- [ ] Keep regenerating harness artifacts after each parity bucket fix to enforce monotonic mismatch reduction.

## Links / References
- Files touched:
  - `scripts/generate_vm_runtime_diff_harness.sh`
  - `tests/vm_runtime_diff_harness_contract.rs`
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`
  - `notes/2026-05-21_18-49_v1vm-par-001-runtime-diff-harness.md`
- Related docs:
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`
  - `docs/generated/VM_RUNTIME_DIFF_HARNESS.md`
  - `docs/generated/VM_RUNTIME_DIFF_HARNESS.csv`
