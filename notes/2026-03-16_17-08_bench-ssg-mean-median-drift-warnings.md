# Kujo Field Notes — Bench SSG Mean/Median Drift Warnings

**Date:** 2026-03-16
**Session:** 17:08 local
**Branch/Commit:** main / 3bf8a55
**Scope:** Implemented the next high-priority v0.11 benchmark-stability slice by adding mean/median drift warning signals for `bench-ssg` measurement-quality interpretation. Added comprehensive warning contracts and synchronized CHANGELOG/ROADMAP/README.

---

## What I Changed
- Updated `src/benchmarks/ssg.rs`:
  - Added `SSG_MEAN_MEDIAN_DRIFT_WARNING_THRESHOLD_PERCENT` (`7.5`).
  - Added `SsgRunStatistics::mean_median_drift_percent(...)` and `is_high_mean_median_drift(...)`.
  - Added `collect_ssg_mean_median_drift_warnings(...)` for Kujo/Python/speedup metrics and stage metrics.
  - Added comprehensive regression tests for drift calculation, zero-median handling, low-run-count gating, and warning emission/suppression.
- Updated `src/main.rs`:
  - Wired new drift warnings into `bench-ssg` “Measurement quality warnings” output.
- Updated docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** Warning signal additions should not alter benchmark metric contracts.
  - **Symptom:** Easy to accidentally expand output contract into new metric keys instead of warning interpretation-only changes.
  - **Root cause:** Measurement-quality work can blur into benchmark payload/schema changes.
  - **Fix:** Added warning-only analysis from existing aggregate stats (mean/median/runs), with no new emitted benchmark metric keys.
  - **Prevention:** For roadmap item “refining warning thresholds/presentation,” limit changes to analysis and CLI warning text only.

- **Gotcha:** Drift warnings become noisy with very small sample counts.
  - **Symptom:** Two-run series can produce large apparent drift from one outlier.
  - **Root cause:** Mean/median divergence is unstable at low `N`.
  - **Fix:** Reused the measured-run gate (`runs >= 3`) before warning emission.
  - **Prevention:** Keep warning gates aligned across variability/trend/drift signals unless roadmap explicitly requests different policy.

## Things I Learned
- Mean/median drift is a complementary signal to CV and trend drift: it catches skewed distributions even when directional trend is small.
- Existing `bench-ssg` test structure makes it straightforward to add warning-signal slices without touching harness subprocess contracts.
- Combining warning families under one output header in CLI keeps interpretation-focused features discoverable without adding command flags.

## Debug Notes (Only if applicable)
- **Failing test / error:** None; no runtime/test failure debugging was required.
- **Repro steps:** N/A.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** N/A.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue benchmark-warning interpretation follow-through by considering de-duplication/prioritization when multiple warning families trigger together.
- [ ] Revisit threshold tuning with empirical run-series data if warning volume is too high/low in real benchmark sessions.

## Links / References
- Files touched:
  - `src/benchmarks/ssg.rs`
  - `src/main.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
