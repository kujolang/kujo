# Kujo Field Notes — Single-worker overlap prefetch lane for fused SSG path

**Date:** 2026-03-17
**Session:** 12:35 local
**Branch/Commit:** main / 7535b2f
**Scope:** Implemented and validated a v0.11 P0 throughput follow-through slice for the fused SSG helper. Specifically, the `concurrency_limit=1` lane in `ssg_read_render_and_write_pages(...)` now overlaps bounded read/write progression, with docs and tests updated.

---

## What I Changed
- Reworked `concurrency_limit=1` execution in `src/interpreter/native_functions/async_ops.rs` for `ssg_read_render_and_write_pages(...)`.
- Replaced strictly sequential read-then-write behavior with a bounded overlap lane:
  - one read in-flight
  - one write in-flight
  - one pending-write slot
- Added helper policy gate `ssg_should_prefetch_single_worker_read(...)` to make prefetch decisions explicit and testable.
- Added focused unit coverage for the prefetch policy helper.
- Added integration regression coverage that verifies per-index output mapping preservation under single-worker overlap/prefetch behavior.
- Updated release docs for the completed follow-through:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Committed incrementally:
  - `dee9110` (`:package: NEW: overlap single-worker fused SSG read and write lane`)
  - `7535b2f` (`:book: DOC: record single-worker overlap prefetch throughput follow-through`)

## Gotchas (Read This Next Time)
- **Gotcha:** Single-worker overlap must guard prefetch against pending buffered writes.
  - **Symptom:** It is easy to accidentally schedule another read while a buffered write payload is waiting, which can skew lane progression and complicate ordering guarantees.
  - **Root cause:** In the single-worker overlap lane, write capacity is intentionally 1 active + 1 pending, so prefetch must consider pending-write occupancy.
  - **Fix:** Gate prefetch on all three conditions via `ssg_should_prefetch_single_worker_read(...)`: remaining reads > 0, no read in-flight, and no pending write.
  - **Prevention:** Keep prefetch as a dedicated policy helper with direct tests; do not inline ad-hoc boolean logic in the select loop.

- **Gotcha:** `cargo fmt`/feature-slice test runs can leave unrelated files dirty in this repo.
  - **Symptom:** After completing this slice, `git status --short` still showed unrelated modifications in `src/benchmarks/ssg.rs` and `src/main.rs`.
  - **Root cause:** Existing workspace state and formatter spillover can coexist with the feature slice.
  - **Fix:** Stage and commit only intentional files for each step (`async_ops.rs` first, docs files second).
  - **Prevention:** Always re-check `git status --short` before each commit and isolate commit scope explicitly.

## Things I Learned
- For fused SSG single-worker lanes, “single-worker” does not mean “strictly sequential”; bounded overlap can preserve contracts while reducing idle gaps.
- Stage-metric semantics are an invariant: `read_ms` must finalize when reads are exhausted, and `render_write_ms` must continue tracking write drain.
- The safest way to evolve concurrency logic is to encode policy in helper functions and lock behavior with focused tests plus output/index-mapping regressions.

## Debug Notes (Only if applicable)
- **Failing test / error:** No feature test failures after implementation; targeted tests passed.
- **Repro steps:**
  - `cargo test ssg_should_prefetch_single_worker_read_requires_remaining_without_pending_write`
  - `cargo test ssg_read_render_and_write_pages_single_worker_prefetch_preserves_index_mapping`
- **Breakpoints / logs used:** Rust test assertions + contract verification through generated output files.
- **Final diagnosis:** Behavior and contracts remained correct with the new overlap lane; unrelated warning/noise remained outside this slice.

## Follow-ups / TODO (For Future Agents)
- [ ] Run comparative `bench-ssg --profile-async` before/after sampling to quantify this lane’s impact under local conditions.
- [ ] Continue v0.11 P0 residual-overhead slices while preserving checksum/file-count/stage-metric contracts.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/README.md`
  - `README.md`
  - `ROADMAP.md`
