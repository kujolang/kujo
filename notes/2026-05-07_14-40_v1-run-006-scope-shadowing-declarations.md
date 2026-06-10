# Kujo Field Notes — V1-RUN-006 Scope, Shadowing, and Declaration Rules

**Date:** 2026-05-07
**Session:** 14:40 local
**Branch/Commit:** main / 53841df
**Scope:** Implemented V1-RUN-006 by enforcing duplicate declaration failures, tightening lexical scope boundaries for control-flow/function contexts, and adding VM/interpreter parity regressions.

---

## What I Changed
- Added `Environment::define_with_kind_checked(...)` in `src/interpreter/environment.rs` to reject duplicate declarations in the current scope.
- Updated interpreter declaration paths in `src/interpreter/mod.rs` so `let` pattern binding and `const` declarations use checked declarations and surface deterministic runtime errors.
- Updated compiler scope/declaration handling in `src/compiler.rs`:
  - added local-scope duplicate checks (`declare_local`, `has_local_in_current_scope`)
  - enabled explicit local-slot mode for function/method/lambda compilers
  - rejected duplicate parameter names in the same function scope
  - scoped `if`/`while`/`loop`/`for` compilation depth so inner declarations do not leak
  - kept root script resolution on runtime environment bindings.
- Updated VM declaration opcode handling in `src/vm.rs` so `DefineGlobal` uses checked declaration behavior.
- Added/updated parity regressions in `tests/vm_interpreter_parity_surfaces.rs` for:
  - duplicate same-scope `let` and `const` failures
  - function-local control-flow binding leakage rejection
  - loop-variable lifetime isolation
  - inner shadowing success and nearest lexical closure capture.
- Updated docs/contracts in `docs/LANGUAGE_SPEC.md`, `README.md`, `CHANGELOG.md`, and `ROADMAP.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** VM scope behavior is split between compile-time local-slot resolution and runtime environment-name resolution.
  - **Symptom:** A naive scope fix can make function-local and top-level semantics diverge, especially in loop/branch bodies.
  - **Root cause:** Function compilers use local slots, but root script compilation still relies heavily on named environment stores (`StoreVar`/`DefineGlobal`).
  - **Fix:** Add explicit compiler-mode handling (`uses_local_slots`) and only rely on local-slot declarations where frames exist.
  - **Prevention:** When touching scope logic, verify both interpreter and VM through parity tests, not just one runtime.

- **Gotcha:** Duplicate-declaration checks must happen at declaration APIs, not at assignment APIs.
  - **Symptom:** Same-scope `let`/`const` redefinitions silently overwrite unless guarded.
  - **Root cause:** Existing `define_with_kind(...)` inserts directly into current scope maps.
  - **Fix:** Introduce `define_with_kind_checked(...)` and route declaration call sites through it.
  - **Prevention:** Keep mutability/reassignment checks (`assign_checked`) separate from declaration checks (`define_with_kind_checked`).

## Things I Learned
- The cleanest low-risk scope hardening path is to centralize declaration rules in the environment and compiler symbol declaration helpers, then assert parity from black-box tests.
- `for` loop variable lifetime is easiest to keep sane in VM by treating loop variable declaration as local-scope declaration in function compilers.
- Closure capture parity is sensitive to shadowing rules; explicit nearest-lexical capture tests are required whenever scope resolution changes.

## Debug Notes (Only if applicable)
- **Failing test / error:** Initial parity failures on new regressions: duplicate declarations were accepted, and loop variable leakage persisted (`expected VM execution to report an error: ()`).
- **Repro steps:** `cargo test --test vm_interpreter_parity_surfaces` after adding V1-RUN-006 tests.
- **Breakpoints / logs used:** Read compiler assignment/declaration lowering and VM `StoreVar`/`StoreLocal`/`DefineGlobal` behavior.
- **Final diagnosis:** Missing declaration guard and incomplete lexical-depth handling in compiler declaration resolution for control-flow/loop scopes.

## Follow-ups / TODO (For Future Agents)
- [ ] Revisit root-script top-level lexical block scoping in VM to fully match interpreter behavior for all control-flow surfaces.
- [ ] Consider propagating duplicate-declaration checking to VM pattern-binding helper paths where declarations are synthesized dynamically.

## Links / References
- Files touched:
  - `src/interpreter/environment.rs`
  - `src/interpreter/mod.rs`
  - `src/compiler.rs`
  - `src/vm.rs`
  - `tests/vm_interpreter_parity_surfaces.rs`
  - `docs/LANGUAGE_SPEC.md`
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `docs/LANGUAGE_SPEC.md`
  - `ROADMAP.md`
