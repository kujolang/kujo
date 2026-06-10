# Kujo Field Notes — bench-ssg benchmark harness and execution gotchas

**Date:** 2026-02-13
**Session:** 23:03 local
**Branch/Commit:** main / 928dd5a
**Scope:** Implemented a new async SSG benchmark command (`bench-ssg`) with Kujo/Python benchmark artifacts, parser/validator support, tests, and roadmap/changelog/readme updates. Debugged runtime failures during harness bring-up and normalized temporary file behavior to workspace-local `tmp/`.

---

## What I Changed
- Added new CLI subcommand in `src/main.rs`:
  - `bench-ssg`
  - optional Python comparison: `--compare-python`
  - configurable scripts: `--kujo-script`, `--python-script`
- Added benchmark module and exports:
  - `src/benchmarks/ssg.rs`
  - `src/benchmarks/mod.rs` (`pub mod ssg;`, `pub use ssg::run_ssg_benchmark;`)
- Added benchmark artifacts:
  - `benchmarks/cross-language/bench_ssg.kujo`
  - `benchmarks/cross-language/bench_ssg.py`
- Implemented SSG benchmark result parsing/validation in `src/benchmarks/ssg.rs`:
  - metric parsing (`*_FILES`, `*_BUILD_MS`, `*_FILES_PER_SEC`)
  - checksum validation and file-count equivalence checks
  - speedup computation helper
- Added comprehensive tests in `src/benchmarks/ssg.rs` for metric parsing and speedup edge cases.
- Updated docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Committed incrementally and pushed:
  - `9b35cad` (`:package: NEW: add bench-ssg command with cross-language SSG benchmark harness`)
  - `cdb08a3` (`:book: DOC: document bench-ssg roadmap status and usage guidance`)
  - `928dd5a` (`:ok_hand: IMPROVE: format vm scheduler test call wrapping`)

## Gotchas (Read This Next Time)
- **Gotcha:** `bench-ssg` subprocess execution is sensitive to working directory when script paths are relative.
  - **Symptom:** subprocess spawn failed with `No such file or directory (os error 2)` after changing benchmark runner internals.
  - **Root cause:** benchmark scripts were passed as relative paths, but subprocess `current_dir` resolved to a non-repo location.
  - **Fix:** added `determine_workspace_root(...)` in `src/benchmarks/ssg.rs` and always executed subprocesses from detected repo root.
  - **Prevention:** for benchmark wrappers that shell out to Kujo/Python, normalize execution cwd explicitly before process spawn.

- **Gotcha:** System temp paths trigger avoidable local permission friction.
  - **Symptom:** repeated permission prompts when Python benchmark used OS temp directories.
  - **Root cause:** `tempfile.mkdtemp(...)` wrote outside workspace-controlled paths.
  - **Fix:** changed Python benchmark to write under `workspace/tmp` (`benchmarks/cross-language/bench_ssg.py`) and ensured Kujo benchmark creates `tmp` root (`create_dir("tmp")`).
  - **Prevention:** prefer repository-local `tmp/` for generated benchmark files unless OS temp behavior is explicitly required.

- **Gotcha:** Array append in this benchmark script path must use `push(array, value)` instead of method-call syntax.
  - **Symptom:** runtime error: `Cannot access field on non-struct` while running `bench_ssg.kujo`.
  - **Root cause:** used `array.push(...)` method syntax in script; this path resolved like field access and failed.
  - **Fix:** replaced all appends with `array := push(array, value)`.
  - **Prevention:** in benchmark scripts, default to builtin functional append (`push`) unless method-call support is explicitly validated for that value type/path.

## Things I Learned
- Benchmark harnesses should validate not only timings but also workload equivalence (file count + checksum) before reporting speedup.
- CWD handling needs to be treated as first-class in cross-language benchmark commands; otherwise behavior depends on invocation location.
- Repository-local `tmp/` reduces environment-specific permission issues and improves reproducibility.
- **Justified behavior:** an isolated formatting-only change in `src/vm.rs` was intentionally committed separately to keep feature commits focused and reviewable.

## Debug Notes (Only if applicable)
- **Failing test / error:** `Runtime Error: Cannot access field on non-struct`
- **Repro steps:**
  1. Run `cargo run -- bench-ssg --compare-python`
  2. Observe runtime error from `benchmarks/cross-language/bench_ssg.kujo`
- **Breakpoints / logs used:**
  - direct reruns of the command after each script/runtime change
  - `git --no-pager diff -- src/vm.rs` to isolate unrelated edits
- **Final diagnosis:** benchmark script used method-style array append and hit field-access behavior; replacing with `push(array, value)` resolved runtime failure.

## Follow-ups / TODO (For Future Agents)
- [ ] Add `--tmp-dir` option to `bench-ssg` for explicit output-root control in constrained CI environments.
- [ ] Add integration test coverage for `bench-ssg` command-level output parsing and failure modes (missing metrics, checksum mismatch, missing scripts).
- [ ] Track `bench-ssg` performance over time in benchmark result artifacts for regression analysis.

## Links / References
- Files touched:
  - `src/main.rs`
  - `src/benchmarks/mod.rs`
  - `src/benchmarks/ssg.rs`
  - `benchmarks/cross-language/bench_ssg.kujo`
  - `benchmarks/cross-language/bench_ssg.py`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
  - `notes/README.md`
