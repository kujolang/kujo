# Kujo Field Notes — Bench SSG Opt-In Stage Profiling

**Date:** 2026-04-28
**Session:** 16:42 local
**Branch/Commit:** main / cd83f98
**Scope:** Implemented the next v0.11 P0 throughput follow-through by making SSG stage profiling opt-in for `bench-ssg` and adding a no-stage-timer fast path for non-profile runs.

---

## What I Changed
- Added profile-toggle propagation in benchmark harness:
  - `src/main.rs` now passes `profile_async` into `run_ssg_benchmark_series(...)`.
  - `src/benchmarks/ssg.rs` now threads a `profile_async: bool` argument through benchmark run helpers and sets `KUJO_BENCH_SSG_PROFILE_ASYNC=1|0` for subprocesses.
- Updated cross-language benchmark scripts:
  - `benchmarks/cross-language/bench_ssg.kujo` now emits `KUJO_SSG_READ_MS` and `KUJO_SSG_RENDER_WRITE_MS` only when profiling is enabled.
  - `benchmarks/cross-language/bench_ssg.py` now conditionally measures/emits stage metrics based on the same env flag.
- Optimized native SSG hot path:
  - `src/interpreter/native_functions/async_ops.rs` now supports stage-timer bypass in `ssg_run_rayon_read_render_write(...)` via `collect_stage_metrics`.
  - `ssg_read_render_and_write_pages(...)` now reads profiling intent from `KUJO_BENCH_SSG_PROFILE_ASYNC` and routes to timer/no-timer path.
- Added tests:
  - Harness test for profile-toggle stage-metric capture in `src/benchmarks/ssg.rs`.
  - Native helper test for zeroed stage metrics with timers disabled in `src/interpreter/native_functions/async_ops.rs`.

## Gotchas (Read This Next Time)
- **Gotcha:** Full `cargo test` can fail on network round-trip tests in sandboxed environments.
  - **Symptom:** `test_release_hardening_network_module_round_trip_behaviors` fails with `PermissionDenied` (`Operation not permitted`) while binding an ephemeral TCP listener.
  - **Root cause:** Sandbox/network permissions can block socket bind operations even when code is correct.
  - **Fix:** Validate changed areas with targeted tests in addition to full-suite attempts; treat bind-permission failures as environment constraints when unrelated to modified code.
  - **Prevention:** When touching benchmark/runtime code unrelated to network sockets, run focused module tests (`benchmarks::ssg`, `async_ops`) and capture full-suite permission failures explicitly.

## Things I Learned
- The cleanest way to make `bench-ssg` stage profiling opt-in without changing script CLI shape is to push profiling intent through environment (`KUJO_BENCH_SSG_PROFILE_ASYNC`) from Rust harness to subprocess scripts.
- `parse_metric_value_optional(...)` already supports absent stage metrics, which makes conditional metric emission safe without changing aggregate contract behavior.
- Adding a boolean timer-collection switch in the Rayon helper gives a direct throughput-path optimization while preserving existing checksum/error contracts.

## Debug Notes (Only if applicable)
- **Failing test / error:** `interpreter::native_functions::tests::test_release_hardening_network_module_round_trip_behaviors` -> `PermissionDenied` during ephemeral listener bind.
- **Repro steps:** `cargo test` from repo root in sandboxed session.
- **Breakpoints / logs used:** N/A (failure is direct test panic with OS error).
- **Final diagnosis:** Environment permission issue, not regression from profile-toggle or SSG path changes.

## Follow-ups / TODO (For Future Agents)
- [ ] Measure `kujo bench-ssg` wall-clock delta before/after this toggle on the same host to quantify non-profile timer-overhead removal.
- [ ] If stage metrics become needed in non-profile diagnostics, consider adding a separate per-run output flag that does not require script edits.

## Links / References
- Files touched:
  - `src/main.rs`
  - `src/benchmarks/ssg.rs`
  - `src/interpreter/native_functions/async_ops.rs`
  - `benchmarks/cross-language/bench_ssg.kujo`
  - `benchmarks/cross-language/bench_ssg.py`
- Related docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
