# Kujo Field Notes — Cross-Language And Release Checks Local Evidence

**Date:** 2026-04-29
**Session:** 21:52 local
**Branch/Commit:** main / ed77acd
**Scope:** Executed the remaining v0.11.0 P0 release-checklist commands (cross-language benchmark context plus focused regression and dispatch-hardening tests), then documented local evidence and release-exception stance so release metadata work can proceed without waiting on idle-machine access.

---

## What I Changed
- Ran cross-language SSG benchmark command:
  - `./target/release/kujo bench-ssg --runs 5 --warmup-runs 1 --compare-python --profile-async --tmp-dir tmp/kujo-v0.11-ssg-python`
- Captured host/run context (`date`, `uptime`, commit SHA).
- Ran focused release-checklist tests:
  - `cargo test ssg`
  - `cargo test bench_ssg`
  - `cargo test run_ssg_benchmark`
  - `cargo test release_hardening_builtin_dispatch_coverage`
  - `cargo test test_release_hardening_ssg_render_pages_dispatch_contracts`
- Updated `ROADMAP.md` with cross-language evidence, test results, dispatch-hardening results, and an explicit non-blocking release-exception stance.
- Updated `CHANGELOG.md` with release-evidence/status documentation for this checkpoint.

## Gotchas (Read This Next Time)
- **Gotcha:** `cargo test bench_ssg` can pass while matching zero tests.
  - **Symptom:** Command reports success with `running 0 tests`.
  - **Root cause:** Filter string does not match any test names even though benchmark harness tests exist under different names.
  - **Fix:** Keep command in checklist for consistency, but pair it with `cargo test run_ssg_benchmark` and `cargo test ssg` for real targeted coverage.
  - **Prevention:** Treat zero-match filtered test commands as execution sanity checks, not as sole coverage evidence.

- **Gotcha:** Cross-language comparison output may not print an explicit Python checksum line even when checksum validation is enforced.
  - **Symptom:** Output includes Kujo checksum and speedup stats but no visible Python checksum field.
  - **Root cause:** Harness validates checksum compatibility internally and exits non-zero on mismatch.
  - **Fix:** Treat successful command completion as checksum-match evidence and explicitly document that no mismatch error occurred.
  - **Prevention:** In release notes, state checksum status explicitly from command success/failure semantics.

## Things I Learned
- Trend and measurement warnings in local cross-language runs are common on a loaded host and should be documented as release-relevant noise signals, not ignored.
- Deterministic test/dispatch coverage can provide strong release confidence when final idle-machine benchmark evidence is temporarily unavailable.
- Explicitly recording the release-exception stance in roadmap/changelog prevents future agents from reopening the same decision loop.

## Debug Notes (Only if applicable)
- **Failing test / error:** None.
- **Repro steps:** Run the P0 checklist commands listed above from repo root.
- **Breakpoints / logs used:** Direct CLI output from benchmark and cargo test commands.
- **Final diagnosis:** Local validation is green for focused correctness and dispatch-hardening; benchmark context remains local smoke evidence due host load/noise.

## Follow-ups / TODO (For Future Agents)
- [ ] After v0.11.0 release metadata is cut, capture one canonical idle-machine benchmark snapshot for post-release archival evidence.
- [ ] Optionally replace or augment `cargo test bench_ssg` with an explicit test filter that always matches at least one benchmark harness test.

## Links / References
- Files touched:
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/2026-04-29_21-52_cross-language-and-release-checks-local-evidence.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `notes/2026-04-29_21-25_release-mode-ssg-gate-local-smoke-followup.md`
  - `ROADMAP.md`
