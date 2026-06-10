# Kujo Field Notes — V1-RUN-004 Truthiness Centralization

**Date:** 2026-05-07
**Session:** 13:03 local
**Branch/Commit:** main / ae4474b
**Scope:** Implemented roadmap item `V1-RUN-004` by centralizing truthiness semantics and aligning interpreter/VM logical short-circuit behavior with parity coverage.

---

## What I Changed
- Added shared truthiness helper `Value::is_truthy()` in `src/interpreter/value.rs`.
- Routed interpreter control-flow truthiness (`if`, `while`, `loop`) through the shared helper in `src/interpreter/mod.rs`.
- Added interpreter `&&`/`||` short-circuit evaluation in `Expr::BinaryOp` handling with boolean result normalization.
- Updated compiler `&&`/`||` lowering in `src/compiler.rs` to emit short-circuit jump paths that skip unreachable RHS evaluation for VM execution.
- Routed VM truthiness checks to the shared helper (`src/vm.rs`).
- Routed native collection/assert predicate truthiness through shared helper in:
  - `src/interpreter/native_functions/collections.rs`
  - `src/interpreter/native_functions/type_ops.rs`
- Added regression tests:
  - `tests/vm_interpreter_parity_surfaces.rs` for truthiness-table parity and logical short-circuit parity/failure paths.
  - `tests/interpreter_tests.rs` for collection predicate truthiness via dict/array returns.
  - `src/interpreter/value.rs` unit test for representative truthy/falsey values.
- Updated docs:
  - `docs/LANGUAGE_SPEC.md` truthiness + logical operator semantics.
  - `README.md` language overview bullets.
  - `CHANGELOG.md` and `ROADMAP.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** First short-circuit compiler lowering shape caused VM `Stack underflow`.
  - **Symptom:** New parity truthiness test failed in VM with `Stack underflow`.
  - **Root cause:** Initial lowering pattern plus optimizer interaction produced an invalid stack-shape path.
  - **Fix:** Reworked lowering to branch-pop both paths and normalize RHS via bool constants + `And`/`Or`; added compiler flag `has_logical_short_circuit` to skip current optimizer passes for chunks using this pattern.
  - **Prevention:** Treat logical short-circuit lowering as stack-sensitive control flow; verify both branch stack depths and run parity tests with optimizer enabled.

- **Gotcha:** `OpCode::Not` cannot be used as generic truthiness coercion in VM.
  - **Symptom:** VM error `Invalid unary operation: ! Int(2)` when using `!!` lowering.
  - **Root cause:** VM unary `Not` intentionally errors for non-boolean operands after invalid-op hardening.
  - **Fix:** Avoid `!!` lowering for generic truthiness conversion; use `And`/`Or` with explicit bool constants instead.
  - **Prevention:** Re-check opcode semantic contracts after runtime-hardening items; do not assume coercion behavior from dynamic languages.

## Things I Learned
- Truthiness drift existed in three places at once: interpreter control-flow, native predicate helpers, and VM helper policy.
- `"false"` string is a compatibility-sensitive semantic: once truthiness is centralized, non-empty strings stay truthy consistently across runtimes.
- Compiler short-circuit lowering must be paired with a stable optimizer contract, not just runtime opcode logic.

## Debug Notes (Only if applicable)
- **Failing test / error:** `vm_and_interpreter_match_truthiness_semantics_across_conditionals` failed with `Invalid binary operation: int && int` first, then VM `Stack underflow`, then VM `Invalid unary operation: ! Int(2)`.
- **Repro steps:** `cargo test --test vm_interpreter_parity_surfaces vm_and_interpreter_match_truthiness_semantics_across_conditionals -- --nocapture`
- **Breakpoints / logs used:** Compiler optimization stderr stats + parity test error traces.
- **Final diagnosis:** Needed both shared truthiness helper routing and safe compiler short-circuit lowering that avoids unsupported unary coercion semantics.

## Follow-ups / TODO (For Future Agents)
- [ ] Revisit optimizer support for logical short-circuit lowering so `has_logical_short_circuit` guard can eventually be removed safely.
- [ ] Expand truthiness-table coverage across additional runtime value kinds if/when semantics for set/queue/stack/native handles are formalized beyond “truthy by default.”

## Links / References
- Files touched:
  - `src/interpreter/value.rs`
  - `src/interpreter/mod.rs`
  - `src/interpreter/native_functions/collections.rs`
  - `src/interpreter/native_functions/type_ops.rs`
  - `src/compiler.rs`
  - `src/vm.rs`
  - `tests/vm_interpreter_parity_surfaces.rs`
  - `tests/interpreter_tests.rs`
  - `docs/LANGUAGE_SPEC.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `docs/LANGUAGE_SPEC.md`
