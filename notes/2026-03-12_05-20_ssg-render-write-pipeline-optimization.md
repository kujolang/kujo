# Kujo Field Notes — SSG render/write pipeline optimization follow-through

**Date:** 2026-03-12
**Session:** 05:20 local
**Branch/Commit:** main / 69a2c31
**Scope:** Completed the v0.11 P0 throughput follow-through by optimizing `ssg_render_and_write_pages(...)` to remove serial pre-render buffering, added regression coverage, and synchronized release docs.

---

## What I Changed
- Optimized `ssg_render_and_write_pages(...)` in `src/interpreter/native_functions/async_ops.rs`:
  - removed serial pre-render buffering of `(index, output_path, html)` tuples
  - moved HTML synthesis into bounded async write workers
  - preserved output-path naming and returned summary contract (`files`, `checksum`)
- Added helper functions in `src/interpreter/native_functions/async_ops.rs`:
  - `ssg_build_output_path(...)`
  - `ssg_build_html(...)`
  - `ssg_html_render_overhead_len(...)`
- Added regression tests in `src/interpreter/native_functions/async_ops.rs`:
  - `test_ssg_render_and_write_pages_checksum_matches_rendered_outputs`
  - `test_ssg_render_and_write_pages_empty_input_returns_zero_summary`
- Updated release-tracking docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Kept commit history atomic across feature, tests, and docs.

## Gotchas (Read This Next Time)
- **Gotcha:** `cargo fmt` can touch unrelated files and pollute commit scope.
  - **Symptom:** unrelated formatting diffs appeared in `src/interpreter/native_functions/io.rs` and `src/vm.rs`.
  - **Root cause:** repository-wide formatting pass included files not part of this feature.
  - **Fix:** reverted unrelated files with `git checkout -- src/interpreter/native_functions/io.rs src/vm.rs` before final docs commit.
  - **Prevention:** after formatting, always run `git status --short` and explicitly drop unrelated diffs before committing.

- **Gotcha:** checksum contract must remain exact even when render/write internals change.
  - **Symptom:** optimization changed where HTML was built (task lane instead of pre-render stage), which could silently drift checksum behavior.
  - **Root cause:** checksum is externally validated by benchmark harness equivalence checks; any output-shape/length drift is contract-breaking.
  - **Fix:** computed checksum using deterministic render overhead + source-body lengths and added explicit test asserting checksum equals sum of written HTML lengths.
  - **Prevention:** for any future render-path optimization, keep checksum validation tests in the same change set and compare against actual written output.

## Things I Learned
- For this SSG path, reducing orchestrator memory/work staging is practical without changing benchmark contracts.
- Throughput work in v0.11 can be incremental if each slice preserves checksum/file-count equivalence and keeps focused contract coverage.
- Atomic commit slicing (feature → tests → docs) made this follow-through easy to reason about and review.

## Debug Notes (Only if applicable)
- **Failing test / error:** none in feature scope; only pre-existing `unused_mut` warning in `src/vm.rs` surfaced during test runs.
- **Repro steps:** run `cargo test ssg_render_and_write_pages -- --nocapture` and `cargo test ssg -- --nocapture`.
- **Breakpoints / logs used:** test-output inspection only.
- **Final diagnosis:** optimization and contracts were correct; warning was unrelated to this task.

## Follow-ups / TODO (For Future Agents)
- [ ] Collect before/after `bench-ssg --profile-async --runs 5` measurements in a stable environment and append a short benchmark delta note.
- [ ] Continue v0.11 render/write overhead reduction while preserving checksum/file-count equivalence guards.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `notes/2026-03-12_05-20_ssg-render-write-pipeline-optimization.md`
  - `notes/README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
