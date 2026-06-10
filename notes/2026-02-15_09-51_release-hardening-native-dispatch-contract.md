# Kujo Field Notes — Release Hardening Native Dispatch Contract

**Date:** 2026-02-15
**Session:** 09:51 local
**Branch/Commit:** main / dc30eaa
**Scope:** Hardened modular native dispatch behavior so unknown builtin names fail explicitly instead of silently returning `0`. Added dispatcher-level regression tests and updated release docs for the new contract.

---

## What I Changed
- Updated unknown-native fallback in `src/interpreter/native_functions/mod.rs` from `Value::Int(0)` to `Value::Error(format!("Unknown native function: {}", name))`.
- Added dispatcher tests in `src/interpreter/native_functions/mod.rs`:
  - `test_unknown_native_function_returns_explicit_error`
  - `test_release_hardening_builtin_dispatch_coverage_for_recent_apis`
- Validated with targeted tests and full suite (`cargo test`).
- Updated release docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** Native dispatcher tests run in both `src/lib.rs` and `src/main.rs` test targets.
  - **Symptom:** The same new test appears twice in output, which can look like duplicate definitions.
  - **Root cause:** Kujo compiles and runs unit tests for both crate targets.
  - **Fix:** No code fix required; this is expected behavior.
  - **Prevention:** Treat duplicate test-name output across `lib`/`main` as normal unless one target fails.

- **Gotcha:** A declared builtin can still silently degrade if dispatcher fallback is permissive.
  - **Symptom:** Runtime returns `0` for a builtin that should error or perform work.
  - **Root cause:** Missing handler arm in `src/interpreter/native_functions/*.rs` with a permissive unknown fallback path.
  - **Fix:** Make unknown-native dispatch explicit error and add contract tests over high-risk builtins.
  - **Prevention:** When adding/hardening builtins, verify all three surfaces: `get_builtin_names()`, `register_builtins()`, and native handler coverage.

## Things I Learned
- For release hardening, dispatcher unknown behavior is a correctness boundary, not a convenience fallback.
- Rule: unknown native names should fail loudly; silent fallback masks registration/dispatch drift.
- Dispatcher-level coverage tests are cheap and catch regression classes that name-list parity checks alone do not.

## Debug Notes (Only if applicable)
- **Failing test / error:** Not applicable (no failing tests in this session).
- **Repro steps:** Not applicable.
- **Breakpoints / logs used:** Used targeted `cargo test <name>` plus full `cargo test` run.
- **Final diagnosis:** Existing behavior was valid Rust but unsafe for API hardening goals due to silent fallback semantics.

## Follow-ups / TODO (For Future Agents)
- [ ] Expand dispatcher contract coverage from “recent/high-risk builtins” to a generated full builtin-to-handler contract check.
- [ ] Consider adding a CI guard that fails if any declared builtin reaches unknown-native fallback.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `notes/GOTCHAS.md`
  - `notes/README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `.github/AGENT_INSTRUCTIONS.md`
