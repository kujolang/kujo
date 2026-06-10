# Session Note — 2026-05-23 — V1H-FEAT-001 closure immutability parity

## Goal
Reduce P0 VM/runtime parity mismatches by closing a concrete interpreter-vs-VM behavior gap without broad refactors.

## What changed
- Enforced immutable captured-binding reassignment checks in VM `StoreVar` paths:
  - Main VM execution loop `OpCode::StoreVar`
  - Generator execution loop `OpCode::StoreVar`
- Added captured binding mutability metadata propagation:
  - `Value::BytecodeFunction` now carries `captured_binding_kinds`
  - `OpCode::MakeClosure` captures binding kind metadata with captured refs
  - `call_bytecode_function` forwards captured mutability metadata into call frames
  - `CallFrame` / `CallFrameData` / `GeneratorState` updated for restore/save parity
- Added parity regression test:
  - `vm_and_interpreter_reject_reassignment_of_captured_immutable_let_binding`
- Updated closure fixture snapshots to interpreter-consistent stdout when immutable capture reassignment errors:
  - `tests/vm_closure_simple.out`
  - `tests/vm_closure_multiple.out`
  - `tests/vm_closure_order.out`
  - `tests/vm_closure_detailed.out`

## Evidence
- Inventory regenerated:
  - `bash scripts/generate_vm_runtime_mismatch_inventory.sh`
  - `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` now reports:
    - `P0 runtime-parity-bug`: `21` (was `25`)
    - `P2 harness-debt`: `16` (unchanged)
- Closure fixtures now classify as parity matches in generated inventory:
  - `vm_closure_simple`, `vm_closure_multiple`, `vm_closure_order`, `vm_closure_detailed` => `both_match_snapshot`

## Validation commands
- `cargo test vm_and_interpreter_reject_reassignment_of_captured_immutable_let_binding --test vm_interpreter_parity_surfaces`
- `cargo test --test vm_interpreter_parity_surfaces`
- `cargo run -- test --runtime vm`
- `cargo run -- test --runtime dual`
- `bash scripts/generate_vm_runtime_mismatch_inventory.sh`

## Results
- Targeted and parity tests passed for touched behavior.
- Runtime sweep commands completed; existing unrelated baseline fixture/parser/runtime failures remain outside this loop scope.
- P0 parity bucket reduced monotonically (`25 -> 21`).
