# Kujo Field Notes - Checklist Evidence Refresh

**Date:** 2026-06-08
**Session:** 22:26
**Branch/Commit:** main / uncommitted
**Scope:** Refreshed the V1.0 universal usefulness checklist evidence snapshot so its headline counts match the current repository state.

---

## What I Changed
- Re-measured the generated TODO triage and unsafe inventory artifacts from the live tree.
- Regenerated `docs/generated/V1_CODE_TODO_TRIAGE.md` and `docs/generated/V1_CODE_TODO_TRIAGE.csv` so the generated triage artifact now reports the current marker count.
- Regenerated `docs/generated/UNSAFE_INVENTORY.md` and `docs/generated/UNSAFE_INVENTORY.csv` so the unsafe inventory artifact date matches the current measurement pass.
- Updated `docs/V1_0_UNIVERSAL_USEFULNESS_EXPANSION_CHECKLIST.md` to reflect the current counts and current line counts for the large hotspots.
- Added a new session note index entry in `notes/README.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** Generated evidence artifacts can drift even when the surrounding checklist text looks unchanged.
  - **Symptom:** The checklist still said `30` TODO markers even though the generator now reported `29`.
  - **Root cause:** The underlying tree changed after the last evidence pass, but the generated docs and the checklist snapshot had not been refreshed together.
  - **Fix:** Regenerate the source-of-truth artifacts first, then update the checklist prose to match the regenerated outputs.
  - **Prevention:** When a checklist cites generated counts, treat the generated docs and the checklist as a single consistency unit.

## Things I Learned
- `src/jit.rs`, `src/vm.rs`, and `src/interpreter/mod.rs` all grew since the last snapshot, so hard-coded LOC figures in evidence sections need periodic refreshes.
- The dependency-footprint note should describe the current feature partitioning, not just the fact that the repository still carries large runtime stacks.

## Follow-ups / TODO (For Future Agents)
- [ ] Refresh the evidence snapshot again after any future `src/` churn that affects generated counts or hot-path line totals.

## Links / References
- Files touched:
  - `docs/V1_0_UNIVERSAL_USEFULNESS_EXPANSION_CHECKLIST.md`
  - `docs/generated/V1_CODE_TODO_TRIAGE.md`
  - `docs/generated/V1_CODE_TODO_TRIAGE.csv`
  - `docs/generated/UNSAFE_INVENTORY.md`
  - `docs/generated/UNSAFE_INVENTORY.csv`
  - `notes/README.md`
- Related docs:
  - `docs/REPO_HYGIENE_POLICY.md`
  - `tests/repo_hygiene_contract.rs`
