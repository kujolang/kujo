# Kujo Field Notes — Release hardening for async batch + shared/task-pool contracts

**Date:** 2026-02-16
**Session:** 09:34 local
**Branch/Commit:** main / b0b4ca9
**Scope:** Expanded v0.10.0 release-hardening contract coverage for async batch file APIs and shared/task-pool concurrency APIs, then synchronized release docs. This session added behavior + argument/error-shape tests in the native dispatcher contract suite.

---

## What I Changed
- Added new release-hardening tests in `src/interpreter/native_functions/mod.rs`:
  - `test_release_hardening_async_batch_file_contracts`
  - `test_release_hardening_shared_state_and_task_pool_contracts`
- Covered async batch API contracts for:
  - `async_read_files`
  - `async_write_files`
- Covered shared-state and task-pool API contracts for:
  - `shared_set`, `shared_get`, `shared_has`, `shared_delete`, `shared_add_int`
  - `set_task_pool_size`, `get_task_pool_size`
- Added success-path lifecycle checks:
  - `async_write_files(...)` then `async_read_files(...)` with deterministic content assertions
  - shared key set/get/add/delete round-trip behavior
  - task-pool get/set return-shape and validation behavior
- Updated release docs for this completed hardening slice:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Validation performed:
  - `cargo test test_release_hardening_async_batch_file_contracts -- --nocapture`
  - `cargo test test_release_hardening_shared_state_and_task_pool_contracts -- --nocapture`
  - `cargo test`
  - `cargo build`

## Gotchas (Read This Next Time)
- **Gotcha:** `shared_*` native APIs use a process-global store that persists across interpreters and tests.
  - **Symptom:** Shared-state tests can become order-dependent or flaky when reusing key names.
  - **Root cause:** `src/interpreter/native_functions/concurrency.rs` stores values in a static `OnceLock<Mutex<HashMap<...>>>`.
  - **Fix:** Use session-unique key names in hardening tests (timestamp-suffixed key) and clean up with `shared_delete`.
  - **Prevention:** Treat shared keys like global resources; never use fixed key literals in independent tests unless test ordering is controlled.

- **Gotcha:** Async batch wrappers return Promises, so contract tests must verify both dispatch-time and awaited-result behavior.
  - **Symptom:** Dispatch-level shape checks pass, but lifecycle behavior can still regress if the awaited payload shape changes.
  - **Root cause:** `call_native_function(...)` only validates immediate return shape (`Value::Promise`) unless tests explicitly await and inspect resolved values.
  - **Fix:** Pair argument/error-shape checks with `await_native_promise(...)` success assertions for `async_read_files`/`async_write_files`.
  - **Prevention:** For Promise-returning hardening APIs, require both pre-await and post-await contracts in the same test slice.

## Things I Learned
- Release-hardening follow-through remains most effective when each slice includes both strict invalid-shape checks and one full success lifecycle path.
- For concurrency-related contracts, deterministic tests come from minimizing shared global state assumptions and using unique identifiers per test run.
- A single hardening slice should include implementation tests first, then docs synchronization in separate commits to preserve clean rollback checkpoints.

## Debug Notes (Only if applicable)
- **Failing test / error:** None in this session (new tests passed on first full run).
- **Repro steps:** Not applicable.
- **Breakpoints / logs used:** Used targeted test runs plus full `cargo test` output verification.
- **Final diagnosis:** No runtime regressions; all behavior matched existing async/concurrency contracts.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue v0.10 release-hardening by selecting the next newly-introduced builtin/alias slice and repeating dispatcher + behavior contract expansion.
- [ ] Consider extracting release-hardening contract sections in `src/interpreter/native_functions/mod.rs` into smaller module-local test files if review noise continues to grow.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/README.md`
  - `notes/GOTCHAS.md`
  - `.github/AGENT_INSTRUCTIONS.md`
