# Kujo Field Notes — Promise cache reuse in async aggregations

**Date:** 2026-02-14
**Session:** 10:10 local
**Branch/Commit:** main / e2db0cb
**Scope:** Implemented P0 async optimization steps for Promise aggregation and mapper overhead reduction. Focused on `Promise.all(...)` / `parallel_map(...)` behavior when immediate values and already-polled promises are mixed.

---

## What I Changed
- Optimized `parallel_map(...)` in `src/interpreter/native_functions/async_ops.rs` to avoid synthetic oneshot Promise wrapping for immediate mapper outputs.
- Refactored `Promise.all(...)` in `src/interpreter/native_functions/async_ops.rs` to preallocate vectors and reuse one debug-flag check in hot path.
- Added cache-aware promise handling in `Promise.all(...)` and `parallel_map(...)`:
  - detect and consume already-cached promise results before touching receivers
  - persist newly resolved/rejected outcomes back into each promise cache (`is_polled` + `cached_result`).
- Added integration tests in `tests/interpreter_tests.rs`:
  - mixed immediate + promise mapper outputs in `parallel_map(...)`
  - immediate-only fast path behavior
  - cached promise reuse via `promise_all([p, p], ...)`
  - cached promise reuse in `parallel_map(...)` mapper return path.
- Updated release/status docs in `CHANGELOG.md`, `ROADMAP.md`, and `README.md` for the completed optimization milestones.

## Gotchas (Read This Next Time)
- **Gotcha:** Promise receivers are single-consumer; reading the same promise twice fails unless cache is consulted first.
  - **Symptom:** Reusing a previously-awaited promise in `promise_all([p, p], ...)` can hit channel-closed errors when code re-consumes the receiver.
  - **Root cause:** oneshot receivers are moved out with `std::mem::replace(...)`; subsequent awaits must use `cached_result` instead of receiver.
  - **Fix:** In aggregation paths, check `is_polled`/`cached_result` first (`read_cached_promise_result(...)`) and only await real pending receivers.
  - **Prevention:** Rule: every code path that consumes `Value::Promise.receiver` must gate on cache state first.

- **Gotcha:** `parallel_map(...)` immediate values do not need Promise wrapping, and wrapping adds avoidable churn.
  - **Symptom:** Extra oneshot allocations/channels even when mapper result is immediate (non-Promise) value.
  - **Root cause:** Previous implementation normalized immediates by wrapping each into synthetic Promise before calling `Promise.all(...)`.
  - **Fix:** Store immediate values directly in preallocated result slots, await only pending promise receivers.
  - **Prevention:** Rule: keep immediate values immediate; only build async await lanes for actual Promise values.

- **Gotcha:** Async integration tests can panic with nested-runtime errors if a path indirectly calls runtime blocking inside runtime worker context.
  - **Symptom:** Panic text: `Cannot start a runtime from within a runtime`.
  - **Root cause:** Test shape used async user-function composition that hit nested runtime/blocking behavior in this interpreter path.
  - **Fix:** Reworked mixed-result test to use a synchronous mapper returning conditional `async_sleep(...)` promises.
  - **Prevention:** For interpreter integration tests, prefer sync mappers returning Promise values over nested async-function orchestration unless specifically testing runtime nesting behavior.

## Things I Learned
- Promise cache fields (`is_polled`, `cached_result`) are not just for `await` expression; aggregation functions must also honor them to be semantically correct under repeated-await usage.
- The highest-value optimization was structural: split immediate lane from pending async lane, then aggregate in one pass.
- A useful invariant: any time an async path consumes a receiver, it should either populate cache on completion or fail loudly with a cache-consistent error.
- “Already-polled promise reuse is intentional behavior” must be treated as a runtime guarantee, not an incidental side effect.

## Debug Notes (Only if applicable)
- **Failing test / error:** `Cannot start a runtime from within a runtime. This happens because a function (like block_on) attempted to block the current thread while the thread is being used to drive asynchronous tasks.`
- **Repro steps:**
  1. Run `cargo test test_parallel_map_handles_mixed_immediate_and_promise_results`
  2. Use mixed mapper shape that composes async user function + `parallel_map(...)`
- **Breakpoints / logs used:** Test-level binary slicing plus targeted reruns (`cargo test <single_test_name>`), then reshaped test case.
- **Final diagnosis:** Failure came from test composition/runtime interaction, not from promise cache logic itself.

## Follow-ups / TODO (For Future Agents)
- [ ] Add a dedicated regression test that intentionally exercises nested async function + `parallel_map(...)` to pin runtime semantics.
- [ ] Measure scalability impact with 10K+ concurrent operations now that cache-aware aggregation is implemented.
- [ ] Consider extracting shared aggregation logic (`Promise.all` and `parallel_map`) into one helper to reduce divergence risk.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `tests/interpreter_tests.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/README.md`
  - `ROADMAP.md`
