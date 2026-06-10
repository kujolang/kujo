# Kujo Field Notes — PREV1-REL-001 RC gate full-run evidence

**Date:** 2026-05-20
**Session:** 09:07 local (EDT)
**Branch/Commit:** main / bcbb58f
**Scope:** Executed the pre-v1 full release-candidate gate command and captured deterministic pass/fail evidence for checklist item `PREV1-REL-001`.

---

## What I Changed
- Ran `bash scripts/release_candidate_gate.sh --full` from repo root.
- Captured outcome and failure surface for release checklist evidence.

## Gotchas (Read This Next Time)
- **Gotcha:** Full RC gate can fail immediately on formatting drift before reaching deeper runtime/integration checks.
  - **Symptom:** `release_candidate_gate.sh --full` exited non-zero during `cargo fmt --check`.
  - **Root cause:** Existing formatting drift in tracked Rust sources/tests (including docgen adapters and diagnostics/runtime security test files) relative to `rustfmt` style.
  - **Fix:** Align tracked sources with repo formatting policy before re-running the full gate.
  - **Prevention:** Run `cargo fmt --check` as a preflight before invoking the full RC gate.

## Things I Learned
- The `PREV1-REL-001` evidence requirement is satisfied by recording exact command outcomes (including failures) with concrete classification.
- This run was a deterministic formatting-gate failure, not socket/timing instability.

## Debug Notes (Only if applicable)
- **Failing command / error:** `bash scripts/release_candidate_gate.sh --full`
- **Repro steps:** Run the command at repo root on `main` with current working tree.
- **Final diagnosis:** Gate failed at `cargo fmt --check` due formatting diffs; no flaky environment condition was required to reproduce.

## Follow-ups / TODO (For Future Agents)
- [x] Re-run formatting preflight and resolve drift (`cargo fmt` + `cargo fmt --check`). (Completed 2026-05-20, `V1U-GATE-001`)
- [ ] Keep classifying future gate failures as deterministic vs environment instability for release sign-off clarity.

## 2026-05-20 Follow-up Update (`V1U-GATE-001`)

- Ran `cargo fmt` to apply repository formatting policy.
- Re-ran `cargo fmt --check` and confirmed clean pass (no diff output).
- Remaining `rustfmt` unstable-option warnings are now tracked separately under `V1U-GATE-002` policy resolution work.

## 2026-05-20 Follow-up Update (`V1U-GATE-002`)

- Simplified `rustfmt.toml` to stable-only options (removed unstable `normalize_comments`, `wrap_comments`, and `group_imports` keys).
- Re-ran `cargo fmt` and `cargo fmt --check`; formatting checks now run without unstable-option warning noise.

## Links / References
- Files touched:
  - `notes/2026-05-20_09-07_prev1-rel-001-rc-gate-evidence.md`
- Related docs:
  - `docs/PRE_V1_ACTION_CHECKLIST.md`
  - `docs/RELEASE_PROCESS.md`
