# V1VM-PAR-004 Throw Stack Parity Burn-down (2026-05-24)

## Scope
- Continue `V1VM-PAR-004` by reducing non-intentional VM/interpreter mismatch volume.
- Focused fix: thrown error stack parity in VM catch paths.

## Changes
- Updated `src/vm.rs` (`OpCode::Throw`) to normalize thrown values into `ErrorObject` with call-stack capture parity.
  - `Str`/`Error`/`Struct` throws now map to `ErrorObject` with deterministic stack/message semantics.
  - Existing `ErrorObject` throws preserve stack and backfill call stack when empty.
- Added parity regression in `tests/vm_interpreter_parity_surfaces.rs`:
  - `vm_and_interpreter_match_throw_call_stack_surface`
- Updated fixture snapshot to match now-aligned behavior:
  - `tests/test_try_except.out`

## Commands And Results
- `cargo test --test vm_interpreter_parity_surfaces vm_and_interpreter_match_throw_call_stack_surface` -> `ok (1 passed)`
- `cargo test --test vm_interpreter_parity_surfaces` -> `ok (88 passed)`
- `bash scripts/generate_vm_runtime_mismatch_inventory.sh` -> regenerated `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.{md,csv}`
- `cargo run -- test --runtime vm` -> `Passed 108/150` (`+1` from prior `107/150`)
- `cargo run -- test --runtime dual` -> `Passed 122/150` (`vm_primary=108`, `interpreter_fallback=14`; `+1` from prior `121/150`)

## Inventory Delta
- `P0 runtime-parity-bug`: `40 -> 36`
- `P1 stale-snapshot-expectation`: `1 -> 0` after refreshing `tests/test_try_except.out`
- VM coverage gate: `133/163 (81.6%) -> 134/163 (82.2%)`

## Residual Blocker
- `V1VM-PAR-004` remains blocked because `P0 runtime-parity-bug` is still non-zero (`36`).
- Next likely high-leverage candidates remain method-call and native-surface mismatches listed in generated inventory.
