# Kujo Field Notes — V1-SEC-003 process and shell hardening

**Date:** 2026-05-12
**Session:** 22:40 local
**Branch/Commit:** main / e23a5b9
**Scope:** Implemented V1-SEC-003 by hardening shell/process native APIs with bounded execution controls, structured result contracts, and capability-aware test/doc follow-through.

---

## What I Changed
- Added centralized bounded process execution helpers in `src/interpreter/native_functions/system.rs`:
  - `ProcessExecOptions` parsing for optional options dict (`timeout_ms`, `max_output_bytes`, `inherit_env`, `env_allow`, `env_deny`, `env`)
  - bounded stdout/stderr collection with truncation flags
  - timeout enforcement with process kill/wait behavior
  - env allow/deny/inject policy application
- Kept `execute(...)` string-return contract for success, but moved it onto bounded shell execution with deterministic error objects for timeout/output-overflow/non-zero exit.
- Added `execute_status(...)` in `src/interpreter/native_functions/system.rs` to expose structured shell execution status.
- Moved `spawn_process(...)` and `pipe_commands(...)` to the same bounded runner with optional options dict support and deterministic `ProcessResult` behavior.
- Updated native capability mapping in `src/interpreter/capabilities.rs` so `execute_status` is shell-exec scoped.
- Registered builtin symbol in `src/interpreter/mod.rs` for `execute_status`.
- Updated function signatures in `src/type_checker.rs` to reflect 1-2 arg forms for `execute`, `execute_status`, `spawn_process`, `pipe_commands`.
- Updated dispatch/unit contract tests in `src/interpreter/native_functions/mod.rs` and `src/interpreter/native_functions/system.rs` for new arity/messages/return-shape.
- Expanded security boundary integration tests in `tests/native_api_security_boundaries.rs`:
  - shell allow path coverage
  - direct argv non-shell-expansion behavior
  - timeout kill behavior
  - output truncation flags
  - env allow/deny/inject policy enforcement
- Updated docs and release tracking in `README.md`, `docs/NATIVE_API_SECURITY_POSTURE.md`, `docs/STANDARD_LIBRARY_REFERENCE.md`, `CHANGELOG.md`, and `ROADMAP.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** Process API arity/argument-contract changes must be applied in three independent layers.
  - **Symptom:** Runtime behaved correctly but tests still failed on argument-count/type expectations.
  - **Root cause:** Contracts are validated by runtime handlers (`system.rs`), type metadata (`type_checker.rs`), and modular dispatch tests (`native_functions/mod.rs`) separately.
  - **Fix:** Updated all three layers to 1-2 arg contract (`[required, optional_options]`) and aligned expected error messages.
  - **Prevention:** Any native API signature change should include a checklist for runtime handler + type checker signature + dispatch contract tests.

- **Gotcha:** Integration `.kujo` scripts used by Rust tests cannot assume test-framework assertion helpers.
  - **Symptom:** Script failed with undefined function for `assert_true` in normal interpreter run mode.
  - **Root cause:** `assert_true` is not universally available in plain runtime execution for these harness scripts.
  - **Fix:** Replaced script-side assertions with printed values and validated expectations in Rust test assertions.
  - **Prevention:** Keep boundary scripts minimal (produce output), and assert behavior from Rust harness side.

- **Gotcha:** Full `cargo test` can fail intermittently on benchmark timing thresholds.
  - **Symptom:** `benchmarks::timer::tests::test_timer` failed once on elapsed-time threshold, then passed on rerun.
  - **Root cause:** Time-budget assertion sensitivity to local machine scheduling jitter.
  - **Fix:** Re-ran targeted timer test and then reran full suite in quiet mode for deterministic pass evidence.
  - **Prevention:** When this test fails in isolation without related code changes, treat as timing flake and confirm by rerun before deeper triage.

## Things I Learned
- `execute(...)` can keep legacy string-return ergonomics while still being hardened by central process controls.
- `execute_status(...)` is the right non-breaking path for structured shell result contracts.
- Unifying `spawn_process` and `pipe_commands` under one bounded runner simplifies security and makes edge behavior deterministic.
- Process security hardening is easier to maintain when timeout/output/env policies are one shared implementation rather than per-builtin branches.

## Debug Notes (Only if applicable)
- **Failing test / error:** `benchmarks::timer::tests::test_timer` failed once with `assertion failed: elapsed < Duration::from_millis(50)`.
- **Repro steps:** `cargo test` on local machine during full-suite run.
- **Breakpoints / logs used:** N/A (test rerun strategy).
- **Final diagnosis:** Intermittent timing sensitivity; unrelated to process/shell hardening changes.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider relaxing or stabilizing timer-threshold assertions in `src/benchmarks/timer.rs` if local/CI flakes continue.
- [ ] Add a small process-options-focused API example to user-facing docs/tutorial snippets if process APIs are promoted further.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/system.rs`
  - `src/interpreter/native_functions/mod.rs`
  - `src/interpreter/mod.rs`
  - `src/interpreter/capabilities.rs`
  - `src/type_checker.rs`
  - `tests/native_api_security_boundaries.rs`
  - `README.md`
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
  - `docs/STANDARD_LIBRARY_REFERENCE.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `README.md`
  - `ROADMAP.md`
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
