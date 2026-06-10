# Kujo Field Notes — Promise.all Large-Array Aggregation Optimization

**Date:** 2026-02-12
**Session:** 18:19 local
**Branch/Commit:** main / c5b255d
**Scope:** Implemented the next P0 roadmap item to optimize `promise_all` / `await_all` for large arrays, added high-volume integration tests, and updated project docs/roadmap status.

---

## What I Changed
- Reworked `Promise.all` receiver waiting logic in `src/interpreter/native_functions/async_ops.rs`.
- Replaced per-receiver `tokio::spawn(async move { rx.await })` fan-out with bounded in-task polling using `FuturesUnordered`.
- Preserved stable output ordering by keeping `(index, receiver)` mapping and writing into preallocated `results[idx]`.
- Added `futures = "0.3"` dependency in `Cargo.toml`.
- Added integration tests in `tests/interpreter_tests.rs`:
  - `test_promise_all_large_array_with_bounded_concurrency`
  - `test_await_all_large_array_uses_configured_default_pool`
- Updated `CHANGELOG.md`, `ROADMAP.md`, and `README.md` to document completion.

## Gotchas (Read This Next Time)
- **Gotcha:** `FuturesUnordered` requires one concrete future type; separate inline `async` blocks in different push sites do not type-match.
  - **Symptom:** Rust error `E0308` when pushing a second `async move` block into the same `FuturesUnordered`.
  - **Root cause:** Each `async` block has a distinct anonymous type even if body is equivalent.
  - **Fix:** Use one closure (`make_wait_future`) to construct all futures pushed into `FuturesUnordered`.
  - **Prevention:** For in-flight future pools, always centralize future creation through a single closure/function.

- **Gotcha:** Kujo-side array growth in stress tests can be misleading when relying on script-level mutation patterns.
  - **Symptom:** Stress test built promises via script loop but `len(results)` was unexpectedly `0`.
  - **Root cause:** Test harness assumptions around in-script mutation/loop update semantics were unreliable for this case.
  - **Fix:** Generate large promise arrays from Rust test code and inject as literal Kujo source.
  - **Prevention:** For high-volume integration tests, prefer deterministic source generation in Rust over dynamic script construction.

- **Gotcha:** Local disk pressure can break formatter/build unexpectedly.
  - **Symptom:** `cargo fmt` failed with `No space left on device (os error 28)`.
  - **Root cause:** `target/` artifacts consumed ~9.7GB in the workspace.
  - **Fix:** Remove `target/`, then rerun validation.
  - **Prevention:** If formatter/build fails with I/O errors, check disk usage first (`du -sh target`).

## Things I Learned
- The highest-value quick win in the current async path is removing *secondary* await-task spawning overhead inside `Promise.all`, since the underlying async operations are already running.
- Bounded in-task polling keeps concurrency limits intact without creating an extra layer of tokio task scheduling overhead.
- Large-array validation should assert shape/order/count behavior and avoid coupling to non-essential script mutation behavior.

## Debug Notes (Only if applicable)
- **Failing test / error:** `error[E0308]: mismatched types ... expected async block, found a different async block` in `async_ops.rs`.
- **Repro steps:** Build after replacing batch-spawn path with `FuturesUnordered` and multiple inline `async` pushes.
- **Breakpoints / logs used:** Rust compiler diagnostics + targeted `cargo test --test interpreter_tests <name>` runs.
- **Final diagnosis:** Distinct `async` block types in separate push sites; consolidated through one closure to stabilize type.

## Follow-ups / TODO (For Future Agents)
- [ ] Add a benchmark-focused regression test (or benchmark harness entry) specifically for `promise_all` with 1K+ promises to track wall-time trends.
- [ ] Re-run SSG benchmark and update roadmap performance target status with measured numbers.
- [ ] Evaluate whether `promise_all` should stream partial progress metrics for long-running batches.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `tests/interpreter_tests.rs`
  - `Cargo.toml`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
