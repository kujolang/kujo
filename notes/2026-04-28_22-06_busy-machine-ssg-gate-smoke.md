# Kujo Field Notes — Busy-Machine SSG Gate Smoke

**Date:** 2026-04-28
**Session:** 22:06 local
**Branch/Commit:** main / 65cc08d
**Scope:** Ran the v0.11.0 release-mode SSG gate command on a machine the operator confirmed was maxed out with other apps. Captured the result as local smoke evidence only, not final release-gate evidence.

---

## What I Changed
- Ran `cargo build --release`; it completed cleanly in release mode.
- Ran `./target/release/kujo bench-ssg --runs 7 --warmup-runs 2 --profile-async --throughput-gate-ms 10000 --tmp-dir tmp/kujo-v0.11-ssg-gate`.
- Updated `ROADMAP.md` with the busy-machine local smoke result while leaving the P0 release-gate item incomplete.

## Gotchas (Read This Next Time)
- **Gotcha:** A numeric PASS from the SSG gate is not automatically final release evidence.
  - **Symptom:** The run passed the `10000 ms` gate with a `993.559 ms` median, but emitted trend drift, CV variability, mean/median drift, and range-spread warnings.
  - **Root cause:** The operator confirmed the machine was maxed out with other apps during the benchmark, so the sample was not an idle-machine release-gate run.
  - **Fix:** Record the run as local smoke evidence only and leave the final release-gate checklist item open.
  - **Prevention:** Before treating SSG gate output as release evidence, confirm the machine is idle enough for release-gate measurement and inspect every benchmark warning section.

## Things I Learned
- `bench-ssg` can still produce a passing median under heavy local load, but the warning sections correctly capture measurement instability.
- For v0.11.0 release work, benchmark warning output is part of the evidence. Do not summarize only the PASS/FAIL line.
- Busy-machine evidence can update roadmap context, but it must not close the P0 final release-mode SSG gate item.

## Debug Notes (Only if applicable)
- **Failing test / error:** None.
- **Repro steps:** Run the exact release-gate command above while the machine is under other heavy app load.
- **Breakpoints / logs used:** Captured benchmark CLI output directly from the release binary.
- **Final diagnosis:** The release binary and SSG gate command work, but this sample is noisy local smoke evidence because the host was busy.

## Follow-ups / TODO (For Future Agents)
- [ ] Re-run the same release-mode SSG gate command on an idle machine before cutting `v0.11.0`.
- [ ] Record final release-gate evidence in the release notes only after the idle-machine run completes with acceptable warning output.

## Links / References
- Files touched:
  - `ROADMAP.md`
  - `notes/2026-04-28_22-06_busy-machine-ssg-gate-smoke.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `notes/2026-04-27_22-31_bench-ssg-range-spread-warnings.md`
  - `notes/2026-04-28_18-16_ssg-reused-output-path-buffer.md`
