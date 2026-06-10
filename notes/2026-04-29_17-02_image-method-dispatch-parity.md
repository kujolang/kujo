# Kujo Field Notes — Image Method Dispatch Parity

**Date:** 2026-04-29
**Session:** 17:02 local
**Branch/Commit:** main / 1031d84
**Scope:** Unified image method dispatch so `Expr::MethodCall` works for image values in both interpreter and VM paths. Added real conversion integration coverage for PNG/JPEG/WebP round-trips and failure paths.

---

## What I Changed
- Added canonical image method handler in `src/interpreter/mod.rs`:
  - `Interpreter::call_image_method_impl(obj, method, args)` now handles `resize`, `crop`, `rotate`, `flip`, `save`, `to_grayscale`, `blur`, `adjust_brightness`, `adjust_contrast`.
- Updated active method call path in `src/interpreter/mod.rs`:
  - `call_method(...)` now dispatches image methods through `call_image_method_impl(...)` before iterator-method matching.
- Removed duplicate image method branch from `Expr::Call` field-access handling in `src/interpreter/mod.rs` and replaced it with a call to the canonical helper.
- Added VM image method parity in `src/vm.rs`:
  - `OpCode::FieldGet` recognizes `Value::Image` and emits `__image_method_*` marker native functions.
  - `call_native_function_vm(...)` handles `__image_method_*` by restoring receiver from stack and delegating to `Interpreter::call_image_method_impl(...)`.
- Added `tests/image_conversion_integration.rs` with end-to-end file-based tests:
  - PNG -> WebP, JPEG -> WebP, WebP -> PNG.
  - Verifies save result, output file creation, non-empty output, and reloadability.
  - Added failure checks for missing input path, unsupported output extension, and invalid arg types.
- Updated example messaging in `examples/image_processing.kujo` to explicitly show WebP conversion and method-call support.

## Gotchas (Read This Next Time)
- **Gotcha:** Image methods existed, but were not on the active method-call execution path.
  - **Symptom:** `Unknown method: save` / `Unknown method: resize` even though image-method code existed in interpreter.
  - **Root cause:** `Expr::MethodCall` used `call_method(...)`, while image behavior lived in a separate `Expr::Call` field-access branch.
  - **Fix:** Centralized image behavior in `call_image_method_impl(...)` and called it from active `call_method(...)`.
  - **Prevention:** When adding method support for a `Value` type, always verify `Expr::MethodCall` path first; do not rely on legacy call sugar branches.

- **Gotcha:** VM method calls on non-struct receivers require explicit FieldGet + native marker support.
  - **Symptom:** `Cannot access field on non-struct` for `img.save("out.webp")` in VM mode.
  - **Root cause:** VM `FieldGet` handled Channel special-case markers but not Image.
  - **Fix:** Added `Value::Image` case in `OpCode::FieldGet` and `__image_method_*` branch in `call_native_function_vm(...)`.
  - **Prevention:** Treat receiver-bearing methods as a stack-layout contract; update both `FieldGet` and native-call dispatch together.

- **Gotcha:** VM test harness needs builtins populated before executing compiled scripts.
  - **Symptom:** `Undefined global: load_image` in VM integration tests.
  - **Root cause:** VM environment was initialized with `Environment::new()` (no builtin registrations).
  - **Fix:** Seeded VM globals from `Interpreter::new().env` in tests (`vm_env_with_builtins()`).
  - **Prevention:** For VM integration tests using native functions, initialize env from an interpreter instance, not a bare environment.

- **Gotcha:** Missing input path for `load_image` is a runtime error path, not a value assignment path.
  - **Symptom:** Expected env variable assertion failed for missing image input.
  - **Root cause:** `load_image` propagates runtime error and short-circuits execution rather than assigning `Value::Error` into target variable.
  - **Fix:** Assert interpreter `return_value` / VM `Err(...)` for missing input scenario.
  - **Prevention:** Validate error shape by running a small repro script before finalizing assertions for native-error behaviors.

## Things I Learned
- Kujo currently has at least two method-related execution shapes (`Expr::MethodCall` and field-access call sugar). If they diverge, behavior appears "implemented" but still fails in real programs.
- Reusing interpreter helpers inside VM dispatch is a good parity pattern when behavior must stay identical and method semantics are stable.
- VM receiver-method dispatch relies on stack choreography: receiver must survive `FieldGet` and be recovered at call time.
- For native-image operations, end-to-end file conversion tests are the only reliable proof; print-based demos do not validate codec behavior.

## Debug Notes (Only if applicable)
- **Failing test / error:**
  - `Unknown method: save`
  - `Unknown method: resize`
  - `Cannot access field on non-struct`
  - `Undefined global: load_image`
- **Repro steps:**
  - Interpreter: `cargo run -- run --interpreter <script.kujo>` with `img := load_image(...); ok := img.save("out.webp")`
  - VM: `cargo run -- run <script.kujo>` with the same method-call form.
- **Breakpoints / logs used:**
  - Traced `Expr::MethodCall` evaluation in `src/interpreter/mod.rs` and VM `OpCode::FieldGet` / `call_native_function_vm(...)` in `src/vm.rs`.
  - Used targeted command output from `cargo test --test image_conversion_integration`.
- **Final diagnosis:**
  - Interpreter and VM both had dispatch gaps for image method syntax. Existing image logic was partly unreachable in active method-call flow and VM field handling lacked image-specific method dispatch.

## Follow-ups / TODO (For Future Agents)
- [ ] Add a focused VM/interpreter parity test helper to reduce duplicate assertion patterns for receiver-method behavior.
- [ ] Decide whether all non-struct method-bearing `Value` types should standardize on marker-based FieldGet dispatch to avoid per-type drift.

## Links / References
- Files touched:
  - `src/interpreter/mod.rs`
  - `src/vm.rs`
  - `tests/image_conversion_integration.rs`
  - `examples/image_processing.kujo`
  - `docs/IMAGE_CONVERSION_AGENT_HANDOFF.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
