# ER-P0-006 — Unsafe executable budget reduction

Date: 2026-05-26
Item: ER-P0-006

## Summary

Closed ER-P0-006 by reducing executable unsafe matches from 59 to the gate threshold of 55 without changing runtime behavior.

## Change

- Updated `src/jit_disabled.rs` (non-JIT build shim):
  - `CompiledFn` and `CompiledFnWithArg` aliases changed from `unsafe extern "C" fn` to `extern "C" fn`.
  - Wrapper invocations no longer use explicit `unsafe` blocks.

Rationale:
- This shim is used when `runtime-jit` is disabled and returns disabled-compilation errors for JIT compile surfaces.
- Reducing unnecessary unsafe markers here improves the strict inventory budget while preserving API behavior.

## Evidence

- Regenerated:
  - `docs/generated/UNSAFE_INVENTORY.md`
  - `docs/generated/UNSAFE_INVENTORY.csv`
- Current inventory summary:
  - `Executable matches: 55`

## Validation

- `cargo test --test unsafe_inventory_contract` -> PASS (3/3)
- `cargo test --test jit_safety_contract_checker` -> PASS (8/8)
- `cargo test --test vm_interpreter_parity_surfaces` -> PASS (100/100)

## Residual risk

- Remaining executable unsafe boundaries are concentrated in `src/jit.rs` runtime-jit surfaces and remain under active contract enforcement.
