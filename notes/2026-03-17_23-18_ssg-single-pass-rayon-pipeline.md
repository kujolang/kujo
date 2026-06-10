# Kujo Field Notes — SSG Single-Pass Rayon Pipeline

**Date:** 2026-03-17
**Session:** 23:18 local
**Branch/Commit:** main / 3b0f35f (pre-commit; implementation in working tree)
**Scope:** Replaced the two-phase Rayon read-barrier pipeline with a single-pass
Rayon pipeline in `ssg_run_rayon_read_render_write`. Each Rayon task now reads,
renders, and writes its own file independently without waiting for all reads to
complete first.

---

## What I Changed

- **`src/interpreter/native_functions/async_ops.rs`** — `ssg_run_rayon_read_render_write`:
  - Removed two `pool.install()` calls (Phase 1 reads, then Phase 2 render+writes)
  - Replaced with a single `pool.install()` where each task does read → render → write
  - Per-task timing tuples `(total_len: usize, read_ns: u64, rw_ns: u64)` collected
    in the result Vec and summed after `collect()` to derive `read_ms` / `render_write_ms`
  - Updated doc comment to reflect single-pass semantics and cumulative timing
- **`src/interpreter/native_functions/async_ops.rs`** — new tests added:
  - `test_ssg_run_rayon_single_pass_timing_fields_are_non_negative`
  - `test_ssg_run_rayon_single_pass_cumulative_timing_grows_with_file_count`
  - `test_ssg_run_rayon_single_pass_large_batch_preserves_checksum`
  - `test_ssg_run_rayon_single_pass_single_worker_preserves_index_mapping`

---

## Gotchas (Read This Next Time)

- **Gotcha:** `read_ms` and `render_write_ms` semantics changed from wall-clock phase
  timing to cumulative CPU-time sums across Rayon workers.
  - **Symptom:** `read_ms` for a 4-worker, 16-file run will now be roughly 4× the
    single-worker `read_ms` (sum of all task read durations).  In the two-phase
    implementation, `read_ms` was wall-clock time for "time from first read start to
    last read end".
  - **Root cause:** Without a phase boundary, there is no single wall-clock instant
    that separates "all reads done" from "writes start". Capturing per-task nanoseconds
    and summing is the only safe approach with `par_iter().collect()`.
  - **Fix:** The sum approach is intentional — it still satisfies the `>= 0.0`
    contract checked by all tests. The benchmark harness uses its own `Instant` at
    the caller level for wall-clock SSG throughput measurement, so this doesn't affect
    bench-ssg numbers.
  - **Prevention:** Do NOT compare `read_ms` between the two-phase and single-pass
    implementations and expect the same values. If you add a test that asserts
    `read_ms < threshold`, the threshold needs to be the sum of all tasks' read
    times, not the wall-clock phase duration.

- **Gotcha:** The `ssg_run_rayon_read_render_write` doc comment said "two-phase" after
  the refactor. Updated to "single-pass" to avoid misleading future readers.
  - **Symptom:** Doc comment began with "Runs a Rayon-parallel two-phase SSG pipeline"
  - **Fix:** Updated the doc comment, `read_ms` / `render_write_ms` semantics paragraph included

- **Gotcha:** Pre-existing `cargo test` warning in `src/vm.rs:6925` — `let mut vm = VM::new()`.
  - **Symptom:** `cargo test` emits 1 warning listing `variable does not need to be mutable`
    at `vm.rs:6925` in `test_cooperative_suspend_enabled_by_default`.
  - **Root cause:** Pre-existing test code not related to this session's changes.
    `cargo build` (production build) produced zero warnings before and after. Only the
    test build activates this code path.
  - **Fix:** None needed — this is not our code to change in this session.
  - **Prevention:** Do not confuse pre-existing `cargo test` warnings in `vm.rs` with
    newly-introduced warnings. Always diff `cargo build` warnings before and after your
    change to isolate what your change introduced.

---

## Things I Learned

- **Rayon `par_iter().collect()` ordering guarantee holds in single-pass too.** The
  index variable in `enumerate()` correctly maps `source_paths[i]` to `output_paths[i]`
  and `render_prefixes[i]` even in the single-pass loop. Rayon preserves input indices.
  This was already confirmed in the last session for two-phase but worth re-verifying
  when restructuring the closure.

- **Per-task `Instant::now()` inside a Rayon `par_iter()` closure is safe.** Each
  Rayon task runs in its own thread from the pool. `Instant::now()` is thread-safe and
  produces monotonic readings. No shared mutable state is needed for timing.

- **`as_nanos() as u64` vs `as_secs_f64() * 1000.0`:** Using `u64` nanoseconds for
  accumulation avoids floating-point precision loss when summing across many tasks.
  Converting to `f64` milliseconds only at the end (`total_read_ns as f64 / 1_000_000.0`)
  keeps the per-task accumulation exact.

- **Peak memory benefit is real but hard to test.** In the two-phase implementation,
  ALL file contents are alive in RAM simultaneously at the Phase 1→Phase 2 boundary.
  In the single-pass:  each task holds one file in memory at a time. For N files with
  K workers, peak in-memory content is at most K files simultaneously, not N. Not
  testable in a unit test, but important for the bench-ssg workload (10k files).

---

## Follow-ups / TODO (For Future Agents)

- [ ] Run `cargo bench --bench bench-ssg` before/after this commit to quantify
      wall-clock impact vs the two-phase `3b0f35f` baseline. The single-pass should
      fare better on I/O-bound workloads where reads and writes can overlap at the
      hardware level even though Rayon serializes within each task.
- [ ] Profile whether `ThreadPoolBuilder::new().num_threads(concurrency_limit)` sizing
      is optimal or whether matching it to CPU logical cores (ignoring concurrency_limit
      for pool sizing) would perform better for I/O work. Rayon defaults to `num_cpus`
      which may already be near-optimal.
- [ ] Consider exposing `read_ms` / `render_write_ms` breakdown in the bench-ssg
      output to distinguish "time in reads" vs "time in render+write" for profiling.
      Currently bench-ssg only reports total SSG wall-clock time.
- [ ] Continue v0.11.0 P0 residual-overhead slices per ROADMAP.md.

---

## Links / References

- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
- Related docs:
  - `ROADMAP.md` (v0.11.0 SSG Throughput section)
  - `CHANGELOG.md`
  - `notes/2026-03-17_18-53_ssg-rayon-parallel-pipeline.md` (previous session — two-phase implementation)
  - `notes/GOTCHAS.md`
