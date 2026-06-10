# Kujo Field Notes — V1-COMP-001 parity gate import + unsupported surface alignment

**Date:** 2026-05-08
**Session:** 17:05 local
**Branch/Commit:** main / 4f52e54
**Scope:** Completed roadmap item `V1-COMP-001` by closing interpreter/VM/compiler parity drift for imports and struct generator methods, expanding parity coverage, and wiring an explicit CI parity gate.

---

## What I Changed
- Added shared error helper `unsupported_struct_generator_method_message(...)` in `src/errors.rs`.
- Updated compiler struct-method lowering in `src/compiler.rs` to reject generator methods in structs with deterministic error text.
- Updated interpreter struct-definition handling in `src/interpreter/mod.rs` to reject struct generator methods with the same message.
- Added VM import execution support in `src/vm.rs`:
  - compiler now lowers `import` and `from ... import ...` into VM native op paths (`__vm_import_all`, `__vm_import_symbol`)
  - VM import handlers now call the shared module loader and bind imported values into the active scope.
- Added parity regressions in `tests/vm_interpreter_parity_surfaces.rs`:
  - `vm_and_interpreter_match_import_export_surface`
  - `vm_and_interpreter_error_on_unsupported_struct_generator_method`
- Replaced narrow parity doc with broader matrix in `docs/VM_INTERPRETER_PARITY_MATRIX.md`.
- Added dedicated CI parity job in `.github/workflows/ci-release-gate.yml`.
- Updated `CHANGELOG.md`, `README.md`, and `ROADMAP.md` for the completed roadmap item.

## Gotchas (Read This Next Time)
- **Gotcha:** Compiler import no-op masked VM/interpreter drift until explicit parity coverage was added.
  - **Symptom:** Interpreter successfully imported `answer`, but VM returned `Undefined variable: answer` for the same script.
  - **Root cause:** `Stmt::Import` was effectively ignored in compiler lowering for VM execution paths.
  - **Fix:** Lowered imports to VM call-native import ops and implemented VM handlers backed by `ModuleLoader`.
  - **Prevention:** Treat `Stmt::Import` as a parity surface and keep direct regression coverage in `tests/vm_interpreter_parity_surfaces.rs`.

- **Gotcha:** Unsupported struct generator methods produced divergent and misleading errors.
  - **Symptom:** Interpreter returned `Unknown method: emit` while VM returned `Yield can only be used inside generator functions`.
  - **Root cause:** Interpreter stored generator methods in struct metadata but method dispatch only accepted `Value::Function`; VM executed lowered bytecode and failed later at `yield`.
  - **Fix:** Centralized unsupported-surface message and rejected generator struct methods explicitly in both compiler and interpreter paths.
  - **Prevention:** For unsupported surfaces, enforce early and deterministic rejection in both runtime paths and lock with a negative parity test.

## Things I Learned
- `OpCode::CallNative` in VM is a critical parity choke-point for statement-lowered runtime effects; import support was most safely added there without introducing a new opcode surface.
- Parity matrix docs were lagging far behind the actual test suite; keeping docs+tests updated in the same commit makes roadmap completion claims auditable.
- Negative parity tests are just as important as happy-path parity tests for unsupported features.

## Debug Notes (Only if applicable)
- **Failing test / error:** `vm_and_interpreter_match_import_export_surface` initially failed with `vm execution failed: Some("Undefined variable: answer")`.
- **Repro steps:** Added parity test that creates a temp module file, runs `from <module> import answer`, and checks shared boolean result.
- **Breakpoints / logs used:** Directly compared interpreter and VM outputs via targeted parity test runs and inspected `Stmt::Import` compiler lowering + VM `CallNative` dispatch.
- **Final diagnosis:** VM path had no import lowering/handler, causing imported names to remain unresolved.

## Follow-ups / TODO (For Future Agents)
- [ ] Add parity coverage for function-local import behavior (explicit local-scope binding expectations) to ensure import-scope semantics remain aligned.
- [ ] Keep parity matrix entries synchronized whenever `tests/vm_interpreter_parity_surfaces.rs` gains new surfaces.

## Links / References
- Files touched:
  - `src/errors.rs`
  - `src/compiler.rs`
  - `src/interpreter/mod.rs`
  - `src/vm.rs`
  - `tests/vm_interpreter_parity_surfaces.rs`
  - `.github/workflows/ci-release-gate.yml`
  - `docs/VM_INTERPRETER_PARITY_MATRIX.md`
  - `CHANGELOG.md`
  - `README.md`
  - `ROADMAP.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `docs/VM_INTERPRETER_PARITY_MATRIX.md`
