# Kujo Field Notes — V1-RUN-005 Mutability Enforcement

**Date:** 2026-05-07
**Session:** 14:13 local
**Branch/Commit:** main / 9fac954
**Scope:** Enforced `let`/`const` immutability semantics across interpreter and VM/compiler paths, including in-place mutation guards and parity regressions.

---

## What I Changed
- Added binding-kind metadata to interpreter environments in `src/interpreter/environment.rs`.
- Routed interpreter assignment/mutation through binding-aware checks in `src/interpreter/mod.rs`.
- Extended bytecode and compiler binding metadata in `src/bytecode.rs` and `src/compiler.rs`.
- Enforced immutable local/global reassign and in-place mutation checks in VM execution paths in `src/vm.rs`.
- Added VM/interpreter parity regressions in `tests/vm_interpreter_parity_surfaces.rs`.
- Added direct environment mutability contract tests in `tests/interpreter_tests.rs`.
- Updated language/runtime docs in `docs/LANGUAGE_SPEC.md`, `README.md`, `CHANGELOG.md`, and `ROADMAP.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** Dead-store elimination can accidentally bypass semantic checks.
  - **Symptom:** Immutable reassign/mutation failures did not trigger in some compiled code paths.
  - **Root cause:** Assignment elision in `compile_stmt` removed stores that now carry required mutability enforcement behavior.
  - **Fix:** Removed the unsound let/assignment dead-store elision for this path.
  - **Prevention:** Treat assignment/store opcodes as semantic boundaries once runtime checks are attached; do not optimize them away without proving equivalence.

- **Gotcha:** In-place mutation semantics need explicit prechecks for globals.
  - **Symptom:** Index/mutation paths could mutate through immutable global bindings when checks only existed at `StoreGlobal`.
  - **Root cause:** In-place ops mutate the container before or independent of scalar reassignment checks.
  - **Fix:** Added `EnsureMutableGlobalForMutation` opcode and VM execution guard.
  - **Prevention:** For mutating operations, enforce mutability at mutation-entry points, not only at assignment sinks.

## Things I Learned
- Binding mutability must be tracked both by variable name and local-slot metadata to keep compiler/VM and interpreter behavior aligned.
- Pattern binding paths are easy to miss; mutability must be propagated through destructuring/match binding code paths, not only direct identifier declarations.
- `let`/`const` immutability in this codebase now applies to in-place container mutation through the binding, not just direct scalar reassignment.

## Debug Notes (Only if applicable)
- **Failing test / error:** `test_generator_with_state` failed once mutability checks became active because the fixture used immutable `let` with reassignment.
- **Repro steps:** Run `cargo test` after enabling assignment checks.
- **Breakpoints / logs used:** Full `cargo test` output plus targeted `interpreter_tests` runs.
- **Final diagnosis:** Generator fixture needed `mut` for incrementing state variable.

## Follow-ups / TODO (For Future Agents)
- [ ] Revisit safe dead-store optimization strategy that preserves semantic check side effects.
- [ ] Consider centralizing VM local mutability metadata lifecycle helpers to reduce duplicated slot initialization logic.

## Links / References
- Files touched:
  - `src/interpreter/environment.rs`
  - `src/interpreter/mod.rs`
  - `src/bytecode.rs`
  - `src/compiler.rs`
  - `src/vm.rs`
  - `tests/vm_interpreter_parity_surfaces.rs`
  - `tests/interpreter_tests.rs`
  - `docs/LANGUAGE_SPEC.md`
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `README.md`
  - `ROADMAP.md`
