# V1VM-PAR-004 Method Dispatch Parity Burn-Down (2026-05-24)

## Scope
- Continue reducing non-intentional VM/interpreter parity drift while `V1VM-PAR-004` remains blocked.
- Focused on method/self dispatch parity and call-shape stability in VM mode.

## Code changes
- `src/compiler.rs`
  - Track method-call lowering flow with `has_method_call_flow` and skip optimizer passes for chunks that use method-call sugar patterns.
  - Lower `Expr::Call` on `Expr::FieldAccess` via receiver-aware call shape.
- `src/vm.rs`
  - Preserve `raw_args` alongside prepared call args in bytecode-call setup.
  - Add additive legacy-method compatibility binding for receiver fields when methods are compiled without explicit `self`.
  - Keep call setup deterministic across direct and slow-path invocation paths.
- `tests/vm_interpreter_parity_surfaces.rs`
  - Added `vm_and_interpreter_match_legacy_method_without_self_field_lookup` regression.

## Commands run
- `cargo test --test vm_interpreter_parity_surfaces`
  - Result: `90 passed; 0 failed`.
- `cargo run -- test --runtime vm`
  - Result: `Passed 115/150` (`vm_primary=115`).
- `cargo run -- test --runtime dual`
  - Result: `Passed 128/150` (`vm_primary=115`, `interpreter_fallback=13`).
- `bash scripts/generate_vm_runtime_mismatch_inventory.sh`
  - Result: regenerated `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` and `.csv`.
- `cargo test --test vm_runtime_mismatch_inventory_contract`
  - Result: `2 passed; 0 failed`.

## Inventory deltas
- `runtime-parity-bug`: `35 -> 24`
- `harness-debt`: `0 -> 0`
- `stale-snapshot-expectation`: `0 -> 0`

## Status
- `V1VM-PAR-004` remains blocked because non-intentional divergence is not yet zero.
- This loop materially reduced P0 parity drift and improved VM-first execution coverage.

## Follow-ups
- Continue burn-down on remaining `runtime-parity-bug` fixtures, prioritizing method/chaining/operator clusters still requiring interpreter fallback in dual mode.
