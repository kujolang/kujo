# Kujo Field Notes — V1 HTTP 004 Hidden and Private File Blocking

**Date:** 2026-05-15
**Session:** 10:49 local
**Branch/Commit:** main / a386de7
**Scope:** Implemented roadmap item `V1-HTTP-004` by adding a centralized hidden/private request-path block policy for `kujo serve`, plus regression coverage and docs updates.

---

## What I Changed
- Added centralized private-path filtering to static serve request-target validation in `src/serve_http.rs`.
- Enforced a deny-by-default policy for hidden/private paths:
  - Dotfile and dot-directory components (for example `.env`, `.git`, `.svn`, `.hg`, `.DS_Store`)
  - Backup/swap-style leaf suffixes (`.bak`, `.backup`, `.tmp`, `.old`, `.orig`, `.swp`, `.swo`, trailing `~`)
- Added `src/serve_http.rs` unit tests for hidden/private path rejection and non-private path pass-through.
- Added integration regressions in `tests/serve_command_integration.rs` for:
  - `.env`
  - `.git/config`
  - `.DS_Store`
  - backup/swap file paths
  - normal file success path
- Updated `README.md`, `CHANGELOG.md`, and `ROADMAP.md` for the completed item.

## Gotchas (Read This Next Time)
- **Gotcha:** Running multiple subprocess-backed `serve` tests in one filtered command can fail with startup-timeout noise instead of behavior failures.
  - **Symptom:** `kujo serve did not become reachable in time on port ...` in `serve_command_integration` tests.
  - **Root cause:** Parallel test execution can add startup contention/race for short-lived serve subprocess checks.
  - **Fix:** Use deterministic test threading (`-- --test-threads=1`) for targeted serve-subprocess runs while developing.
  - **Prevention:** Prefer single-threaded targeted runs for socket/subprocess-heavy integration slices, then confirm full-suite behavior with `cargo test`.

## Things I Learned
- Path-policy checks are safest at request-target validation time, before any filesystem existence checks, because that avoids existence leaks and keeps one deterministic `403` contract.
- Component-level dotfile blocking plus leaf-level suffix blocking is enough to close the listed private-file leak surface without adding a broad CLI behavior change.

## Debug Notes (Only if applicable)
- **Failing test / error:** Hidden/private file regression tests initially returned `200` for `.env`, `.git/config`, `.DS_Store`, and backup/swap files.
- **Repro steps:** `cargo test --test serve_command_integration serve_blocks_ -- --nocapture`
- **Breakpoints / logs used:** Regression assertions and focused serve-target validator tests.
- **Final diagnosis:** `validate_request_target(...)` sanitized traversal/encoding but had no hidden/private path policy.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider whether a documented `--allow-hidden` flag is needed for power users (roadmap says only if strong use case appears).
- [ ] Revisit broader static-server request/connection limit hardening in `V1-HTTP-005`.

## Links / References
- Files touched:
  - `src/serve_http.rs`
  - `tests/serve_command_integration.rs`
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
