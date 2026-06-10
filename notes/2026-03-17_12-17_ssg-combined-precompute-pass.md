# Kujo Field Notes — SSG combined precompute pass

**Date:** 2026-03-17
**Session:** 12:17 local
**Branch/Commit:** main / 716e1e7
**Scope:** Implemented the next v0.11 P0 SSG throughput slice by fusing output-path and HTML-prefix precompute into one batch pass and validating checksum/file-count and stage-metric contract preservation.

---

## What I Changed
- Added `ssg_build_output_paths_and_prefixes_for_batch(output_dir, file_count)` in `src/interpreter/native_functions/async_ops.rs`.
- Switched both `ssg_render_and_write_pages(...)` and `ssg_read_render_and_write_pages(...)` to use the combined precompute helper.
- Added regression tests:
  - `test_ssg_build_output_paths_and_prefixes_for_batch_generates_parallel_outputs`
  - `test_ssg_build_output_paths_and_prefixes_for_batch_handles_empty_input`
- Scoped legacy helper functions to `#[cfg(test)]` to avoid dead-code warnings after migration while keeping existing helper tests valid.
- Updated release docs for the completed throughput slice:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** Throughput refactors can leave helper functions orphaned in non-test builds
  - **Symptom:** New `dead_code` warnings appeared for `ssg_build_output_path(...)`, `ssg_build_output_paths_for_batch(...)`, and `ssg_build_render_prefixes_for_batch(...)` after switching runtime paths to the combined helper.
  - **Root cause:** Old helpers remained referenced only by tests, not production code.
  - **Fix:** Marked legacy helpers with `#[cfg(test)]` so they stay available to tests without polluting non-test builds.
  - **Prevention:** After performance refactors, run `cargo build` and check for orphaned helper warnings before committing.

- **Gotcha:** Combined precompute changes still must preserve benchmark equivalence contracts
  - **Symptom:** Any drift in indexed names or prefixes would silently break checksum/file-count comparability in `bench-ssg`.
  - **Root cause:** Bench contracts depend on deterministic output naming/content (`post_<index>.html` and stable `<h1>Post <index></h1>` prefixes).
  - **Fix:** Kept path/prefix logic deterministic and added direct helper regression tests for both normal and empty input.
  - **Prevention:** Treat path/prefix generation as contract-critical, not implementation-detail optimization.

## Things I Learned
- For SSG throughput work, one-pass precompute is safe when output naming and prefix format remain byte-for-byte compatible.
- Legacy helper functions can remain valuable as test fixtures; test-only scoping is cleaner than deleting useful test utilities immediately.
- The effective optimization loop is: implement -> run focused SSG tests -> run `cargo build` for warning validation -> then commit.

## Debug Notes (Only if applicable)
- **Failing test / error:** No test failures; warning surfaced during validation: `function ... is never used` in `src/interpreter/native_functions/async_ops.rs`.
- **Repro steps:** `cargo build` after wiring the combined precompute helper.
- **Breakpoints / logs used:** Build output warning scan plus targeted test commands (`cargo test test_ssg_build_output_paths_and_prefixes_for_batch`, `cargo test ssg_render_and_write_pages`, `cargo test ssg_read_render_and_write_pages`).
- **Final diagnosis:** Runtime code no longer called legacy helpers; they were test-only after refactor.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider consolidating or retiring duplicated test-only helper coverage if it no longer adds value.
- [ ] Address unrelated persistent warning in `src/vm.rs` (`unused_mut`) to align with strict zero-warning policy.

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