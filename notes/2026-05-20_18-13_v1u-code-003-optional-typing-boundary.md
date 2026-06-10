# Kujo Field Notes — V1U-CODE-003 Optional Typing Boundary Verification

**Date:** 2026-05-20
**Session:** 18:13 local
**Branch/Commit:** main / 0610965
**Scope:** Verified and locked optional-typing non-enforcement boundaries by adding runtime-path contract coverage and aligning scope/design docs with current interpreter vs VM behavior.

---

## What I Changed
- Added interpreter-vs-VM boundary contract test in `tests/optional_typing_v1_contract.rs`:
  - `v1_optional_typing_warnings_are_interpreter_only`
- Updated docs to keep optional-typing boundary explicit and consistent:
  - `docs/OPTIONAL_TYPING_DESIGN.md`
  - `docs/V1_SCOPE.md`
- Updated checklist closure evidence in `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** Optional typing warnings are surfaced on interpreter CLI path only; VM path does not run the same warning pass.
  - **Symptom:** Users can see warning output in `kujo run --interpreter` but no equivalent warning header in default VM mode.
  - **Root cause:** Type-check pass is currently wired in interpreter fallback branch in `src/main.rs`, not in default VM execution branch.
  - **Fix:** Document this as an explicit boundary and lock it with a test that runs both modes.
  - **Prevention:** Any future VM-side type-gate work should be introduced as explicit contract changes with docs + tests, not as implicit behavior drift.

## Things I Learned
- Boundary clarity for optional typing requires both policy docs and executable contract tests across runtime modes.
- The most stable contract marker for current behavior is the presence/absence of the `Type checking warnings:` stderr header.

## Debug Notes (Only if applicable)
- **Failing test / error:** None.
- **Repro steps:** N/A.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** Current behavior is intentional and now explicitly contract-locked.

## Follow-ups / TODO (For Future Agents)
- [ ] If VM-side type-checking is introduced, update optional typing docs and add explicit migration guidance.
- [ ] Keep interpreter warning wording stable or update contract assertions intentionally.

## Links / References
- Files touched:
  - `tests/optional_typing_v1_contract.rs`
  - `docs/OPTIONAL_TYPING_DESIGN.md`
  - `docs/V1_SCOPE.md`
  - `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`
- Related docs:
  - `docs/OPTIONAL_TYPING_DESIGN.md`
  - `docs/V1_SCOPE.md`
  - `README.md`
