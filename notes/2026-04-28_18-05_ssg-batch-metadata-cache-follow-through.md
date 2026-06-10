# Kujo Field Notes — SSG batch metadata cache follow-through

**Date:** 2026-04-28
**Session:** 18:05 local
**Branch/Commit:** main / ed7aab8
**Scope:** Implemented the next v0.11 P0 SSG throughput item by caching reusable per-index SSG batch metadata (render prefixes + output filename suffixes) and wiring cached metadata into both SSG write paths.

---

## What I Changed
- Added cache surfaces in `src/interpreter/native_functions/async_ops.rs`:
  - `SSG_RENDER_PREFIX_CACHE` keyed by `file_count`
  - `SSG_OUTPUT_FILE_SUFFIX_CACHE` keyed by `file_count`
- Added cache access/build helpers:
  - `ssg_get_or_build_render_prefixes(file_count)`
  - `ssg_get_or_build_output_file_suffixes(file_count)`
- Updated `ssg_build_output_paths_and_prefixes_for_batch(...)` to:
  - reuse cached output suffixes when constructing `output_dir/post_<N>.html`
  - return cached render-prefix metadata for downstream write paths
- Updated `ssg_run_rayon_read_render_write(...)` to consume cached render-prefix metadata in its Rayon hot lane.
- Updated `ssg_render_and_write_pages(...)` to consume cached render-prefix metadata.
- Added focused cache regression tests:
  - cached render-prefix Arc reuse for identical `file_count`
  - cached output-suffix Arc reuse for identical `file_count`
  - cache separation when `file_count` differs

## Gotchas (Read This Next Time)
- **Gotcha:** Changing prefix metadata from `Vec<String>` to shared cached metadata can silently break helper test call-sites.
  - **Symptom:** Type mismatch at `ssg_run_rayon_read_render_write(...)` call sites expecting `Vec<String>` prefixes.
  - **Root cause:** The hot-path function now expects shared cached prefix metadata (`Arc<Vec<Arc<str>>>`) rather than owned prefix strings.
  - **Fix:** Update direct call-sites/tests to pass shared prefix metadata (or convert test-only local vectors to the shared type).
  - **Prevention:** When changing metadata ownership shape for throughput work, immediately grep for direct helper invocations and fix them before broad test runs.

- **Gotcha:** Full-suite test runs in this environment can fail on restricted network/socket permissions unrelated to SSG work.
  - **Symptom:** `test_release_hardening_network_module_round_trip_behaviors` fails with `PermissionDenied` while binding ephemeral TCP listener.
  - **Root cause:** Sandbox restrictions on local socket operations in this run environment.
  - **Fix:** Validate SSG-focused suites and full build/warning checks; treat network-module failure as environment-related unless networking code changed.
  - **Prevention:** Keep a targeted test set for SSG throughput work and record environment-related full-suite failures explicitly.

## Things I Learned
- Reusing per-index metadata by `file_count` is a low-risk throughput optimization because SSG prefix text and output suffixes are deterministic on index only.
- Keeping checksum/file-count/stage-metric contracts intact is easier when cache wiring is isolated to metadata creation and not interleaved with read/write accounting logic.
- For this codebase, preserving error-message shapes is as important as preserving output values because release-hardening tests assert message contracts.

## Debug Notes (Only if applicable)
- **Failing test / error:** `expected Arc<Vec<Arc<str>>> found Vec<String>` at `ssg_run_rayon_read_render_write(...)` call-site in sync-vectored write-failure test.
- **Repro steps:**
  - `CARGO_TARGET_DIR=target/agent-temp cargo test test_ssg_build_output_paths_and_prefixes_for_batch_reuses_cached_prefixes_for_same_count -- --nocapture`
- **Breakpoints / logs used:** Compiler type error output + `sed` inspection around failing call-site.
- **Final diagnosis:** One direct helper test still passed old owned-prefix vector type after hot-path signature change.

## Follow-ups / TODO (For Future Agents)
- [ ] Measure bench-level wall-clock deltas from cached batch metadata under repeated-run `bench-ssg --runs <N>` in an unrestricted benchmark environment.
- [ ] Consider a bounded eviction strategy for metadata caches if `file_count` cardinality grows in non-benchmark workloads.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `notes/2026-04-28_18-05_ssg-batch-metadata-cache-follow-through.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `ROADMAP.md`
