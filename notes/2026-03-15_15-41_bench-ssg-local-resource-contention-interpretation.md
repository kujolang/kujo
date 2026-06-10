# Kujo Field Notes — bench-ssg local resource contention interpretation

**Date:** 2026-03-15
**Session:** 15:41 local
**Branch/Commit:** main / 39b932d
**Scope:** Re-ran SSG throughput comparison after a stalled attempt, isolated reproducibility issues, and interpreted before/after medians with explicit local system load caveats.

---

## What I Changed
- Re-ran benchmark comparison with a stable process after prior hangs/spins:
  - Benchmarked current commit (`39b932d`) and pre-change commit (`e3af9e2`) with `--runs 3` and `--profile-async`.
  - Used a reduced benchmark script at `checkpoints/bench_ssg_small.kujo` (`file_count := 3000`) to keep runs tractable under local load.
- Captured benchmark artifacts:
  - `checkpoints/bench_small_head_39b932d.txt`
  - `checkpoints/bench_small_pre_e3af9e2.txt`
- Used a temporary git worktree for pre-change execution:
  - `checkpoints/wt_e3af9e2` (removed after run)

## Gotchas (Read This Next Time)
- **Gotcha:** Local machine contention can invert short benchmark comparisons.
  - **Symptom:** Throughput medians drifted and appeared regressive even with small code-path changes.
  - **Root cause:** Concurrent user workloads (CPU, memory pressure, filesystem/cache contention) dominated the benchmark signal.
  - **Fix:** Treated numbers as directional and documented environment caveat explicitly.
  - **Prevention:** Prefer idle-machine runs for pass/fail conclusions; use loaded-machine runs only for rough trend checks.

- **Gotcha:** Full `bench-ssg` can fail in this environment before producing comparable metrics.
  - **Symptom:** Scheduler/runtime errors and stalled attempts during full-size benchmark execution.
  - **Root cause:** Environment/runtime instability under heavier workload during benchmark orchestration.
  - **Fix:** Switched to a reduced-size script and fixed run-count profiling path for reproducible medians.
  - **Prevention:** Keep a reduced-size benchmark profile for sanity checks when the full profile is unstable.

## Things I Learned
- Benchmark interpretation needs an explicit environment-quality label (idle vs loaded machine).
- Under load, median shifts can reflect host contention more than code-level performance deltas.
- Reproducible comparison flow (fixed script, fixed run count, same flags) is more important than absolute values during busy local sessions.

## Debug Notes (Only if applicable)
- **Failing test / error:**
  - `Scheduler did not complete all contexts within 1000 rounds (1 pending)` during full `bench-ssg` path.
- **Repro steps:**
  - `target/debug/kujo bench-ssg --profile-async --runs 3`
- **Breakpoints / logs used:** benchmark command output and emitted summary metrics.
- **Final diagnosis:** environment load likely contributed substantial noise; reduced-script run path produced stable comparable medians.

## Follow-ups / TODO (For Future Agents)
- [ ] Re-run the same before/after comparison on an idle machine with full benchmark script to validate directionality.
- [ ] Add a documented “loaded-machine” benchmark mode note (trend-only) vs “release-gate” mode (idle, repeatable).

## Links / References
- Files touched:
  - `checkpoints/bench_ssg_small.kujo`
  - `checkpoints/bench_small_head_39b932d.txt`
  - `checkpoints/bench_small_pre_e3af9e2.txt`
  - `notes/2026-03-15_15-41_bench-ssg-local-resource-contention-interpretation.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `ROADMAP.md`
