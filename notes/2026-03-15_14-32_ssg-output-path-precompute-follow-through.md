# Kujo Field Notes — SSG output-path precompute throughput follow-through

**Date:** 2026-03-15
**Session:** 14:32 local
**Branch/Commit:** main / 26ef9e7
**Scope:** Completed the next v0.11 P0 throughput slice by precomputing indexed output paths once per batch in native async SSG write pipelines. Added regression coverage and synchronized roadmap/changelog/readme updates.

---

## What I Changed
- Optimized `src/interpreter/native_functions/async_ops.rs`:
  - Added `ssg_build_output_paths_for_batch(output_dir, file_count)` helper.
  - Updated `ssg_render_and_write_pages(...)` to reuse precomputed paths across bounded async write workers.
  - Updated `ssg_read_render_and_write_pages(...)` to reuse precomputed paths across fused read/render/write dispatch.
- Added regression coverage in `src/interpreter/native_functions/async_ops.rs`:
  - `test_ssg_build_output_paths_for_batch_generates_stable_indexed_paths`
  - `test_ssg_render_and_write_pages_large_batch_low_concurrency_preserves_outputs`
- Updated release docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** `cargo fmt` touched unrelated files during a focused SSG slice.
  - **Symptom:** Formatting-only diffs appeared in `src/interpreter/native_functions/io.rs` and `src/vm.rs` after formatting.
  - **Root cause:** Workspace formatter normalization spilled into unrelated modules.
  - **Fix:** Restored unrelated files and committed only intended SSG changes.
  - **Prevention:** Always run `git status --short` after formatting and `git restore` unrelated paths before staging.

- **Gotcha:** Output-path optimizations can silently break benchmark equivalence if path naming drifts.
  - **Symptom:** Any mismatch in indexed output names would break checksum/file-count equivalence checks in `bench-ssg` validation.
  - **Root cause:** Throughput refactors can accidentally diverge path generation logic between fused helpers.
  - **Fix:** Centralized batch path generation in one helper and reused it in both async SSG write pipelines.
  - **Prevention:** Keep path generation contract-stable (`post_<index>.html`) and lock with high-volume output-contract tests.

## Things I Learned
- Small per-write string construction work still matters in high-file-count benchmark paths.
- Shared helper reuse across both SSG pipelines is safer than duplicating path generation logic inside futures.
- Existing checksum/file-count and stage-metric contracts are the right guardrails for throughput-only slices.

## Debug Notes (Only if applicable)
- **Failing test / error:** none in feature scope.
- **Repro steps:**
  - `cargo test ssg_render_and_write_pages -- --nocapture`
  - `cargo test ssg_read_render_and_write_pages -- --nocapture`
  - `cargo test ssg -- --nocapture`
- **Breakpoints / logs used:** test output and contract assertions only.
- **Final diagnosis:** output-path precompute optimization preserved benchmark/output contracts and passed focused + broader SSG suites.

## Follow-ups / TODO (For Future Agents)
- [ ] Capture before/after `kujo bench-ssg --profile-async --runs 5` medians for this slice under stable load.
- [ ] Continue profiling residual render/write overhead after path precompute without changing benchmark key contracts.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `notes/2026-03-15_14-32_ssg-output-path-precompute-follow-through.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
