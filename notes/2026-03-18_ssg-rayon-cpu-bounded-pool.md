# Kujo Field Notes — SSG Rayon CPU-Bounded Thread-Pool Sizing

**Date:** 2026-03-18
**Session:** local
**Branch/Commit:** main / bc8a0ef (impl), 6478e48 (tests), 470ec9f (docs)
**Scope:** Capped the Rayon `ThreadPoolBuilder` thread count in
`ssg_run_rayon_read_render_write` at `min(concurrency_limit, available_parallelism).max(1)`
to prevent thread over-subscription when `concurrency_limit` >> CPU count.

---

## What I Changed

- **`src/interpreter/native_functions/async_ops.rs`** — new helper:
  - Added `ssg_rayon_cpu_cap() -> usize` (directly before `ssg_run_rayon_read_render_write`)
  - Calls `std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)`
  - No new crate dependency — `available_parallelism` is stable std since Rust 1.59

- **`src/interpreter/native_functions/async_ops.rs`** — `ssg_run_rayon_read_render_write`:
  - Replaced `ThreadPoolBuilder::new().num_threads(concurrency_limit.max(1))`
    with `ThreadPoolBuilder::new().num_threads(concurrency_limit.min(cpu_cap).max(1))`
    where `cpu_cap = ssg_rayon_cpu_cap()`
  - Added inline comment explaining the capping rationale

- **New regression tests** (4 tests, lib count 346 → 350):
  - `test_ssg_rayon_cpu_cap_returns_at_least_one` — floor guarantee
  - `test_ssg_run_rayon_oversized_concurrency_limit_clamps_to_cpu_count` — `concurrency_limit=256` still produces correct checksums
  - `test_ssg_run_rayon_small_concurrency_limit_respected_below_cpu_count` — `concurrency_limit=1` still produces correct checksums
  - `test_ssg_run_rayon_cpu_cap_equal_concurrency_limit_produces_correct_output` — boundary branch when limit == cpu_cap

---

## Gotchas (Read This Next Time)

- **Gotcha:** `std::thread::available_parallelism()` returns `NonZeroUsize`, not `usize`.
  - **Symptom:** `available_parallelism().unwrap_or(1)` fails to compile — `NonZeroUsize`
    does not implement `From<usize>`, so the fallback type doesn't match.
  - **Root cause:** The standard library wraps the result in `NonZeroUsize` to guarantee
    the value is always at least 1. That means you can't unwrap directly to `usize`.
  - **Fix:** Use `.map(|n| n.get()).unwrap_or(1)` — exactly as coded in `ssg_rayon_cpu_cap()`.
  - **Prevention:** Any time you call `available_parallelism()`, always chain `.map(|n| n.get())` before unwrapping.

- **Gotcha:** A new `ThreadPoolBuilder` pool is created on every call to `ssg_run_rayon_read_render_write`. This is intentional and safe.
  - **Symptom:** Might look like a pool is being recreated wastefully on each SSG batch.
  - **Root cause:** The function is called inside `tokio::task::spawn_blocking`, so it
    runs in a Tokio blocking thread. There is no persistent per-process Rayon pool for
    SSG — each call scopes its own pool to the batch.
  - **Fix:** None — this is the intended design.
  - **Prevention:** Do not try to cache or share the Rayon pool across `spawn_blocking`
    calls without careful analysis of Tokio + Rayon interaction. The per-call pool
    is simpler and correct. The CPU cap re-evaluation via `ssg_rayon_cpu_cap()` is a
    negligible `available_parallelism()` syscall at SSG scale.

- **Gotcha:** `parallel_map`'s `ThreadPoolBuilder` call (line ~371 in `async_ops.rs`) does NOT have the CPU cap.
  - **Symptom:** It still uses `concurrency_limit.max(1)` without clamping to CPU count.
  - **Root cause:** Intentionally left out of scope — `parallel_map` is a user-facing
    operation with explicit concurrency semantics, whereas the SSG pool was always an
    implementation detail. Applying the same cap to `parallel_map` could break user
    expectations for explicit over-subscription use cases.
  - **Fix:** None in this session — this needs a separate ROADMAP analysis.
  - **Prevention:** Do NOT apply `ssg_rayon_cpu_cap()` to `parallel_map` without a
    clear ROADMAP item and contract analysis for that function's explicit-limit semantics.

- **Gotcha:** `concurrency_limit` values at or below CPU count must pass through unchanged. The formula achieves this without any extra branching.
  - **Symptom:** If you change the formula to `cpu_cap.min(concurrency_limit)` vs
    `concurrency_limit.min(cpu_cap)`, they are equivalent — but the second form makes
    it clear that `concurrency_limit` is the primary value being constrained.
  - **Root cause:** `min` is commutative, so order doesn't matter for correctness, but
    intent is clearer with `concurrency_limit.min(cpu_cap)`.
  - **Prevention:** Verify expected values mentally: `1.min(10).max(1)` = 1, `4.min(10).max(1)` = 4,
    `256.min(10).max(1)` = 10, `0.min(10).max(1)` = 1 (guarded upstream, but safe).

---

## Things I Learned

- `bench_ssg.kujo` uses `batch_size = 256` which equals `DEFAULT_ASYNC_TASK_POOL_SIZE`.
  This is the default concurrency limit passed all the way down to `ssg_run_rayon_read_render_write`.
  On a machine with 10 cores, this previously created 256 Rayon worker threads, all competing
  for 10 CPU slots. The cap now limits the pool to `min(256, cpu_count)` = `cpu_count`.

- Rayon's work-stealing model gives no throughput benefit from having more threads than
  CPUs when each task is synchronous I/O + compute. Extra threads only add OS scheduling
  overhead and thread creation/teardown cost on each `spawn_blocking` invocation.

- `std::thread::available_parallelism()` is the idiomatic Rust way to query logical
  CPU count without a crate dependency. It reads the affinity mask (not raw CPU count),
  so it correctly respects container CPU limits and taskset/numactl restrictions.

- The `ssg_rayon_cpu_cap()` helper is placed directly before `ssg_run_rayon_read_render_write`
  in the source file so future readers see the purpose immediately next to the usage site.
  Keep them adjacent — don't move `ssg_rayon_cpu_cap()` far from its sole call site.

---

## Follow-ups / TODO (For Future Agents)

- [ ] Run `cargo bench --bench bench-ssg` to measure wall-clock impact of CPU cap vs
  uncapped pool on local hardware (`bench_ssg.kujo` uses `batch_size=256`).
- [ ] Evaluate whether `parallel_map`'s `ThreadPoolBuilder` sizing warrants a similar
  CPU cap — requires separate ROADMAP item with explicit concurrency-limit contract analysis.
- [ ] Continue v0.11.0 P0 residual-overhead slices per ROADMAP.md.

---

## Links / References

- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `notes/README.md`
- Related docs:
  - `ROADMAP.md` — "SSG Throughput Focus" remaining workstreams
  - `notes/2026-03-17_23-18_ssg-single-pass-rayon-pipeline.md` — previous session (single-pass pipeline)
