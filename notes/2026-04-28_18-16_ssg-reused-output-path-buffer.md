# Kujo Field Notes — Reused Output-Path Buffers in SSG Rayon Hot Path

**Date:** 2026-04-28
**Session:** 18:16 local
**Branch/Commit:** main / 804b675
**Scope:** Implemented the next v0.11 P0 throughput follow-through by removing prebuilt per-batch output-path vectors in the Rayon SSG read/render/write lane. Added dedicated regression coverage and updated release docs.

---

## What I Changed
- Added `ssg_get_cached_output_suffixes_and_prefixes(file_count)` in `src/interpreter/native_functions/async_ops.rs` to fetch cached output suffix and render-prefix metadata together.
- Added `ssg_run_rayon_read_render_write_with_reused_output_path_buffer(...)` and routed `ssg_read_render_and_write_pages(...)` to use it.
- Reworked the Rayon lane to build output paths via per-worker reusable `String` buffers (`rayon::map_init`) using cached `post_<N>.html` suffixes, instead of prebuilding a full `Vec<String>` output path list.
- Kept checksum/file-count/stage-metric and read/write error-shape contracts stable.
- Added four regression tests for the reusable-buffer lane (output correctness, checksum+stage toggle behavior, shape mismatch rejection, and write failure propagation).

## Gotchas (Read This Next Time)
- **Gotcha:** `rayon::map_init` worker-state tuple needs explicit closure parameter typing when destructuring mutable tuple state.
  - **Symptom:** Rust type inference error `E0282` for `&mut (_, _)` in the map closure.
  - **Root cause:** The compiler could not infer tuple element types (`Vec<u8>`, `String`) from destructured `&mut` state alone.
  - **Fix:** Added explicit type annotation: `|(read_buffer, output_path_buffer): &mut (Vec<u8>, String), ...|`.
  - **Prevention:** When adding multi-value worker state to Rayon `map_init`, annotate destructured mutable tuple types explicitly.

## Things I Learned
- Cached suffix metadata is enough to construct output paths lazily in-worker while avoiding per-batch upfront path-vector allocation.
- The safest way to preserve error-message path context while reusing path buffers is to format the error directly from the current mutable path buffer inside the worker closure.
- Keeping the previous helper as `#[cfg(test)]` preserves existing helper-level tests without introducing runtime dead-code warnings.

## Debug Notes (Only if applicable)
- **Failing test / error:** `error[E0282]: type annotations needed for &mut (_, _)` during `cargo build`.
- **Repro steps:** Build after introducing tuple worker-state destructuring in `ssg_run_rayon_read_render_write_with_reused_output_path_buffer(...)`.
- **Breakpoints / logs used:** Compiler error location in `src/interpreter/native_functions/async_ops.rs` closure signature.
- **Final diagnosis:** Missing explicit tuple type annotation in `rayon::map_init` worker-state closure.

## Follow-ups / TODO (For Future Agents)
- [ ] Run `bench-ssg --runs <N>` with and without `--profile-async` to quantify wall-clock impact of output-path buffer reuse.
- [ ] Consider a matching reusable-path-buffer follow-through for `ssg_render_and_write_pages(...)` if benchmark traces show meaningful write-path time there.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
