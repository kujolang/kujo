# Kujo Field Notes — V1U-CODE-002 Runtime TODO Deferrals

**Date:** 2026-05-20
**Session:** 18:09 local
**Branch/Commit:** main / dde6d30
**Scope:** Closed `V1U-CODE-002` by converting high-risk runtime TODO markers into explicit post-v1 deferment notes, documenting deferment guardrails in `docs/V1_SCOPE.md`, and regenerating strict TODO triage artifacts.

---

## What I Changed
- Updated runtime-path comments to remove ambiguous high-risk TODO markers and replace them with explicit deferment notes:
  - `src/vm.rs`
  - `src/compiler.rs`
  - `src/interpreter/native_functions/async_ops.rs`
- Added `Deferred Runtime Execution Backlog (Explicit v1 Deferrals)` section to `docs/V1_SCOPE.md`.
- Regenerated strict triage artifacts:
  - `docs/generated/V1_CODE_TODO_TRIAGE.md`
  - `docs/generated/V1_CODE_TODO_TRIAGE.csv`
- Updated checklist evidence and triage contracts:
  - `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`
  - `tests/v1_code_todo_triage_contract.rs`

## Gotchas (Read This Next Time)
- **Gotcha:** `TODO` markers in runtime files can be interpreted as unresolved release blockers even when behavior is intentionally deferred.
  - **Symptom:** Triaged runtime debt kept surfacing as high-severity TODO backlog.
  - **Root cause:** Comments were open-ended TODOs instead of explicit deferment statements tied to release-scope docs.
  - **Fix:** Replace high-risk TODO markers with explicit post-v1 deferment comments and cross-link scope docs.
  - **Prevention:** For runtime-risk backlog in release windows, use explicit deferment comments + scope-doc references instead of bare TODO markers.

## Things I Learned
- The strict triage script becomes a practical release signal only when high-risk runtime comments are either implemented or explicitly deferred.
- Scope docs need concrete backlog entries, not just generic “post-v1 candidates,” to keep triage/action workflows unambiguous.

## Debug Notes (Only if applicable)
- **Failing test / error:** `v1_code_todo_triage_script_is_deterministic_for_repo_scan` initially failed.
- **Repro steps:** Run `cargo test --test v1_code_todo_triage_contract` before sorting scan output.
- **Breakpoints / logs used:** Compared two generated markdown outputs from back-to-back script runs.
- **Final diagnosis:** `rg` result ordering was nondeterministic; sorting scan lines before classification fixed output drift.

## Follow-ups / TODO (For Future Agents)
- [ ] If any deferred runtime item moves into v1 scope, replace deferment comments with implementation plus targeted runtime/parity/security tests.
- [ ] Keep `docs/generated/V1_CODE_TODO_TRIAGE.*` refreshed when runtime TODO/FIXME/HACK markers change.

## Links / References
- Files touched:
  - `src/vm.rs`
  - `src/compiler.rs`
  - `src/interpreter/native_functions/async_ops.rs`
  - `docs/V1_SCOPE.md`
  - `docs/generated/V1_CODE_TODO_TRIAGE.md`
  - `docs/generated/V1_CODE_TODO_TRIAGE.csv`
  - `tests/v1_code_todo_triage_contract.rs`
  - `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`
- Related docs:
  - `docs/V1_SCOPE.md`
  - `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`
