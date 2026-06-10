# Kujo Field Notes — V1-BASE-002 release-gate script follow-through

**Date:** 2026-05-07
**Session:** 07:36 local
**Branch/Commit:** main / pending
**Scope:** Completed `V1-BASE-002` by hardening `scripts/release_gate.sh` with explicit full/minimal modes, adding lightweight CI smoke execution, and aligning release-gate docs and roadmap status.

---

## What I Changed
- Updated `scripts/release_gate.sh` to support `--full` (default) and `--minimal`.
- Enforced full-mode command order: `cargo fmt --check`, `cargo clippy`, `cargo test`, selected integration tests, Kujo self-test command, optional `cargo audit`/`cargo deny`.
- Added optional full-mode benchmark smoke toggle: `KUJO_RELEASE_GATE_RUN_BENCH=1`.
- Added CI lightweight script execution job `release-gate-minimal-smoke` in `.github/workflows/ci-release-gate.yml`.
- Updated release-gate usage docs in `README.md` and `docs/RELEASE_PROCESS.md` with prerequisites and runtime expectations.
- Marked `V1-BASE-002` complete in `ROADMAP.md` with verification evidence.
- Hardened a deterministic test blocker in `src/interpreter/native_functions/mod.rs` by using a unique per-run temp directory for `test_release_hardening_env_os_path_and_assert_contracts`.

## Gotchas (Read This Next Time)
- **Gotcha:** `cargo test` can fail on persistent temp-directory state even when feature work is unrelated.
  - **Symptom:** `test_release_hardening_env_os_path_and_assert_contracts` failed because `os_rmdir` returned false.
  - **Root cause:** The test reused a fixed path (`tmp/release_hardening_os_path_contract`) that could contain stale nested content from prior runs.
  - **Fix:** Switched the test to a unique per-run directory name derived from PID + timestamp, with stale-dir pre-clean.
  - **Prevention:** Avoid fixed shared temp paths in integration-style native API tests; prefer unique run-scoped paths.

## Things I Learned
- Release-gate script ergonomics need two tracks: strict full gate for release confidence and lightweight minimal mode for deterministic CI script smoke.
- Full mode should stay fail-fast and explicit; optional checks (`cargo audit`, `cargo deny`, benchmark smoke) should be opt-in/auto-detected rather than silently ignored via shell shortcuts.
- Existing repository-level fmt drift can make a full gate fail immediately; minimal smoke mode is valuable for proving script wiring independently.

## Debug Notes (Only if applicable)
- **Failing test / error:** `assertion failed: matches!(os_rmdir_result, Value::Bool(true))` in `test_release_hardening_env_os_path_and_assert_contracts`.
- **Repro steps:** `cargo test` and `cargo test interpreter::native_functions::tests::test_release_hardening_env_os_path_and_assert_contracts`.
- **Breakpoints / logs used:** Inspected `src/interpreter/native_functions/mod.rs` test body and listed on-disk temp path contents.
- **Final diagnosis:** Reused fixed temp directory caused stale nested directory content and deterministic `os_rmdir` failure.

## Follow-ups / TODO (For Future Agents)
- [ ] If `cargo fmt --check` is intended to pass in this branch, run `cargo fmt` in a dedicated formatting follow-up and validate CI impact.
- [ ] Consider moving release-gate mode behavior into a dedicated docs table once more gate toggles are added.

## Links / References
- Files touched:
  - `scripts/release_gate.sh`
  - `.github/workflows/ci-release-gate.yml`
  - `README.md`
  - `docs/RELEASE_PROCESS.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `src/interpreter/native_functions/mod.rs`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `README.md`
  - `ROADMAP.md`
