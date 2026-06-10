# V1VM-PAR-004 Chain-Name Method Dispatch Parity (2026-05-24)

## Scope
- Continue `V1VM-PAR-004` parity burn-down on method/self dispatch collisions.
- Focused fix: struct methods named `chain` were incorrectly routed through native-method call lowering in VM mode.

## Root cause
- Compiler `Expr::MethodCall` lowering treated `chain` as a built-in iterator/native method name and emitted `OpCode::CallNative("chain", ...)`.
- For struct receivers, VM/native dispatch produced runtime failures (`Unknown native function: chain`) instead of struct method execution.

## Changes made
- `src/compiler.rs`
  - Removed `"chain"` from native-iterator method-call lowering special-case list so struct `chain` methods use regular field/method resolution.
- `tests/vm_interpreter_parity_surfaces.rs`
  - Added regression: `vm_and_interpreter_match_struct_method_named_chain_collision_surface`.
- `tests/test_self_param.out`
  - Refreshed snapshot after VM parity fix (fixture now executes successfully in VM mode).
- `tests/test_chain_debug.out`
  - Refreshed snapshot where VM/interpreter output now agree (`In chain`).

## Validation commands
- `cargo test --test vm_interpreter_parity_surfaces`
  - Result: `91 passed; 0 failed`.
- `cargo run -- test --runtime vm`
  - Result: `Passed 116/150` (`vm_primary=116`).
- `cargo run -- test --runtime dual`
  - Result: `Passed 129/150` (`vm_primary=116`, `interpreter_fallback=13`).
- `bash scripts/generate_vm_runtime_mismatch_inventory.sh`
  - Result: regenerated `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` and `.csv`.
- `cargo test --test vm_runtime_mismatch_inventory_contract`
  - Result: `2 passed; 0 failed`.

## Inventory deltas
- `runtime-parity-bug`: `24 -> 22`
- `stale-snapshot-expectation`: `1 -> 0`
- `harness-debt`: `0 -> 0`
- `both match snapshot`: `140 -> 141`

## Status
- `V1VM-PAR-004` remains blocked (non-intentional parity bucket still non-zero).
- This loop removed a concrete method-name collision class and improved VM-first pass rate/coverage.

## Follow-ups
- Prioritize remaining `runtime-parity-bug` fixtures with highest fallback pressure (`method_chaining`, operator-overload/collection drift, and env/stdlib snapshot drifts).
