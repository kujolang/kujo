# Kujo Field Notes — VM Map Missing-Key Runtime Errors

**Date:** 2026-05-06
**Session:** 12:25 local
**Branch/Commit:** main / 1c5dab1
**Scope:** Completed `V1-TEST-001` by making missing map keys runtime errors instead of silent `Int(0)`/`Null` fallbacks. Added VM/interpreter parity coverage for missing keys, invalid key types, and map update success paths.

---

## What I Changed
- Added centralized VM map/index read handling in `src/vm.rs` through `VM::get_indexed_value`.
- Routed normal `IndexGet`, optimized `IndexGetInPlace`, and nested bytecode-call `IndexGetInPlace` execution through the shared helper.
- Changed interpreter dictionary index reads in `src/interpreter/mod.rs` so missing keys return `Value::Error("Missing map key: ...")`.
- Fixed compiler storage for index assignment on captured variables in `src/compiler.rs` by using `StoreVar` for upvalues.
- Added VM/interpreter parity tests in `tests/vm_interpreter_parity_surfaces.rs` for missing string keys, missing integer keys, nested missing keys, invalid key types, and local/nested/captured map updates.
- Updated `CHANGELOG.md`, `README.md`, `ROADMAP.md`, `docs/LANGUAGE_SPEC.md`, and `docs/VM_INTERPRETER_PARITY_MATRIX.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** VM index-read behavior exists in multiple execution loops.
  - **Symptom:** Fixing only the primary `OpCode::IndexGet` match would miss optimized local reads and nested bytecode-call execution.
  - **Root cause:** `IndexGetInPlace` is implemented in the main VM loop and again in the bytecode-function call path used from JIT/native integration.
  - **Fix:** Add one shared `VM::get_indexed_value` helper and call it from every index-read opcode arm.
  - **Prevention:** When changing VM opcode semantics, search for every opcode arm, not only the first match in the main loop.
- **Gotcha:** Regular `func name(...)` declarations are not closure-capturing in the interpreter.
  - **Symptom:** A captured-map update test using a nested named function failed with `Invalid index operation`.
  - **Root cause:** `Stmt::FuncDef` stores `Value::Function(..., None)`, while anonymous `func(...) { ... }` expressions capture environment state.
  - **Fix:** Use `bump := func() { ... }` for closure-capture parity tests.
  - **Prevention:** Use function expressions when a test requires captured locals; named nested functions are a separate language gap.

## Things I Learned
- Missing optimized integer-map entries can surface as dense/sparse storage variants (`DenseIntDict`, `DenseIntDictInt`, `DenseIntDictIntFull`), so missing-key semantics need to cover all optimized map representations.
- `DenseIntDictInt` uses `None` for sparse integer-map holes, so treating `None` as a missing key is the safe behavior for this roadmap item.
- Captured index assignment in compiler fallback must store through `StoreVar`, not `StoreGlobal`, when the indexed object is an upvalue.

## Debug Notes (Only if applicable)
- **Failing test / error:** `vm::tests::test_sum_int_map_until_local_in_place_missing_key_errors` previously panicked with `Expected runtime error, got value: Int(0)`.
- **Repro steps:** `cargo test vm::tests::test_sum_int_map_until_local_in_place_missing_key_errors -- --nocapture`.
- **Breakpoints / logs used:** Traced `src/compiler.rs` loop pattern matching and VM `IndexGet`/`IndexGetInPlace` arms with source inspection and targeted cargo tests.
- **Final diagnosis:** The top-level failing program did not always use the fused local-map opcode path; general map reads could return `Null`, and unsupported arithmetic later turned that into `Int(0)`.

## Follow-ups / TODO (For Future Agents)
- [ ] `cargo fmt --check` currently reports broad pre-existing formatting drift across unrelated files; avoid whole-repo formatting churn unless that is the selected roadmap item.
- [ ] Consider a later roadmap item for named nested function closure capture if language semantics require it.

## Links / References
- Files touched:
  - `src/vm.rs`
  - `src/compiler.rs`
  - `src/interpreter/mod.rs`
  - `tests/vm_interpreter_parity_surfaces.rs`
  - `CHANGELOG.md`
  - `README.md`
  - `ROADMAP.md`
  - `docs/LANGUAGE_SPEC.md`
  - `docs/VM_INTERPRETER_PARITY_MATRIX.md`
- Related docs:
  - `ROADMAP.md`
  - `docs/LANGUAGE_SPEC.md`
  - `docs/VM_INTERPRETER_PARITY_MATRIX.md`
