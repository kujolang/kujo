# Kujo Field Notes — Undefined Identifier Runtime Errors

**Date:** 2026-05-06
**Session:** 17:07 local
**Branch/Commit:** main / unknown
**Scope:** Implemented `V1-RUN-001` by replacing implicit identifier-to-string fallback with runtime errors across interpreter/VM parity surfaces.

---

## What I Changed
- Changed `src/interpreter/mod.rs` so `Expr::Identifier` returns `Value::Error("Undefined variable: <name>")` when lookup misses instead of `Value::Str(name)`.
- Added targeted error propagation through common expression wrappers so undefined names fail inside returns, binary expressions, conditions, functions, closures, interpolation, calls, fields, index expressions, arrays, dictionaries, and method receivers.
- Kept native `Value::Error` bindings compatible for `let result := native_call(...)` tests that intentionally inspect error values.
- Changed `src/vm.rs` to use one undefined-variable message helper for globals/locals, surface returned error values through cooperative execution, and exclude `LoadVar`/`LoadGlobal` from top-level script JIT until JIT lookup semantics are parity-covered.
- Added VM/interpreter parity regression coverage in `tests/vm_interpreter_parity_surfaces.rs`.
- Updated image conversion failure tests to inspect interpreter `return_value` where method errors now stop evaluation.

## Gotchas (Read This Next Time)
- **Gotcha:** `Value::Error` is not always a fatal language error.
  - **Symptom:** `tests/interpreter_tests.rs` had many failures after making `let` return early on any error value.
  - **Root cause:** Native helpers intentionally return `Value::Error` as data so scripts/tests can bind and inspect it.
  - **Fix:** Let/const/assignment still bind the error value while setting `return_value`; undefined identifiers still surface as runtime errors.
  - **Prevention:** Before changing statement-level error propagation, check native API tests for error-as-value contracts.
- **Gotcha:** Script JIT can bypass checked VM errors.
  - **Symptom:** VM parity tests for top-level undefined identifiers completed successfully even after `LoadGlobal` had the right error message.
  - **Root cause:** Tiny scripts were admitted to top-level script JIT, which did not preserve undefined variable errors.
  - **Fix:** Gate `LoadVar` and `LoadGlobal` out of script JIT until JIT lookup parity is explicit.
  - **Prevention:** When changing VM runtime semantics, audit bytecode VM, nested function execution, and JIT admission paths.
- **Gotcha:** Channel methods were split between call paths.
  - **Symptom:** `chan.send("hello")` became an error once expression-statement errors surfaced.
  - **Root cause:** Channel send/receive methods were implemented for the legacy `Expr::Call` field-access path but not the active `Expr::MethodCall` dispatcher.
  - **Fix:** Added channel `send`/`receive` support to `call_method`.
  - **Prevention:** For method behavior, wire `Expr::MethodCall` and VM `FieldGet`/native-call handling together.

## Things I Learned
- Identifier fallback was interpreter-only; VM bytecode lookup already had missing global/local errors but needed message normalization and JIT gating.
- `execute_until_suspend` is the production `kujo run` VM path, so returned `Value::Error` values must be converted to execution errors there.
- Tests that assert native failure behavior often inspect either environment bindings or `return_value`; changing one path can expose compatibility assumptions.

## Debug Notes (Only if applicable)
- **Failing test / error:** Initial parity failures expected VM errors but got successful completion for top-level undefined identifiers.
- **Repro steps:** `cargo test --test vm_interpreter_parity_surfaces`
- **Breakpoints / logs used:** Inspected compiler output paths and VM script-JIT admission logic in `src/vm.rs`.
- **Final diagnosis:** Top-level script JIT was executing variable-load scripts before bytecode VM missing-binding checks could run.

## Follow-ups / TODO (For Future Agents)
- [ ] Add explicit JIT parity tests before re-enabling script JIT for `LoadVar`/`LoadGlobal`.
- [ ] Consider a dedicated runtime error type that separates fatal semantic errors from native error-as-value results.

## Links / References
- Files touched:
  - `src/interpreter/mod.rs`
  - `src/vm.rs`
  - `tests/vm_interpreter_parity_surfaces.rs`
  - `tests/image_conversion_integration.rs`
  - `README.md`
  - `docs/LANGUAGE_SPEC.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `notes/GOTCHAS.md`
  - `notes/README.md`
- Related docs:
  - `ROADMAP.md`
  - `docs/LANGUAGE_SPEC.md`
