# Kujo Field Notes — Cooperative scheduler timeout-budget follow-through

**Date:** 2026-04-27
**Session:** 22:03 local
**Branch/Commit:** main / b26dac5
**Scope:** Replaced fixed-round cooperative scheduler completion in the CLI VM run path with timeout-budget completion to keep high-volume async workloads (including `bench-ssg`) from failing early. Added VM timeout-scheduler regression coverage and updated project docs.

---

## What I Changed
- Added `VM::run_scheduler_until_complete_with_timeout(Duration)` in `src/vm.rs`.
- Updated `src/main.rs` cooperative execution path to call timeout-based scheduler completion after suspension.
- Added `KUJO_SCHEDULER_TIMEOUT_MS` override support in `src/main.rs` with a default of `120000` ms.
- Added VM regression tests in `src/vm.rs` for:
  - timeout scheduler success with pending async contexts
  - zero-timeout validation
  - timeout exhaustion error behavior
- Updated `CHANGELOG.md`, `ROADMAP.md`, and `README.md` with the new v0.11 P0 milestone.

## Gotchas (Read This Next Time)
- **Gotcha:** Fixed scheduler-round budgets are brittle for long-running async workloads.
  - **Symptom:** `kujo bench-ssg --profile-async` failed with `Scheduler did not complete all contexts within 1000 rounds (1 pending)`.
  - **Root cause:** Round budget is not proportional to wall-clock async completion time; a single pending context can legitimately need more than 1000 scheduler rounds.
  - **Fix:** Added timeout-based completion (`run_scheduler_until_complete_with_timeout`) and switched CLI VM run path to timeout budgeting.
  - **Prevention:** For production async execution, prefer wall-clock timeout budgets over hardcoded round counts; keep round-budget API for deterministic unit tests.

- **Gotcha:** Full `cargo test` in sandbox can fail on TCP bind tests even when code is correct.
  - **Symptom:** `PermissionDenied` in `test_release_hardening_network_module_round_trip_behaviors`.
  - **Root cause:** Sandbox restrictions blocked ephemeral TCP listener binding during the full suite.
  - **Fix:** Re-ran full suite with elevated permissions.
  - **Prevention:** If network round-trip tests fail with permission errors, rerun test verification outside sandbox before diagnosing code regressions.

## Things I Learned
- Scheduler correctness for async-heavy scripts is better controlled by explicit timeout contracts than by fixed round counts.
- Keeping both scheduler APIs is useful: round-budget API for deterministic budget-exhaustion tests, timeout-budget API for real CLI workloads.
- `bench-ssg` remains a fast sanity check that cooperative scheduler changes did not regress high-volume async completion behavior.

## Debug Notes (Only if applicable)
- **Failing test / error:** `Scheduler did not complete all contexts within 1000 rounds (1 pending)` from `kujo bench-ssg --profile-async`.
- **Repro steps:**
  - `cargo run --quiet -- bench-ssg --profile-async`
- **Breakpoints / logs used:**
  - Reviewed `src/main.rs` cooperative scheduler call site and `src/vm.rs` scheduler loop.
  - Captured benchmark output before/after scheduler timeout-budget migration.
- **Final diagnosis:**
  - Execution path relied on fixed round budget unsuitable for long-running async workloads; timeout-budget completion resolves this without changing async API output contracts.

## Follow-ups / TODO (For Future Agents)
- [ ] Benchmark default timeout sensitivity on slower hardware and confirm whether `120000` ms should remain default.
- [ ] Consider exposing scheduler timeout as a first-class CLI flag in addition to env override for reproducible benchmark runs.

## Links / References
- Files touched:
  - `src/vm.rs`
  - `src/main.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `notes/2026-04-27_22-03_scheduler-timeout-budget-follow-through.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `ROADMAP.md`
