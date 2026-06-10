# Kujo Field Notes — Scheduler Timeout CLI Override Precedence

**Date:** 2026-04-29
**Session:** 21:00 local
**Branch/Commit:** main / fc20c73
**Scope:** Added a first-class CLI timeout override for cooperative VM scheduler completion in `kujo run` and documented the release-roadmap follow-through. Added focused timeout-resolution tests for precedence and validation behavior.

---

## What I Changed
- Added `--scheduler-timeout-ms` to the `Run` CLI subcommand in `src/main.rs`.
- Refactored timeout resolution into `cooperative_scheduler_timeout(cli_timeout_ms)` with explicit precedence: CLI, then `KUJO_SCHEDULER_TIMEOUT_MS`, then default (`120000`).
- Added `src/main.rs` unit tests for default fallback, env override, CLI-over-env precedence, invalid env fallback, and zero CLI rejection.
- Updated release/docs surfaces in `CHANGELOG.md`, `ROADMAP.md`, and `README.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** Timeout resolution behavior can silently drift if CLI/env/default precedence is implicit.
  - **Symptom:** It becomes unclear whether benchmark/repro runs are using shell environment values or command-line overrides.
  - **Root cause:** Timeout selection lived as env-only behavior without a first-class `run` command override surface.
  - **Fix:** Introduced explicit `cooperative_scheduler_timeout(...)` resolution with deterministic precedence and tests.
  - **Prevention:** Keep precedence logic centralized in one helper and lock it with tests before adding new timeout config surfaces.

## Things I Learned
- The highest-priority incomplete roadmap item that required code (not operator execution) was the scheduler timeout CLI decision under `v0.11.0` release evidence/documentation work.
- `src/main.rs` is already heavily unit-tested, so adding CLI/config contract tests there is the lowest-friction way to guard behavior.
- Environment-variable tests need a global mutex to avoid test flakiness from process-global env mutation.

## Debug Notes (Only if applicable)
- **Failing test / error:** None.
- **Repro steps:** N/A.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** N/A.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider adding explicit clap-level range validation (`>= 1`) for `--scheduler-timeout-ms` if CLI UX should fail before execution-path handling.
- [ ] If `bench-ssg` needs scheduler reproducibility parity, evaluate whether scheduler timeout should also be surfaced there or remain scoped to `run`.

## Links / References
- Files touched:
  - `src/main.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/README.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `.github/AGENT_INSTRUCTIONS.md`
