# Kujo Field Notes — V1-RUN-003 invalid-operation runtime errors

**Date:** 2026-05-06
**Session:** 21:06 local
**Branch/Commit:** main / 2a93f60
**Scope:** Replaced silent invalid-operation fallback behavior with structured runtime errors across interpreter and VM, then locked behavior with parity regression tests and roadmap/docs updates.

---

## What I Changed
- Added centralized interpreter runtime helpers in `src/interpreter/mod.rs`:
  - `index_value`
  - `assign_index`
  - `assign_member`
  - `binary_op_value`
  - `unary_op_value`
- Replaced assignment-path `eprintln!` continuation behavior with returned runtime errors in interpreter assignment execution.
- Replaced interpreter expression indexing fallbacks (`Int(0)` / empty string) with runtime errors for out-of-bounds and invalid indexing.
- Replaced VM binary-op `Int(0)` fallback behavior with structured runtime errors in `src/vm.rs`.
- Updated VM unary behavior so `Not` and `Negate` share strict unary contracts instead of using truthiness fallback for all values.
- Gated top-level script JIT away from `OpCode::Negate` and `OpCode::Not` until JIT parity for strict unary error semantics is explicit.
- Added/expanded parity regression coverage in `tests/vm_interpreter_parity_surfaces.rs` for:
  - out-of-bounds array indexing
  - out-of-bounds string indexing
  - indexing non-indexable values
  - invalid index-assignment target
  - unsupported unary operation
  - unsupported binary operation
  - valid index assignment success path

## Gotchas (Read This Next Time)
- **Gotcha:** Top-level script JIT can bypass strict runtime error semantics for unary operations.
  - **Symptom:** `return -true` did not fail in VM parity checks even after VM bytecode unary-op checks were hardened.
  - **Root cause:** Script JIT admission still allowed unary opcodes (`Negate` / `Not`) and the JIT path did not preserve the strict error contract used by the bytecode VM path.
  - **Fix:** Excluded `OpCode::Negate` and `OpCode::Not` from script-JIT-safe opcode admission.
  - **Prevention:** When tightening runtime semantic checks, audit script JIT admission and nested bytecode-call opcode handlers in addition to the primary VM execute loop.

## Things I Learned
- Assignment-path error behavior needs centralized helpers (`assign_index`, `assign_member`) to avoid regressions from scattered closure-local logic.
- Interpreter and VM parity work is easiest to keep stable when both routes share similarly named helpers for indexing/binary/unary semantics.
- Focused parity regression tests catch mode-specific semantic drift quickly, especially with JIT admission side effects.

## Debug Notes (Only if applicable)
- **Failing test / error:** Parity regression `vm_and_interpreter_error_on_unsupported_unary_operation` expected VM error but got successful execution.
- **Repro steps:** `cargo test --test vm_interpreter_parity_surfaces vm_and_interpreter_error_on_unsupported_`
- **Breakpoints / logs used:** Checked VM opcode handlers for `OpCode::Negate` and `OpCode::Not`, then script JIT admission rules in `VM::execute`.
- **Final diagnosis:** VM bytecode path was strict, but script JIT path bypassed strict unary runtime-error behavior.

## Follow-ups / TODO (For Future Agents)
- [ ] Add explicit parity tests that exercise strict unary-op behavior on JIT-admissible scripts and assert script-JIT admission policy as part of runtime semantics hardening.
- [ ] Revisit script JIT unary opcode support once parity-checked error propagation is implemented in JIT execution.

## Links / References
- Files touched:
  - `src/interpreter/mod.rs`
  - `src/vm.rs`
  - `tests/vm_interpreter_parity_surfaces.rs`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `README.md`
  - `docs/LANGUAGE_SPEC.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `ROADMAP.md`
