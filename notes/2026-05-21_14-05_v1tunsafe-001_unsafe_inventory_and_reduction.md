# V1TUNSAFE-001 - Unsafe Inventory Refresh And Targeted Reduction

Date: 2026-05-21
Checklist item: `V1TUNSAFE-001`

## Inventory Refresh

- Added machine-verifiable inventory generator:
  - `scripts/generate_unsafe_inventory.sh`
- Added contract coverage:
  - `tests/unsafe_inventory_contract.rs`
- Generated refreshed inventory artifacts:
  - `docs/generated/UNSAFE_INVENTORY.md`
  - `docs/generated/UNSAFE_INVENTORY.csv`

Inventory summary from refreshed artifact:
- Total matches: `53`
- Executable matches: `49`
- Non-executable matches: `4`
- Unknown classifications: `0`

## Executable Unsafe Reduction (Scoped)

- Added safe wrapper `set_return_int(&mut VMContext, i64)` in `src/jit.rs` to centralize one audited unsafe call boundary.
- Updated `jit::tests::test_return_value_optimization` to use the safe wrapper for valid-context paths.
- Result: removed multiple ad-hoc executable `unsafe` callsites from test flow while preserving explicit null-pointer negative-path coverage.

## Commands Run

1. `bash scripts/generate_unsafe_inventory.sh --output-md docs/generated/UNSAFE_INVENTORY.md --output-csv docs/generated/UNSAFE_INVENTORY.csv --strict` -> PASS
2. `cargo test --test unsafe_inventory_contract` -> PASS
3. `cargo test test_return_value_optimization` -> PASS
4. `cargo test --test vm_interpreter_parity_surfaces` -> PASS

## Outcome

`V1TUNSAFE-001` is complete for this loop scope: unsafe inventory generation is reproducible and classification-checked, and executable unsafe usage has been reduced in a focused JIT path with regression coverage.
