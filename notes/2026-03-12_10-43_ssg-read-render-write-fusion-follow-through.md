# Kujo Field Notes — SSG read/render/write fusion follow-through

**Date:** 2026-03-12
**Session:** 10:43 local
**Branch/Commit:** main / f0535cf
**Scope:** Completed the next v0.11 P0 throughput slice by adding fused async native `ssg_read_render_and_write_pages(...)`, wiring benchmark usage, validating contracts, and synchronizing release docs.

---

## What I Changed
- Added fused native helper `ssg_read_render_and_write_pages(source_paths, output_dir, concurrency_limit?)` in `src/interpreter/native_functions/async_ops.rs`.
- Exposed and registered the helper in interpreter builtin surfaces:
  - `src/interpreter/mod.rs`
  - `src/interpreter/native_functions/mod.rs`
- Updated timed SSG benchmark path to use fused helper summary metrics (`read_ms`, `render_write_ms`) in `benchmarks/cross-language/bench_ssg.kujo`.
- Added comprehensive contract coverage for:
  - success + summary shape
  - checksum/file-count equivalence
  - argument validation
  - read/write failure propagation
- Synchronized release docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** `cargo fmt` can introduce unrelated diffs during docs/finalization.
  - **Symptom:** unrelated modified files appeared (`src/interpreter/native_functions/io.rs`, `src/vm.rs`) after formatting.
  - **Root cause:** repository-wide formatter pass touched files outside feature scope.
  - **Fix:** reverted unrelated files before staging docs commit.
  - **Prevention:** always run `git status --short` after formatting and prune non-scope changes before commit.

- **Gotcha:** benchmark stage metrics must remain contract-stable when helper orchestration changes.
  - **Symptom:** moving from script-level orchestration to fused helper could silently break `KUJO_SSG_READ_MS` / `KUJO_SSG_RENDER_WRITE_MS` reporting.
  - **Root cause:** benchmark output depends on exact metric keys and stage meanings.
  - **Fix:** returned `read_ms` and `render_write_ms` from helper summary and mapped them in benchmark script without changing external metric key contract.
  - **Prevention:** whenever benchmark-path helpers are refactored, keep metric-key compatibility checks and parser tests in the same slice.

## Things I Learned
- Throughput improvements are safer when benchmark contracts (checksum, file-count, metric keys) are treated as first-class APIs.
- Fused helper slices are reviewable when scoped across three layers only: native helper, builtin registration/dispatch, benchmark call-site.
- Reading current file contents immediately before edits is essential in active branches where formatter/automations may have touched target files.

## Debug Notes (Only if applicable)
- **Failing test / error:** none in feature scope.
- **Repro steps:**
  - `cargo test ssg_read_render_and_write_pages -- --nocapture`
  - `cargo test test_release_hardening_ssg_render_pages_dispatch_contracts -- --nocapture`
  - `cargo test ssg -- --nocapture`
  - `cargo test`
- **Breakpoints / logs used:** test-output inspection only.
- **Final diagnosis:** helper + benchmark integration held contracts and passed targeted + broad suites.

## Follow-ups / TODO (For Future Agents)
- [ ] Capture before/after `kujo bench-ssg --profile-async --runs 5` medians for this fused helper slice under stable machine load.
- [ ] Continue reducing `bench-ssg` path overhead while preserving checksum/file-count and stage-metric compatibility.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `src/interpreter/native_functions/mod.rs`
  - `src/interpreter/mod.rs`
  - `benchmarks/cross-language/bench_ssg.kujo`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `notes/2026-03-12_10-43_ssg-read-render-write-fusion-follow-through.md`
  - `notes/README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
