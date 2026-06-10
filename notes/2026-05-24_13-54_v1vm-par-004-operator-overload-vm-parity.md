# V1VM-PAR-004 Operator Overload VM Parity Burn-Down (2026-05-24)

## Scope
- Continue `V1VM-PAR-004` runtime-parity reduction by fixing VM handling of struct binary operator overloading.
- Target fixtures: `test_op_add`, `test_op_add_debug`, `test_vec_add`, and related drift surfaced in dual-mode fallback.

## Root causes
1. VM `binary_op` never consulted struct operator methods (`op_add`, etc.) and errored with `Invalid binary operation: struct + struct`.
2. VM JIT slow-path nested function execution (`call_function_from_jit`) did not prepare method-call arguments for method semantics, and lacked opcode support needed by operator methods (`FieldGet`, `CallNative`, `MakeStruct`).

## Changes made
- `src/vm.rs`
  - Added VM operator dispatch path in `binary_op` via `try_call_vm_binary_operator_method` using operator-method mapping (`+ -> op_add`, etc.).
  - Ensured JIT slow-path bytecode calls prepare method arguments with `prepare_bytecode_call_args` before frame setup.
  - Added nested-call opcode support for `CallNative`, `FieldGet`, and `MakeStruct` in JIT slow-path execution.
- `tests/vm_interpreter_parity_surfaces.rs`
  - Added regression: `vm_and_interpreter_match_struct_op_add_overload_surface`.
- Snapshot updates
  - `tests/test_operator_add_working.out`

## Validation commands
- `cargo test --test vm_interpreter_parity_surfaces`
  - Result: `92 passed; 0 failed`.
- `cargo run -- test --runtime vm`
  - Result: `Passed 118/150` (`vm_primary=118`).
- `cargo run -- test --runtime dual`
  - Result: `Passed 128/150` (`vm_primary=118`, `interpreter_fallback=10`).
- `bash scripts/generate_vm_runtime_mismatch_inventory.sh`
  - Result: regenerated `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` and `.csv`.
- `cargo test --test vm_runtime_mismatch_inventory_contract`
  - Result: `2 passed; 0 failed`.

## Inventory deltas
- `runtime-parity-bug`: `22 -> 18`
- `stale-snapshot-expectation`: `1 -> 0`
- `harness-debt`: `0 -> 0`
- `both match snapshot`: `141 -> 145`
- VM coverage metric: `142/163 (87.1%) -> 145/163 (89.0%)`

## Status
- `V1VM-PAR-004` remains blocked (non-intentional parity drift still present).
- This loop removed the struct `+` operator-overload VM parity defect class and reduced dual fallback pressure (`interpreter_fallback: 13 -> 10`).

## Follow-ups
- Prioritize remaining `runtime-parity-bug` fixtures with largest VM-only drift clusters (`method_chaining` operator family, function/loop drop path, env/stdlib parity).
