# Kujo Field Notes — Await-all batching + JWT provider fix

**Date:** 2026-02-12
**Session:** 15:33 local
**Branch/Commit:** main / 5c4a708
**Scope:** Implemented the top P0 async quick win (`promise_all`/`await_all` concurrency limiting), added coverage, and fixed full-suite JWT failures caused by `jsonwebtoken` provider configuration. Updated runtime registration and docs so behavior matches roadmap and README claims.

---

## What I Changed
- Added optional `concurrency_limit` support to `Promise.all` / `promise_all` implementation with bounded batching in `src/interpreter/native_functions/async_ops.rs`.
- Registered missing `await_all` builtin alias in `src/interpreter/mod.rs`.
- Added integration tests for bounded await-all behavior and invalid limit validation in `tests/interpreter_tests.rs`.
- Updated `CHANGELOG.md`, `ROADMAP.md`, and `README.md` to document the new async aggregation behavior.
- Fixed JWT test-suite failures by enabling `jsonwebtoken` feature `rust_crypto` in `Cargo.toml`.

## Gotchas (Read This Next Time)
- **Gotcha:** `await_all` existed in planning/docs but was not callable from Kujo code.
  - **Symptom:** `await_all(...)` resolved incorrectly in scripts despite roadmap/readme indicating support.
  - **Root cause:** Native logic existed, but alias registration in `Interpreter::register_builtins()` was missing.
  - **Fix:** Added `self.env.define("await_all", Value::NativeFunction("await_all"))` in `src/interpreter/mod.rs`.
  - **Prevention:** For every native alias, verify both implementation dispatch and builtin registration are present.

- **Gotcha:** `Promise.all(...)` (dotted name) is not a safe form for Kujo test/program calls.
  - **Symptom:** Validation tests using `Promise.all([..], 0)` did not produce expected `Value::Error` checks.
  - **Root cause:** Dotted syntax is parsed as field/method access semantics, not a plain callable identifier path in all contexts.
  - **Fix:** Use `promise_all(...)` / `await_all(...)` aliases in Kujo programs/tests.
  - **Prevention:** Expose identifier-safe aliases for any dotted native names and use aliases in tests.

- **Gotcha:** `jsonwebtoken` 10.x requires an explicit crypto provider path.
  - **Symptom:** Full `cargo test` failed with panic: `Could not automatically determine the process-level CryptoProvider ...` in JWT tests.
  - **Root cause:** Dependency config did not force one provider backend.
  - **Fix:** Set `jsonwebtoken = { version = "10.3.0", features = ["rust_crypto"] }` in `Cargo.toml`.
  - **Prevention:** When upgrading `jsonwebtoken`, pin exactly one provider feature (`rust_crypto` or `aws_lc_rs`) and run full test suite, not only targeted tests.

## Things I Learned
- Native-function correctness in Kujo has three practical checkpoints: implementation dispatch, builtin registration, and user-facing alias ergonomics.
- Async aggregation can be improved incrementally with bounded batching without redesigning VM await semantics.
- Passing targeted tests is insufficient for dependency-level changes; full-suite runs are required because failures may surface in unrelated modules (JWT here).

## Debug Notes (Only if applicable)
- **Failing test / error:** `Could not automatically determine the process-level CryptoProvider from jsonwebtoken crate features...`
- **Repro steps:** Run `cargo test` on main after async changes; JWT tests panic in interpreter integration suite.
- **Breakpoints / logs used:** Full test output from `cargo test`; compared targeted async test pass vs full-suite JWT failures.
- **Final diagnosis:** Dependency feature configuration issue in `Cargo.toml`, not async implementation regressions.

## Follow-ups / TODO (For Future Agents)
- [ ] Add a small dependency-health CI check that scans for security/crypto crates with mandatory feature-provider selection.
- [ ] Consider de-emphasizing dotted builtins in user docs/examples unless parser/runtime semantics are made fully consistent.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `src/interpreter/mod.rs`
  - `tests/interpreter_tests.rs`
  - `Cargo.toml`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
