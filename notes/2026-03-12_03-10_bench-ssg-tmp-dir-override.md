# Kujo Field Notes — bench-ssg tmp-dir override and artifact-root control

**Date:** 2026-03-12
**Session:** 03:10 local
**Branch/Commit:** main / 3a65947
**Scope:** Implemented the next high-priority v0.11 benchmark-stability slice by adding `bench-ssg --tmp-dir`, wiring a shared artifact-root override through Kujo/Python benchmark subprocesses, and updating tests/docs.

---

## What I Changed
- Added CLI option `--tmp-dir <PATH>` to `BenchSsg` in `src/main.rs` and passed it through to benchmark execution.
- Extended `run_ssg_benchmark(...)` in `src/benchmarks/ssg.rs` to accept `tmp_dir: Option<&Path>`.
- Added benchmark subprocess env override wiring via `KUJO_BENCH_SSG_TMP_DIR` in `src/benchmarks/ssg.rs`.
- Added tmp-dir contract tests in `src/benchmarks/ssg.rs` (`resolve_tmp_dir_override` success/none/non-UTF8 cases).
- Updated `benchmarks/cross-language/bench_ssg.kujo` to use `env_or("KUJO_BENCH_SSG_TMP_DIR", "tmp")`.
- Updated `benchmarks/cross-language/bench_ssg.py` to use `os.environ.get("KUJO_BENCH_SSG_TMP_DIR")` fallbacking to repo `tmp`.
- Updated `CHANGELOG.md`, `ROADMAP.md`, and `README.md` for the completed benchmark-stability milestone.

## Gotchas (Read This Next Time)
- **Gotcha:** Formatting can spill into unrelated files during this workflow.
  - **Symptom:** `cargo fmt` modified files outside the feature scope (`src/interpreter/native_functions/io.rs`, `src/vm.rs`).
  - **Root cause:** Workspace formatter normalized long-line wrapping in files untouched by feature logic.
  - **Fix:** Reverted unrelated files before commit (`git checkout -- src/interpreter/native_functions/io.rs src/vm.rs`).
  - **Prevention:** Always run `git status --short` after formatting and restore non-scope files before staging.

- **Gotcha:** tmp-dir override is passed through environment and therefore requires UTF-8 path conversion.
  - **Symptom:** Non-UTF8 path values cannot be forwarded to Kujo/Python benchmark scripts.
  - **Root cause:** subprocess env propagation uses `KUJO_BENCH_SSG_TMP_DIR` string values; `Path` must convert with `to_str()`.
  - **Fix:** Added explicit validation in `resolve_tmp_dir_override(...)` and deterministic error message on invalid UTF-8 paths.
  - **Prevention:** Keep path-to-env boundary checks explicit whenever adding path-based CLI overrides for subprocess-driven workflows.

## Things I Learned
- The cleanest cross-language benchmark-root control is a single shared env contract consumed by both benchmark scripts.
- Keeping benchmark-path control in harness (`src/benchmarks/ssg.rs`) avoids script-argument contract churn.
- Existing warning output from `cargo test ssg` includes an unrelated `unused_mut` in `src/vm.rs`; it is pre-existing and outside this feature scope.

## Debug Notes (Only if applicable)
- **Failing test / error:** none for feature tests; smoke run and SSG test slice passed.
- **Repro steps:**
  1. `cargo test ssg`
  2. `cargo run --quiet -- bench-ssg --runs 1 --tmp-dir tmp/kujo_bench_tmp_override_smoke`
- **Breakpoints / logs used:** terminal output from `cargo build`, `cargo test ssg`, and bench command smoke output.
- **Final diagnosis:** tmp-dir path override is correctly propagated end-to-end and benchmark output remains valid.

## Follow-ups / TODO (For Future Agents)
- [ ] Add command-level integration tests for `bench-ssg` CLI failure modes (missing metrics/checksum mismatch/script-not-found) when practical.
- [ ] Track median impact across multi-run samples with and without custom tmp-dir roots in CI-like environments.

## Links / References
- Files touched:
  - `src/main.rs`
  - `src/benchmarks/ssg.rs`
  - `benchmarks/cross-language/bench_ssg.kujo`
  - `benchmarks/cross-language/bench_ssg.py`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `notes/README.md`
