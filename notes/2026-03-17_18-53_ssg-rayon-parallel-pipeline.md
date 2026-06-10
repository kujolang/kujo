# Kujo Field Notes — Rayon-based parallel read+render+write pipeline for SSG

**Date:** 2026-03-17
**Session:** 18:53 local
**Branch/Commit:** main / 7ee437a
**Scope:** Replaced the Tokio `FuturesUnordered` pipeline in `ssg_read_render_and_write_pages(...)` with a `spawn_blocking` + Rayon `par_iter` two-phase pipeline. This is a v0.11.0 P0 SSG Throughput slice targeting elimination of per-file Tokio task-spawn overhead and `FuturesUnordered` polling overhead.

---

## What I Changed
- Rewrote the `ssg_read_render_and_write_pages` handler body in `src/interpreter/native_functions/async_ops.rs`.
- Extracted a new synchronous blocking helper `ssg_run_rayon_read_render_write(...)` (~96 lines):
  - Creates one Rayon `ThreadPool` sized to `concurrency_limit.max(1)` threads.
  - Phase 1: `pool.install(|| source_paths.par_iter().enumerate().map(...).collect())` — bounded parallel reads.
  - Phase 2: `pool.install(|| contents.par_iter().enumerate().map(|(i, content)| { build html, std::fs::write }).collect())` — parallel HTML render + single-call `std::fs::write`.
  - Returns `Result<(i64, f64, f64), String>` = (checksum, read_ms, render_write_ms); error contract preserved.
- Replaced ~350-line Tokio `FuturesUnordered` handler body with ~45-line `spawn_blocking` + `ssg_run_rayon_read_render_write` invocation.
- Removed `use std::collections::VecDeque` (was only used by the old pipeline).
- Moved 4 scheduling helpers to `#[cfg(test)]` (they are only used by tests now):
  - `ssg_read_ahead_limit`
  - `ssg_target_read_in_flight`
  - `ssg_should_refill_writes_first`
  - `ssg_should_prefetch_single_worker_read`
- Added 5 targeted regression tests for the new Rayon helper:
  - `test_ssg_run_rayon_read_render_write_reads_and_writes_correctly`
  - `test_ssg_run_rayon_read_render_write_checksum_matches_written_bytes`
  - `test_ssg_run_rayon_read_render_write_propagates_read_failure`
  - `test_ssg_run_rayon_read_render_write_propagates_write_failure`
  - `test_ssg_run_rayon_read_render_write_unicode_checksum_matches_written_bytes`
- Updated release docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Committed incrementally:
  - `faf3bf9` (`:package: NEW: implement Rayon-based parallel read+render+write pipeline for ssg_read_render_and_write_pages`)
  - `f420625` (`:ok_hand: IMPROVE: add 5 regression tests for ssg_run_rayon_read_render_write`)
  - `7ee437a` (`:book: DOC: document Rayon parallel SSG pipeline in CHANGELOG, ROADMAP, and README`)

## Gotchas (Read This Next Time)

- **Gotcha:** After replacing a large Tokio pipeline, previously-live scheduling helpers become dead code.
  - **Symptom:** `cargo build` emits `warning: function ... is never used` for `ssg_read_ahead_limit`, `ssg_target_read_in_flight`, `ssg_should_refill_writes_first`, `ssg_should_prefetch_single_worker_read` — even though they still have test coverage.
  - **Root cause:** The production code path no longer calls them; only `#[cfg(test)]` paths call them.
  - **Fix:** Add `#[cfg(test)]` annotation to each helper and its doc comment block. Do NOT use `#[allow(dead_code)]`.
  - **Prevention:** After any large pipeline replacement, run `cargo build` immediately and enumerate all new warnings before committing.

- **Gotcha:** `VecDeque` import becomes unused when the pipelined loop is removed.
  - **Symptom:** `warning: unused import: \`std::collections::VecDeque\`` after pipeline swap.
  - **Root cause:** The old Tokio FuturesUnordered read-ahead loop used `VecDeque` as a pending-write buffer; the Rayon approach collects into `Vec` directly.
  - **Fix:** Delete the `use std::collections::VecDeque;` line.
  - **Prevention:** Check all imports in the handler file after a large refactor, not just function bodies.

- **Gotcha:** Rayon `par_iter().collect()` preserves input index ordering.
  - **Symptom (confusion):** It is easy to assume work-stealing might reorder results (it does NOT for `collect()`).
  - **Root cause:** Rayon's collect preserves index order by design; the output `Vec` is index-matched to the input.
  - **Fix:** None needed; rely on this guarantee for `output_paths[index]` and `render_prefixes[index]` lookups.
  - **Prevention:** Document this assumption in the helper's doc comment and in tests.

- **Gotcha:** The two-phase Rayon approach (all-reads-then-all-writes) does NOT overlap read and write stages.
  - **Symptom:** Unlike the previous `FuturesUnordered` pipeline, there is a full barrier between Phase 1 and Phase 2.
  - **Root cause:** Rayon `install()` blocks until all workers complete; the second `install()` can only start after the first finishes.
  - **Fix:** This is intentional and acceptable for initial adoption. If a read/write overlap is needed later, a single `par_iter` that pipelines both stages per file would eliminate the barrier.
  - **Prevention:** Document that this is a two-phase barrier, not a streaming pipeline, and profile before assuming it is a bottleneck.

## Things I Learned
- The Rayon approach is architecturally simpler than the Tokio FuturesUnordered pipeline: no in-flight counters, no pending-write buffers, no select loops — just two parallel `collect()` calls.
- `std::fs::write` (one syscall: open + write + close) is simpler than `tokio::fs::File::create + write_vectored + flush` and works naturally in a Rayon blocking context.
- Error format contracts (`"Failed to read file '{}' (index {}): {}"`) must be preserved exactly if tests assert on the error string. Always grep for the old format before changing it.
- `multi_replace_string_in_file` can update 3–4 documentation files in a single tool call, which is the most efficient way to handle docs-only changes.
- After a large pipeline replacement, re-examine every non-test function in the file for newly-dead helpers; don't wait for CI to find them.

## Debug Notes (Only if applicable)
- **Failing test / error:** No feature test failures; 5 warnings emitted after first build — all resolved before commit.
- **Repro steps (dead-code warnings):**
  - `cargo build 2>&1 | grep warning`
  - Identify all functions never used outside `#[cfg(test)]`
  - Add `#[cfg(test)]` wrapper
- **Final diagnosis:** After wrapping test-only helpers and removing `VecDeque`, build produced zero warnings. Full test suite: 342 lib + 238 integration = 580 tests, 0 failures.

## Follow-ups / TODO (For Future Agents)
- [ ] Run comparative `bench-ssg` before/after to quantify the Rayon pipeline's wall-clock impact.
- [ ] Consider a single-pass Rayon pipeline that interleaves per-file read + render + write to eliminate the Phase 1 → Phase 2 barrier.
- [ ] Profile whether `ThreadPoolBuilder::new().num_threads(concurrency_limit)` sizing is optimal or whether the global Rayon pool would perform better.
- [ ] Continue v0.11.0 P0 residual-overhead slices while preserving checksum/file-count/stage-metric contracts.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/README.md`
  - `README.md`
  - `ROADMAP.md`
