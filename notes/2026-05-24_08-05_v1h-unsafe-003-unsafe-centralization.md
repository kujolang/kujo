# V1H-UNSAFE-003 Unsafe Centralization

Date: 2026-05-24
Item: `V1H-UNSAFE-003`

## Scope

Reduce executable unsafe callsites in `src/jit.rs` via additive safe wrappers without changing runtime behavior.

## Changes

- Added centralized wrappers in `src/jit.rs`:
  - `with_vm_context_mut`
  - `with_vm_stack_mut`
  - `compiled_fn_from_code_ptr`
  - `compiled_fn_with_arg_from_code_ptr`
- Migrated `jit_make_dict` and `jit_make_dict_with_keys` away from repeated ad hoc pointer-deref unsafe blocks to wrapper usage.
- Migrated four repeated `transmute(code_ptr)` callsites to centralized typed pointer-cast wrappers.
- Added/updated unsafe inventory contract budget guard in `tests/unsafe_inventory_contract.rs`.

## Unsafe Reduction Evidence

- Prior checker baseline (from `V1H-UNSAFE-002` closure):
  - `Checked 47 executable unsafe boundaries in src/jit.rs; missing contracts: 0`
- Post-change checker:
  - `Checked 43 executable unsafe boundaries in src/jit.rs; missing contracts: 0`
- Inventory summary delta:
  - Executable matches `59 -> 55`
  - Total matches `67 -> 63`

## Commands Run

- `bash scripts/generate_unsafe_inventory.sh` -> pass
- `bash scripts/check_jit_safety_contracts.sh` -> pass (`missing contracts: 0`)
- `cargo test --test unsafe_inventory_contract` -> pass (`3 passed`)
- `cargo test --test jit_safety_contract_checker` -> pass (`8 passed`)
- `cargo test --test jit_execution_contract` -> pass (`3 passed`)
- `cargo test --test vm_interpreter_parity_surfaces` -> pass (`87 passed`)

## Residual Follow-up

- Further reduction opportunities remain in JIT runtime helper boundaries (`unsafe extern "C" fn` declarations), but this pass completed the scoped centralization with measurable unsafe density reduction and preserved parity.
