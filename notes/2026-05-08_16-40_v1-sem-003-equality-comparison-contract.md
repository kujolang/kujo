# Kujo Field Notes — V1-SEM-003 Equality/Comparison Contract

**Date:** 2026-05-08
**Session:** 16:40 local
**Branch/Commit:** main / (working tree)
**Scope:** Implemented `V1-SEM-003` by centralizing equality and ordering semantics in `Value` and aligning interpreter+VM behavior with parity and regression coverage.

---

## What I Changed
- Added centralized comparison helpers in `src/interpreter/value.rs`:
  - `Value::equals(...)`
  - `Value::compare_order(...)`
- Defined shared runtime semantics for cross-type numeric equality (`int`/`float`), deep array/dictionary equality, callable identity equality, and deterministic ordering-type rejection.
- Routed interpreter binary comparison logic in `src/interpreter/mod.rs` through the shared helpers.
- Routed VM comparison/equality op paths in `src/vm.rs` through the same shared helpers.
- Added parity regressions in `tests/vm_interpreter_parity_surfaces.rs` for:
  - cross-type numeric equality + string ordering success
  - collection + callable equality success
  - unsupported ordering failures (bool/bool and int/string)
- Added focused helper unit tests in `src/interpreter/value.rs` for equality and ordering edge cases.
- Updated behavior docs in `docs/LANGUAGE_SPEC.md`, `README.md`, `CHANGELOG.md`, and marked `V1-SEM-003` complete in `ROADMAP.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** VM closure values are `BytecodeFunction`, not interpreter `Function`.
  - **Symptom:** Callable equality parity tests passed in interpreter but failed in VM for `adder == adder`.
  - **Root cause:** Equality logic initially handled interpreter function variants but not VM bytecode closure values.
  - **Fix:** Added explicit `Value::BytecodeFunction` equality arm using chunk equality plus captured-binding identity (`Arc::ptr_eq` per captured key).
  - **Prevention:** Any callable semantic change must cover both `Function` and `BytecodeFunction` variants.
- **Gotcha:** Constant-folding can hide runtime comparison errors in tests.
  - **Symptom:** `return true < false` did not fail in VM even after ordering hardening.
  - **Root cause:** optimizer folded literal-only expressions before runtime compare-op execution.
  - **Fix:** Used non-literal execution (`func compare(left, right) { return left < right }`) to force runtime path.
  - **Prevention:** Negative runtime-op tests should avoid pure literal expressions when optimizer folding is enabled.

## Things I Learned
- Equality semantics were fragmented across interpreter helper logic, interpreter binary-op logic, and VM opcode logic; centralizing in `Value` removed drift risk.
- Map equality needs representation-agnostic behavior because VM can promote dictionaries into optimized int/dense encodings.
- Ordering should remain strict (`numeric` and `string` only) while equality can be broader and well-defined across structured data.

## Debug Notes (Only if applicable)
- **Failing test / error:** `vm_and_interpreter_define_collection_and_callable_equality_contract` failed in VM with parity bool unset.
- **Repro steps:** `cargo test --test vm_interpreter_parity_surfaces vm_and_interpreter_define_collection_and_callable_equality_contract -- --nocapture`
- **Breakpoints / logs used:** test assertion traces + variant inspection in `src/interpreter/value.rs` and `src/vm.rs`.
- **Final diagnosis:** VM function values require `BytecodeFunction` identity semantics; interpreter-only callable equality was insufficient.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider whether broader host-handle identity equality (database/socket/task/resource wrappers) should be explicitly documented in the language spec or deferred to a dedicated semantics item.
- [ ] Consider adding dedicated parser/optimizer guard tests for runtime-error expectations in comparison-heavy expressions with mixed constant/non-constant operands.

## Links / References
- Files touched:
  - `src/interpreter/value.rs`
  - `src/interpreter/mod.rs`
  - `src/vm.rs`
  - `tests/vm_interpreter_parity_surfaces.rs`
  - `docs/LANGUAGE_SPEC.md`
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `docs/LANGUAGE_SPEC.md`
