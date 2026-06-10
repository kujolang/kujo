# Kujo Field Notes — Release-Mode SSG Gate Local Evidence

**Date:** 2026-04-29
**Session:** 21:22 local
**Branch/Commit:** main / 15d1eaa
**Scope:** Ran the v0.11.0 release-mode SSG gate command from `ROADMAP.md` and recorded the result as local evidence because idle-machine status was not operator-confirmed.

---

## What I Changed
- Ran `cargo build --release`; it completed cleanly with no compiler warnings in the captured output.
- Ran `./target/release/kujo bench-ssg --runs 7 --warmup-runs 2 --profile-async --throughput-gate-ms 10000 --tmp-dir tmp/kujo-v0.11-ssg-gate`.
- Updated `ROADMAP.md` with the local release-mode PASS while leaving the P0 final idle-machine release gate incomplete.

## Gotchas (Read This Next Time)
- **Gotcha:** A clean warning-free gate run is still not final release-gate evidence without idle-machine confirmation.
  - **Symptom:** The run passed the `10000 ms` gate with a `1804.098 ms` median and emitted no benchmark warning sections.
  - **Root cause:** The release checklist explicitly requires an idle-machine run, and the host load captured before this run was `3.95 4.71 3.75`; no operator confirmed the machine was idle.
  - **Fix:** Record the run as local release-mode evidence and leave the final idle-machine gate open.
  - **Prevention:** Treat both benchmark warnings and host-idle confirmation as part of the release evidence contract before marking the P0 item complete.

## Things I Learned
- The required release-mode command can take much longer wall-clock time than the measured `KUJO_SSG_BUILD_MS` values because the harness also seeds and cleans up 10,000 input/output files per warmup/measured run.
- Absence of benchmark warning sections is useful evidence, but it does not override the roadmap's idle-machine requirement.
- The benchmark-facing metric contracts were present in the summary: files `10000`, checksum `946670`, median build time, median throughput, and profiled read/render-write stage medians.

## Debug Notes (Only if applicable)
- **Failing test / error:** None.
- **Repro steps:** Run the release build, then run the exact release-gate command from `ROADMAP.md`.
- **Breakpoints / logs used:** Captured CLI output plus host context from `date`, `uname -a`, `sw_vers`, and `uptime`.
- **Final diagnosis:** The code passes the configured throughput gate locally, but this sample is not sufficient to close the final idle-machine release gate.

## Follow-ups / TODO (For Future Agents)
- [ ] Re-run the same release-mode SSG gate command on an operator-confirmed idle machine before cutting `v0.11.0`.
- [ ] After final gate evidence is accepted, run the P0 cross-language comparison command on the same machine.

## Links / References
- Files touched:
  - `ROADMAP.md`
  - `notes/README.md`
  - `notes/2026-04-29_21-22_release-mode-ssg-gate-local-evidence.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `notes/2026-04-28_22-06_busy-machine-ssg-gate-smoke.md`
  - `ROADMAP.md`
