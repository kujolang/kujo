# Kujo Field Notes — V1VM-PAR-002 Highest-Volume Bucket Burn-Down

**Date:** 2026-05-21
**Session:** 19:10 local
**Branch/Commit:** main / ae8e66a
**Scope:** Closed the highest-volume baseline mismatch bucket by refreshing stale snapshot expectations and revalidating VM/dual runtime sweeps plus parity contracts.

---

## What I Changed
- Updated `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md` to mark `V1VM-PAR-002` complete with dated evidence.
- Refreshed 48 stale snapshot files (`tests/*.out`) identified by `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv` where bucket was `stale-snapshot-expectation`.
- Regenerated mismatch inventory artifacts:
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv`
- Verified bucket reduction and runtime pass-count improvements with required commands.

## Gotchas (Read This Next Time)
- **Gotcha:** `tests/*.out` snapshot files are ignored by default in `.gitignore`.
  - **Symptom:** Snapshot refresh appears successful locally, but `git status` shows only a tiny subset unless files are force-added.
  - **Root cause:** Repo ignore policy includes `/tests/*.out`; only previously force-tracked files show as normal modifications.
  - **Fix:** Force-add the targeted snapshot set for this loop so the burn-down is durable in version control.
  - **Prevention:** For any snapshot-contract bucket task, explicitly account for ignored snapshot paths during staging.

## Things I Learned
- The highest-volume baseline bucket (`stale-snapshot-expectation`) was test-contract drift, not runtime semantic drift; removing it significantly improved VM and dual pass totals without runtime code changes.
- Regenerating the mismatch inventory immediately after snapshot refresh is the fastest way to prove monotonic reduction (`48 -> 0`).
- Keeping bucket ownership explicit (`docs-owner` vs `runtime-owner`) helps avoid mixing parity-engineering work with snapshot-contract hygiene.

## Debug Notes (Only if applicable)
- **Failing test / error:** None in implementation logic; key operational issue was ignored snapshot files not appearing as tracked changes by default.
- **Repro steps:** Refresh stale `.out` files, then run `git status` without force-add.
- **Breakpoints / logs used:** `git status --short`, inventory summary tail, runtime sweep totals.
- **Final diagnosis:** Snapshot updates were correct on disk; missing VCS tracking was due to ignore rules.

## Follow-ups / TODO (For Future Agents)
- [ ] Execute `V1VM-PAR-003` against the next highest-volume mismatch bucket (currently `runtime-parity-bug` at 30).
- [ ] Keep using regenerated inventory artifacts as the canonical source of next bucket selection.

## Links / References
- Files touched:
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv`
  - `notes/2026-05-21_19-10_v1vm-par-002-highest-volume-bucket-burndown.md`
  - `tests/*.out` (stale snapshot fixture set)
- Related docs:
  - `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv`
