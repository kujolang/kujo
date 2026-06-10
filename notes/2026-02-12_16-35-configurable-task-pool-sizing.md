# Kujo Field Notes — Configurable async task pool sizing

**Date:** 2026-02-12
**Session:** 16:35 local
**Branch/Commit:** main / 85963a9
**Scope:** Implemented the next P0 incomplete roadmap item (configurable task pool sizing for async batching), added comprehensive unit/integration tests, and updated roadmap/changelog/readme with incremental commits.

---

## What I Changed
- Added per-interpreter async pool size state in `src/interpreter/mod.rs`:
  - `DEFAULT_ASYNC_TASK_POOL_SIZE`
  - `async_task_pool_size` field on `Interpreter`
  - `get_async_task_pool_size()` / `set_async_task_pool_size()` helpers
- Added new async builtins in `src/interpreter/native_functions/async_ops.rs`:
  - `set_task_pool_size(size)` returns previous size
  - `get_task_pool_size()` returns current default
- Changed default behavior in `src/interpreter/native_functions/async_ops.rs`:
  - `Promise.all`/`promise_all` now use interpreter default when explicit `concurrency_limit` is omitted
  - `parallel_map` now forwards interpreter default limit when omitted
- Registered builtins in `src/interpreter/mod.rs` (`get_builtin_names` + `register_builtins`)
- Added type signatures in `src/type_checker.rs` for both builtins
- Added tests:
  - Unit tests in `src/interpreter/native_functions/async_ops.rs`
  - Integration tests in `tests/interpreter_tests.rs`
- Updated docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** `run_code(...)` test scripts can stop progressing after a runtime error value is produced.
  - **Symptom:** A single integration test that tried to validate multiple invalid builtin calls in one Kujo script only asserted the first error reliably; later variables were missing or not set.
  - **Root cause:** In this interpreter test harness flow, once a runtime error path is hit, later statements in the same script may not execute as the test author expects.
  - **Fix:** Split error-condition assertions into isolated `run_code(...)` snippets (one failing call per script) and assert each result separately.
  - **Prevention:** For negative-path integration tests, do not chain multiple expected failures in one Kujo script; prefer one failure scenario per `run_code` invocation.

- **Gotcha:** Default async batching configuration should be interpreter-local, not global.
  - **Symptom:** A global/static default would risk cross-test contamination and nondeterministic behavior in parallel test runs.
  - **Root cause:** Async configuration is mutable runtime state and tests frequently instantiate multiple interpreters.
  - **Fix:** Store default pool size on `Interpreter` and thread it through async builtin handlers via `_interp`.
  - **Prevention:** Keep mutable runtime tuning knobs scoped to interpreter instances unless there is a hard requirement for process-global behavior.

## Things I Learned
- Interpreter-level configuration is a clean extension point for async defaults because all native function dispatch already receives `&mut Interpreter`.
- For Kujo async APIs, alias-safe identifier names (`promise_all`, `await_all`) remain the most predictable user-facing path, and new controls should align with that style.
- Comprehensive async behavior tests should include both direct native-unit coverage and full parser→interpreter integration coverage.

## Debug Notes (Only if applicable)
- **Failing test / error:**
  - `thread 'test_task_pool_size_validation_errors' panicked at tests/interpreter_tests.rs:2432:5: assertion failed: matches!(interp.env.get("bad_type"), Some(Value::Error(message)) if message.contains("requires an integer size argument"))`
- **Repro steps:**
  - Run `cargo test test_task_pool_size -- --nocapture` after adding one integration script that performs multiple invalid calls.
- **Breakpoints / logs used:**
  - Focused on test output and variable presence checks via targeted test reruns.
- **Final diagnosis:**
  - The test attempted to validate multiple error paths in one script execution; isolating each scenario into separate `run_code` calls made assertions deterministic.

## Follow-ups / TODO (For Future Agents)
- [ ] Complete the remaining Option 2 item in roadmap: optimize `Promise.all` for large arrays.
- [ ] Add benchmark coverage that compares configured default pool size behavior against explicit per-call `concurrency_limit` values.

## Links / References
- Files touched:
  - `src/interpreter/mod.rs`
  - `src/interpreter/native_functions/async_ops.rs`
  - `src/type_checker.rs`
  - `tests/interpreter_tests.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
