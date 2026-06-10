# V1VM-PAR-004 Parity Closure Notes

Date: 2026-05-24
Owner: runtime/vm

## Summary

Closed the final unresolved VM runtime parity drift by aligning assignment semantics for unresolved `:=` inside function scopes with interpreter behavior and validating full mismatch artifacts.

## Root cause addressed

- VM compiler/runtime previously created local frame bindings for unresolved identifier assignment inside functions (`:=`) even when an outer binding existed.
- Interpreter semantics use lexical assignment (`assign_checked`) to update an existing outer binding first, only defining a new binding when no prior binding exists.
- This mismatch caused `tests/stdlib_os_path_test.kujo` to diverge (`test_count`/`passed` remained `0` in VM).

## Code changes

- `src/compiler.rs`
  - Updated `compile_assignment` unresolved in-function identifier path to emit `StoreVar` (dynamic lexical assignment behavior) instead of auto-allocating a new local slot.
- `src/vm.rs`
  - Updated `StoreVar` handling (main VM loop + slow path + generator path) to:
    - update captured bindings when captured,
    - update frame locals when present,
    - otherwise update existing global binding if present,
    - else define a new mutable local binding in frame.

## Commands run

- `bash scripts/generate_vm_runtime_mismatch_inventory.sh`
- `cargo test --test vm_runtime_mismatch_inventory_contract`
- `cargo test --test vm_runtime_mismatch_baseline_contract`
- `cargo test --test vm_interpreter_parity_surfaces`
- `cargo run -- test --runtime vm`
- `cargo run -- test --runtime dual`

## Results

- `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` summary:
  - `P0 runtime-parity-bug: 0`
  - `P2 harness-debt: 0`
  - remaining mismatches: `P1 stale-snapshot-expectation: 8`
- `cargo test --test vm_runtime_mismatch_inventory_contract`: passed (2/2)
- `cargo test --test vm_runtime_mismatch_baseline_contract`: passed (4/4)
- `cargo test --test vm_interpreter_parity_surfaces`: passed (95/95)
- `cargo run -- test --runtime vm`: passed `129/150`
- `cargo run -- test --runtime dual`: passed `129/150` (`vm_primary=129`, `interpreter_fallback=0`)

## Follow-ups

- Snapshot-only drift remains (`stale-snapshot-expectation: 8`) and can be addressed in docs-owner snapshot refresh loops.
- No unexplained runtime-path parity drift remains in current mismatch artifact.
