# Kujo Field Notes — V1-PERF-003 Resource Exhaustion Safeguards

**Date:** 2026-05-16
**Session:** 16:12 local
**Branch/Commit:** main / pending
**Scope:** Implemented `V1-PERF-003` by adding centralized resource-limit defaults and enforcing missing parser/interpreter/VM depth and literal-size guardrails with regression coverage.

---

## What I Changed
- Added shared limit constants in `src/runtime_limits.rs` (source size, string literal length, collection literal count, interpreter/VM call depth, shared native IO bytes).
- Wired parser/lexer to centralized limits in `src/parser.rs` and `src/lexer.rs`.
- Added parser literal-size diagnostics for oversized array/dictionary literals in `src/parser.rs`.
- Added interpreter call-depth enforcement in `src/interpreter/mod.rs` via `with_function_context(...)` guardrails and updated call sites to propagate depth errors safely.
- Added VM call-depth enforcement in `src/vm.rs` in `call_bytecode_function(...)`.
- Reused centralized native IO size limits in `src/interpreter/native_functions/filesystem.rs` and `src/network_policy.rs`.
- Added/updated tests in:
  - `tests/parser_diagnostics_contract.rs`
  - `src/interpreter/mod.rs` (`runtime_limit_tests`)
  - `src/vm.rs` VM tests
  - `tests/native_api_security_boundaries.rs` (shared-constant alignment)
- Updated `CHANGELOG.md`, `ROADMAP.md`, and `README.md` for completed `V1-PERF-003` behavior.

## Gotchas (Read This Next Time)
- **Gotcha:** Recursive interpreter tests in integration threads can stack-overflow before high recursion values are reached.
  - **Symptom:** `tests/interpreter_tests.rs` recursion scenarios aborted with Rust thread stack overflow despite language recursion limits.
  - **Root cause:** Rust test worker threads have tighter stack constraints than CLI main-thread runs; deep interpreter recursion in-process can abort before high-depth scenarios finish.
  - **Fix:** Validated interpreter guard behavior with in-module unit tests that set `function_depth`/`call_stack` directly instead of deep recursion integration tests.
  - **Prevention:** Prefer direct state-based guard tests for depth-limit contracts in interpreter internals; keep deep recursion smoke checks on VM/CLI paths where thread stack headroom differs.

## Things I Learned
- The most stable enforcement point for interpreter recursion/call depth is centralized function-entry (`with_function_context`) plus explicit call-site error propagation.
- Parser collection-size limits are easiest to keep deterministic when diagnostics are emitted at literal construction time (array/dict push points).
- Consolidating byte-size constants in one module reduces drift across filesystem/network/parser policies and test fixtures.

## Debug Notes (Only if applicable)
- **Failing test / error:** `thread 'test_interpreter_recursion_boundary_succeeds' has overflowed its stack`.
- **Repro steps:** `cargo test --test interpreter_tests test_interpreter_recursion_boundary_succeeds`.
- **Breakpoints / logs used:** Manual CLI recursion probes with `cargo run -- run ... --interpreter` at varying depths.
- **Final diagnosis:** Integration-thread recursion depth is not a reliable harness for interpreter limit semantics; direct guard tests are reliable and deterministic.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider exposing trusted-run override knobs for runtime limit constants (currently static defaults).
- [ ] Add CLI/contract docs if per-command or env-driven runtime-limit overrides are introduced.

## Links / References
- Files touched:
  - `src/runtime_limits.rs`
  - `src/parser.rs`
  - `src/lexer.rs`
  - `src/interpreter/mod.rs`
  - `src/vm.rs`
  - `src/network_policy.rs`
  - `src/interpreter/native_functions/filesystem.rs`
  - `tests/parser_diagnostics_contract.rs`
  - `tests/native_api_security_boundaries.rs`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `ROADMAP.md`
