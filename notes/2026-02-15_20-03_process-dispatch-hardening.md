# Kujo Field Notes — Process Dispatch Hardening Closure

**Date:** 2026-02-15
**Session:** 20:03 local
**Branch/Commit:** main / e503cbe
**Scope:** Closed the next v0.10.0 P1 release-hardening dispatch gap by adding modular native handlers for `spawn_process` and `pipe_commands`, with contract coverage and docs updates.

---

## What I Changed
- Added modular process dispatch handlers in `src/interpreter/native_functions/system.rs`:
  - `spawn_process(command_array)`
  - `pipe_commands(commands_array)`
- Preserved legacy-compatible argument validation and runtime error-object shape for spawn/pipeline failures.
- Added comprehensive native-function tests in `src/interpreter/native_functions/system.rs` for:
  - argument-shape rejection paths
  - `ProcessResult` struct-shape success behavior
  - single-command pipeline output behavior
- Updated release-hardening dispatcher contract tests in `src/interpreter/native_functions/mod.rs`:
  - Added process APIs to critical coverage list
  - Removed process APIs from expected known dispatch gaps
  - Added process argument-contract hardening test
- Updated milestone docs in `CHANGELOG.md`, `ROADMAP.md`, and `README.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** Running `cargo fmt` can reformat unrelated files during focused feature work.
  - **Symptom:** `git status` shows changes in files not touched by the feature (for example `async_ops.rs`, `json.rs`).
  - **Root cause:** Workspace-wide formatter pass applies style updates globally.
  - **Fix:** Revert unrelated formatting-only files and keep commits atomic.
  - **Prevention:** Check `git status --short` immediately after formatting and restore unrelated files before committing.

- **Gotcha:** Hardening known-gap tests require synchronized list updates when a dispatch gap is closed.
  - **Symptom:** Release-hardening drift test fails even though new handler exists.
  - **Root cause:** The expected known-gap list still includes APIs that are no longer gaps.
  - **Fix:** Remove migrated APIs from `expected_known_legacy_dispatch_gaps` and add them to critical coverage where appropriate.
  - **Prevention:** Treat implementation + hardening-list updates as one atomic change.

## Things I Learned
- The highest-priority incomplete v0.10.0 item is still iterative release hardening, and the safest execution unit is a single declared API cluster closure.
- The process APIs can be validated portably by invoking `std::env::current_exe()` with `--help`, avoiding dependency on platform-specific shell binaries.
- Current dispatch hardening strategy relies on both broad exhaustive probes and targeted high-risk API contract tests.

## Debug Notes (Only if applicable)
- **Failing test / error:** None; no functional regressions encountered.
- **Repro steps:** N/A.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** N/A.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue release-hardening follow-through by closing the next modular dispatch gap cluster (`crypto` and/or `network` APIs still listed in expected known gaps).
- [ ] Add per-cluster contract tests as each remaining declared gap is migrated.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/system.rs`
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/README.md`
  - `notes/GOTCHAS.md`
  - `.github/AGENT_INSTRUCTIONS.md`
