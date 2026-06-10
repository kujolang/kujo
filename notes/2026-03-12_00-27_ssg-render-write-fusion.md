# Kujo Field Notes — Async SSG Render+Write Fusion

**Date:** 2026-03-12
**Session:** 00:27 local
**Branch/Commit:** main / 664aa39
**Scope:** Implemented the next P0 roadmap throughput slice by adding a fused async native helper for SSG render+write, migrated `bench-ssg` to use it, and expanded contract + dispatcher coverage.

---

## What I Changed
- Added `ssg_render_and_write_pages(source_pages, output_dir, concurrency_limit?)` in `src/interpreter/native_functions/async_ops.rs`.
- Registered the new builtin in `src/interpreter/mod.rs` (`get_builtin_names()` and `register_builtins()`).
- Added dispatcher-level release-hardening coverage in `src/interpreter/native_functions/mod.rs`.
- Switched benchmark render/write stage to the fused helper in `benchmarks/cross-language/bench_ssg.kujo`.
- Added helper tests for:
  - success path (writes expected HTML and returns `{ checksum, files }`),
  - argument-shape validation,
  - write-failure propagation.
- Updated release docs (`CHANGELOG.md`, `ROADMAP.md`, `README.md`).

## Gotchas (Read This Next Time)
- **Gotcha:** `cargo fmt` touched unrelated files during this slice.
  - **Symptom:** `git status` showed unrelated diffs in `src/interpreter/native_functions/io.rs` and `src/vm.rs`.
  - **Root cause:** workspace formatting pass rewrote long lines in files outside the intended change scope.
  - **Fix:** explicitly restored unrelated files before committing: `git restore src/interpreter/native_functions/io.rs src/vm.rs`.
  - **Prevention:** always run `git status --short` after formatting and keep commits atomic by restoring non-scope files.

- **Gotcha:** New fused helper intentionally does not create missing output directories.
  - **Symptom:** promise resolves to rejection when `output_dir` does not exist.
  - **Root cause:** helper delegates to async file write path and preserves existing `async_write_files` contract behavior.
  - **Fix:** create `output_dir` before calling helper (benchmark script already does this).
  - **Prevention:** keep directory setup explicit in scripts/tests; don’t silently add implicit directory-creation behavior without a contract decision.

## Things I Learned
- Throughput-focused SSG optimizations can stay contract-safe by fusing orchestration while preserving established error semantics.
- The right insertion point for this helper is `async_ops.rs` (returns Promise, bounded concurrency, write lifecycle), not `strings.rs`.
- Release-hardening dispatcher tests are the safest place to guard new builtin argument contracts against dispatch drift.

## Debug Notes (Only if applicable)
- **Failing test / error:** none (no persistent failures).
- **Repro steps:** N/A.
- **Breakpoints / logs used:** test-only validation via targeted and full-suite cargo commands.
- **Final diagnosis:** implementation integrated cleanly; no runtime regressions observed.

## Assumptions I Almost Made
- I initially considered adding directory auto-creation inside `ssg_render_and_write_pages`, but that would silently change write-path contract semantics versus existing async file APIs.

## Follow-ups / TODO (For Future Agents)
- [ ] Benchmark and compare `bench-ssg` median render/write stage deltas pre/post fusion to quantify impact in roadmap snapshots.
- [ ] Consider whether a separate explicit helper (e.g., `ensure_dir`) should be part of any future SSG convenience API contract.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `src/interpreter/mod.rs`
  - `src/interpreter/native_functions/mod.rs`
  - `benchmarks/cross-language/bench_ssg.kujo`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
