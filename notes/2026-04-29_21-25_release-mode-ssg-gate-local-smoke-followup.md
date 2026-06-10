# Kujo Field Notes — Release-Mode SSG Gate Local Smoke Follow-Up

**Date:** 2026-04-29
**Session:** 21:25 local
**Branch/Commit:** main / b70af59
**Scope:** Executed the v0.11.0 release-mode SSG gate command from ROADMAP, captured metric/warning output, and recorded it as local smoke evidence because host load was high and idle-machine status was not operator-confirmed.

---

## What I Changed
- Ran `cargo build --release`; release build completed cleanly in `1.17s`.
- Ran `./target/release/kujo bench-ssg --runs 7 --warmup-runs 2 --profile-async --throughput-gate-ms 10000 --tmp-dir tmp/kujo-v0.11-ssg-gate`.
- Captured host context (`date`, `uname -a`, `sw_vers`, `uptime`) and commit SHA (`b70af59`).
- Updated `ROADMAP.md` with this run's release-mode evidence and left the P0 idle-machine gate item open.

## Gotchas (Read This Next Time)
- **Gotcha:** A release-mode gate PASS can still be smoke evidence only.
  - **Symptom:** Throughput gate reported `PASS` (`1017.971 ms <= 10000 ms`) but host load averages were high (`5.41 8.26 8.38`) and measurement-quality variability warnings were emitted.
  - **Root cause:** The release checklist requires an idle-machine run for final gate evidence; this run did not satisfy that requirement.
  - **Fix:** Record this run as local smoke evidence and keep the final idle-machine gate item incomplete.
  - **Prevention:** Always capture host load context and classify evidence as final only when machine-idle status is explicitly confirmed.

## Things I Learned
- Even when trend/mean-median/range-spread warnings are absent, CV warnings across build/throughput and stage metrics are release-relevant and should be documented.
- Stage medians (`read_ms`, `render_write_ms`) continue to be much larger than wall-clock build median due to cumulative Rayon task timing semantics.
- Bench evidence quality requires both metric values and run-environment context; PASS alone is not enough for a release decision.

## Debug Notes (Only if applicable)
- **Failing test / error:** None.
- **Repro steps:** Run the two roadmap commands in sequence from repository root.
- **Breakpoints / logs used:** CLI benchmark output and host metadata commands.
- **Final diagnosis:** Benchmark path is healthy and gate-compatible locally, but final release gate remains blocked pending an idle-machine run.

## Follow-ups / TODO (For Future Agents)
- [ ] Re-run the same release-mode gate command on an operator-confirmed idle machine.
- [ ] After final idle-machine gate evidence, run the cross-language comparison command on the same machine and capture checksum/speedup context.

## Links / References
- Files touched:
  - `ROADMAP.md`
  - `notes/2026-04-29_21-25_release-mode-ssg-gate-local-smoke-followup.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `notes/2026-04-29_21-22_release-mode-ssg-gate-local-evidence.md`
  - `ROADMAP.md`
