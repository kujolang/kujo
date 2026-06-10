# Kujo Field Notes — bench-ssg command failure-contract hardening

**Date:** 2026-03-12
**Session:** 04:05 local
**Branch/Commit:** main / 4cb078f
**Scope:** Implemented the next v0.11 high-priority benchmark-stability slice by hardening `bench-ssg` command-level failure contracts (missing scripts, missing metrics, checksum mismatch), adding comprehensive harness tests, and synchronizing docs.

---

## What I Changed
- Added preflight script existence checks in `run_ssg_benchmark(...)` in `src/benchmarks/ssg.rs`:
  - immediate error when Kujo benchmark script path is missing
  - immediate error when Python benchmark script path is missing
- Added command-level benchmark harness fixture utilities in `src/benchmarks/ssg.rs` tests:
  - unique workspace-local fixture dirs under `tmp/bench_ssg_harness_tests`
  - executable stub writers with deterministic outputs
- Added comprehensive failure-contract tests for:
  - missing required Kujo metric output (`KUJO_SSG_FILES_PER_SEC`)
  - missing required Python metric output (`PYTHON_SSG_FILES_PER_SEC`)
  - Kujo/Python checksum mismatch rejection
  - missing Kujo/Python script preflight behavior
- Updated release docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** Stub-output fixtures must emit real newlines, not escaped `\\n` sequences.
  - **Symptom:** metric parsing tests failed with missing-metric assertions even though fixture arrays listed the metric keys.
  - **Root cause:** fixture joined lines with escaped newline text (`"\\n"`) instead of actual line separators (`"\n"`), so parser saw one combined line.
  - **Fix:** changed fixture join delimiters to real newlines before writing stub outputs.
  - **Prevention:** when building parser contract fixtures, verify serialized output shape (line-delimited `KEY=VALUE`) before asserting parser behavior.

- **Gotcha:** `cargo fmt` still touches unrelated files in this workspace.
  - **Symptom:** formatter changed unrelated files (`src/interpreter/native_functions/io.rs`, `src/vm.rs`) while this slice only modified `src/benchmarks/ssg.rs`.
  - **Root cause:** workspace-wide formatting spillover.
  - **Fix:** restored unrelated files before staging and kept atomic commit scope.
  - **Prevention:** always run `git status --short` after formatting and `git checkout -- <unrelated files>` before commit.

## Things I Learned
- Preflight path validation in harness code is worth duplicating even when CLI also validates, because it makes lower-level API behavior deterministic and testable.
- Command-level failure coverage can be done entirely with local stub executables; no external benchmark runtime is needed for contract tests.
- Keeping benchmark tests workspace-local (`tmp/...`) aligns with repo operational constraints and avoids environment-sensitive temp behavior.

## Debug Notes (Only if applicable)
- **Failing test / error:** three new tests initially failed (`missing metric` + `checksum mismatch`) due to malformed stub output serialization.
- **Repro steps:** run `cargo test ssg` after adding initial harness fixtures.
- **Breakpoints / logs used:** direct `cargo test ssg` output and assertion messages in `src/benchmarks/ssg.rs`.
- **Final diagnosis:** fixture output contained literal escaped newline text, preventing metric line parsing.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider adding command-level success-path integration test with real `kujo` and benchmark artifacts when test runtime budget allows.
- [ ] Consider extending harness contract tests for malformed numeric values at command level (not only unit parser level).

## Links / References
- Files touched:
  - `src/benchmarks/ssg.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/README.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
