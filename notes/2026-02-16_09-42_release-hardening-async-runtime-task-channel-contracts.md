# Kujo Field Notes — Release Hardening Async Runtime/Task/Channel Contracts

**Date:** 2026-02-16
**Session:** 09:42 local
**Branch/Commit:** main / db77d73
**Scope:** Expanded v0.10.0 P1 release-hardening compatibility contracts for async runtime/task-channel builtins and synchronized roadmap/changelog/readme documentation. Focus was dispatcher coverage plus behavior/argument-shape contracts, not runtime implementation changes.

---

## What I Changed
- Added async runtime/task-channel release-hardening tests in `src/interpreter/native_functions/mod.rs`:
  - `test_release_hardening_async_sleep_timeout_contracts`
  - `test_release_hardening_async_http_argument_contracts`
  - `test_release_hardening_async_file_wrapper_contracts`
  - `test_release_hardening_channel_and_task_handle_contracts`
- Extended the release-hardening critical builtin probe list to include:
  - `channel`, `async_sleep`, `async_timeout`, `async_http_get`, `async_http_post`
  - `async_read_file`, `async_write_file`, `spawn_task`, `await_task`, `cancel_task`
- Added test helper utilities in the same test module:
  - `await_native_promise(...)`
  - `noop_spawnable_function()`
- Updated docs to reflect this completed hardening slice:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Commits created:
  - `76cb6e8` (`:ok_hand: IMPROVE: harden async core API dispatch and contract coverage`)
  - `db77d73` (`:book: DOC: record async runtime release-hardening contract follow-through`)

## Gotchas (Read This Next Time)
- **Gotcha:** `call_native_function(&mut interpreter, ...)` cannot be nested inside another `call_native_function(...)` argument list.
  - **Symptom:** Rust compile error `E0499: cannot borrow 'interpreter' as mutable more than once at a time`.
  - **Root cause:** The outer native call holds one mutable borrow while evaluating argument expressions; nested call attempts a second mutable borrow.
  - **Fix:** Evaluate inner native call first into a local variable, then pass that value to the outer call.
  - **Prevention:** In contract tests, never inline nested `call_native_function(&mut interpreter, ...)` expressions; stage intermediate values with `let`.

- **Gotcha:** `await_task(...)` consumes the task handle and a later await on the same handle returns an already-consumed error.
  - **Symptom:** First await resolves; second await on cloned handle fails with `Task handle already consumed`.
  - **Root cause:** `await_task` takes the inner join handle (`handle_guard.take()`), so subsequent awaits have no handle to poll.
  - **Fix:** Assert consumed-handle behavior explicitly in tests instead of assuming re-await is valid.
  - **Prevention:** Treat task handles as single-consumer for await semantics in release-hardening contracts.

## Things I Learned
- Release-hardening coverage should include both dispatcher non-fallback assertions and concrete behavior/shape contracts for the same API slice.
- For async APIs in this codebase, argument-shape tests are often the most deterministic high-signal contracts; behavior tests should use local deterministic primitives (`async_sleep`, local temp files) when possible.
- Task lifecycle semantics are intentionally asymmetric:
  - `cancel_task` can return `true` then `false` on repeated calls
  - `await_task` is one-shot per handle

## Debug Notes (Only if applicable)
- **Failing test / error:** `error[E0499]: cannot borrow 'interpreter' as mutable more than once at a time` during `cargo test release_hardening_async`.
- **Repro steps:** Inline nested `call_native_function(&mut interpreter, ...)` inside another call's argument array in `src/interpreter/native_functions/mod.rs` tests.
- **Breakpoints / logs used:** No runtime logging needed; fixed at compile-time by reading borrow site in compiler output.
- **Final diagnosis:** Test shape issue only (nested mutable borrow), not runtime behavior regression.

## Follow-ups / TODO (For Future Agents)
- [ ] Add release-hardening behavior contracts for successful `async_http_get`/`async_http_post` paths using deterministic local server fixtures (if/when stable harness exists).
- [ ] Consider adding a tiny shared test helper for one-shot task-handle lifecycle assertions to reduce repeated boilerplate in future async contract slices.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
