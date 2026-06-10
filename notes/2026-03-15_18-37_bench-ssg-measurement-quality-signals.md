# Kujo Field Notes — bench-ssg measurement quality signals

**Date:** 2026-03-15
**Session:** 18:37 local
**Branch/Commit:** main / 1fda8ec
**Scope:** Added two benchmark-stability slices for `bench-ssg`: variability warning signals (CV-based) and first-to-last measured-run trend reporting. Updated CLI output, benchmark-module contracts, and release docs.

---

## What I Changed
- Added variability analysis helpers in `src/benchmarks/ssg.rs`:
  - `SSG_VARIABILITY_WARNING_THRESHOLD_PERCENT`
  - `SsgRunStatistics::coefficient_of_variation_percent(...)`
  - `SsgRunStatistics::is_high_variability(...)`
  - `collect_ssg_variability_warnings(...)`
- Added trend analysis helpers in `src/benchmarks/ssg.rs`:
  - `SsgTrendMetric`
  - `SsgBenchmarkTrendReport`
  - `analyze_ssg_benchmark_trends(...)`
- Updated CLI output in `src/main.rs` (`bench-ssg`) to print:
  - measurement-quality variability warnings
  - first→last measured-run trend summary
- Added comprehensive benchmark-module tests in `src/benchmarks/ssg.rs` for:
  - high/low variability warning emission contracts
  - trend analysis no-python/python paths
  - single-run trend suppression
  - inconsistent python-comparison presence rejection
  - zero-baseline percent-delta handling
- Updated release docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Completed and pushed incremental commits:
  - `9ec604a`, `15de932`, `3d87541`, `1fda8ec`

## Gotchas (Read This Next Time)
- **Gotcha:** Trend reporting is intentionally suppressed for a single measured run.
  - **Symptom:** No `Measured trend (first→last ...)` section appears when `--runs 1` is used.
  - **Root cause:** `analyze_ssg_benchmark_trends(...)` returns `Ok(None)` for `< 2` measured runs because drift has no meaning without at least two points.
  - **Fix:** Run with `--runs >= 2` when you need directional trend output.
  - **Prevention:** Treat trend analysis as series-only metadata; keep summary medians as the primary signal for single-run execution.

- **Gotcha:** Python trend analysis requires cross-run consistency, not best-effort fallback.
  - **Symptom:** Trend analysis fails with an explicit error when only some runs contain Python metrics.
  - **Root cause:** `analyze_ssg_benchmark_trends(...)` enforces all-or-none Python metric presence to avoid mixed-contract output.
  - **Fix:** Ensure `--compare-python` is either present for all measured runs or absent for all measured runs in a series.
  - **Prevention:** Keep run-series contract inputs stable across measured runs; do not mix compare and non-compare samples.

- **Gotcha:** Existing warning in `src/vm.rs` can appear during targeted benchmark work.
  - **Symptom:** `cargo build` / targeted tests complete but still report `unused_mut` around `src/vm.rs:6925`.
  - **Root cause:** This warning is pre-existing and unrelated to benchmark-module changes.
  - **Fix:** No change in this session (kept scope focused on benchmark stability feature slices).
  - **Prevention:** Record this explicitly during scoped work so unrelated baseline warnings are not mistaken for regressions introduced by the benchmark change.

## Things I Learned
- `bench-ssg` measurement quality is now clearer when combining three signals in one run summary:
  - aggregate stats (median/mean/min/max/stddev)
  - variability warnings (CV threshold)
  - directional drift (first→last trend)
- For trend deltas, percent-change from a near-zero baseline should be treated as undefined (`None`) rather than forced to an arbitrary large number.
- Keeping trend analysis in `src/benchmarks/ssg.rs` and only rendering in `src/main.rs` keeps contract testing straightforward and avoids CLI-only logic drift.

## Debug Notes (Only if applicable)
- **Failing test / error:** None (feature implemented test-first style).
- **Repro steps:** N/A.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** No runtime bug; main effort was contract-shape and output-semantics hardening.

## Follow-ups / TODO (For Future Agents)
- [ ] Add optional stage-level trend reporting (`read_ms` / `render_write_ms`) if output noise remains understandable.
- [ ] Re-run `bench-ssg --runs 5 --profile-async --compare-python` on an idle machine and capture trend/variability observations for roadmap tracking.

## Links / References
- Files touched:
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
  - `notes/README.md`
