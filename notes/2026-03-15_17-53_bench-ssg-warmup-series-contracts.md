# Kujo Field Notes — bench-ssg warmup series contracts and failure surfacing

**Date:** 2026-03-15
**Session:** 17:53 local
**Branch/Commit:** main / 0524efe
**Scope:** Added `bench-ssg --warmup-runs` support and a shared warmup+measured series orchestrator in the benchmark harness. Added contract tests for warmup exclusion and phase-specific failure surfacing, then synchronized roadmap/changelog/readme docs.

---

## What I Changed
- Added benchmark series orchestration API `run_ssg_benchmark_series(...)` in `src/benchmarks/ssg.rs`
  - Validates `measured_runs >= 1`
  - Executes warmup runs first and excludes them from measured summary data
  - Executes measured runs and returns only measured results
  - Emits phase-specific errors (`Warmup run ... failed`, `Measured run ... failed`)
- Added CLI support for `kujo bench-ssg --warmup-runs <N>` in `src/main.rs`
  - New `BenchSsg` option field: `warmup_runs`
  - Bench command now uses series orchestrator instead of manually looping measured runs only
  - Output now prints `Warmup runs: <N>` in benchmark summary
- Updated benchmark exports in `src/benchmarks/mod.rs`
  - Exported `run_ssg_benchmark_series`
  - Removed now-unused `run_ssg_benchmark` re-export to avoid new warning
- Added comprehensive tests in `src/benchmarks/ssg.rs`
  - `test_run_ssg_benchmark_series_rejects_zero_measured_runs`
  - `test_run_ssg_benchmark_series_warmups_are_excluded_from_measured_results`
  - `test_run_ssg_benchmark_series_reports_warmup_failures`
  - `test_run_ssg_benchmark_series_reports_measured_failures`
- Updated release docs
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** Warmup support is easy to implement in CLI loops, but that makes behavior hard to test and easier to drift.
  - **Symptom:** Warmup/measured semantics can regress silently if logic stays in `main.rs` ad hoc loops.
  - **Root cause:** CLI-only control flow couples orchestration behavior to command output paths and skips dedicated unit-level contracts.
  - **Fix:** Centralized sequencing in `run_ssg_benchmark_series(...)` inside `src/benchmarks/ssg.rs` and tested it directly.
  - **Prevention:** Keep benchmark sequencing contracts in the benchmark module, not only in command handlers.

- **Gotcha:** Error messaging loses phase context unless warmup and measured failures are wrapped explicitly.
  - **Symptom:** Without wrappers, failures look identical regardless of warmup or measured phase, slowing diagnosis.
  - **Root cause:** `run_ssg_benchmark(...)` returns command/metric failures without phase labels.
  - **Fix:** Wrapped failures in `run_ssg_benchmark_series(...)` as `Warmup run X/Y failed: ...` and `Measured run X/Y failed: ...`.
  - **Prevention:** For multi-phase benchmark flows, always include phase+index in surfaced errors.

- **Gotcha:** Existing `unused_mut` warning in `src/vm.rs` appears during benchmark test runs and is unrelated to benchmark changes.
  - **Symptom:** `cargo test` prints `variable does not need to be mutable` for `src/vm.rs:6925`.
  - **Root cause:** Pre-existing warning outside benchmark files.
  - **Fix:** Left unchanged in this slice (non-scope warning; no new warning introduced by benchmark changes).
  - **Prevention:** Verify new work does not introduce additional warnings; do not conflate baseline warnings with slice regressions.

## Things I Learned
- Benchmark measurement quality features should be modeled as first-class orchestration APIs, not CLI-only loops.
- Warmup runs are most robust when represented as an explicit pre-measurement phase with separate error contracts.
- Keeping summary aggregation input as “measured runs only” preserves existing stats contracts with minimal downstream changes.

## Debug Notes (Only if applicable)
- **Failing test / error:** N/A (no failing tests during this slice)
- **Repro steps:** N/A
- **Breakpoints / logs used:** N/A
- **Final diagnosis:** N/A

## Follow-ups / TODO (For Future Agents)
- [ ] Add an integration-style CLI test for `bench-ssg --warmup-runs` output banner/summary text once command-level CLI harness coverage is available.
- [ ] Capture before/after local benchmark variance using `--warmup-runs` + `--runs` to quantify practical stability impact on this machine.

## Links / References
- Files touched:
  - `src/benchmarks/mod.rs`
  - `src/benchmarks/ssg.rs`
  - `src/main.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
