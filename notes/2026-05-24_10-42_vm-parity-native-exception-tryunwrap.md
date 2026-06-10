# VM parity burn-down note (2026-05-24)

## Scope
- Continue runtime parity burn-down from `runtime-parity-bug: 9` toward target `4`.
- Focused surfaces:
  - native error propagation through VM call paths
  - `TryUnwrap` early-return control-flow semantics

## Implementation summary
- `src/vm.rs`
  - Added `throw_runtime_value` helper and reused it from `OpCode::Throw`.
  - Updated VM native-call paths to throw into VM exception handlers instead of hard-failing:
    - `OpCode::CallNative`
    - `OpCode::Call` when callee is `Value::NativeFunction`
  - Fixed `OpCode::TryUnwrap` to early-return from the current function frame for `Err`/`None` paths (instead of exiting top-level script execution).

## Evidence commands
- `bash scripts/generate_vm_runtime_mismatch_inventory.sh`
- `cargo test --test vm_interpreter_parity_surfaces`
- `cargo test --test vm_runtime_mismatch_inventory_contract`
- `cargo test --test vm_runtime_mismatch_baseline_contract`
- `cargo run -- test --runtime vm`
- `cargo run -- test --runtime dual`

## Results
- Mismatch inventory after regeneration:
  - `P0 runtime-parity-bug`: `9 -> 4`
  - Remaining fixtures:
    - `tests/stdlib_os_path_test.kujo`
    - `tests/stdlib_test.kujo`
    - `tests/test_connection_pooling.kujo`
    - `tests/test_generators.kujo`
- Runtime sweeps:
  - VM: `Passed 128/150`
  - Dual: `Passed 129/150` (`vm_primary=128`, `interpreter_fallback=1`)
- Parity contracts:
  - `vm_interpreter_parity_surfaces`: `95 passed`
  - `vm_runtime_mismatch_inventory_contract`: `2 passed`
  - `vm_runtime_mismatch_baseline_contract`: `4 passed`

## Residual risks / follow-ups
- `test_generators` remains an active VM runtime-path parity bug (generator call/resume semantics).
- stdlib/path/pooling fixtures still show runtime-output drift and need dedicated fixture-family closure loops.
