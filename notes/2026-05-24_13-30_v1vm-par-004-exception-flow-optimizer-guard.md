# V1VM-PAR-004 Exception-Flow Optimizer Guard (2026-05-24)

## Scope
- Continue parity burn-down for `V1VM-PAR-004` with a focused VM/runtime correctness fix.
- Target concrete VM runtime failure on `tests/test_exceptions_comprehensive.kujo` (`level1 expects 0 arguments, got 1`).

## Changes
- `src/compiler.rs`
  - Added `has_exception_flow` compiler flag.
  - Marked exception-flow surfaces (`Stmt::TryExcept`, `Expr::Try`, `throw(...)`) as exception-flow.
  - Updated optimization gate to skip optimizer passes when exception-flow lowering is present.
- `src/vm.rs`
  - Added exceptional unwind metadata cleanup in `OpCode::Throw` frame unwind path:
    - pop `function_call_stack`
    - decrement `recursion_depth`
- `tests/vm_interpreter_parity_surfaces.rs`
  - Added `vm_and_interpreter_execute_exception_fixture_without_runtime_arity_drift` regression test that executes `tests/test_exceptions_comprehensive.kujo` in both runtimes and asserts VM no longer fails.

## Commands And Results
- `cargo test --test vm_interpreter_parity_surfaces vm_and_interpreter_execute_exception_fixture_without_runtime_arity_drift` -> `ok (1 passed)`
- `cargo test --test vm_interpreter_parity_surfaces` -> `ok (89 passed)`
- `cargo run --quiet -- run tests/test_exceptions_comprehensive.kujo` -> VM now exits `0` and completes output
- `bash scripts/generate_vm_runtime_mismatch_inventory.sh` -> regenerated inventories
- `cargo run -- test --runtime vm` -> `Passed 109/150` (`+1`)
- `cargo run -- test --runtime dual` -> `Passed 122/150` (`vm_primary=109`, `interpreter_fallback=13`; fallback `-1`)

## Inventory Delta
- `tests/test_exceptions_comprehensive.kujo`: `runtime-parity-bug` -> `both_match_snapshot`
- `P0 runtime-parity-bug`: `36 -> 35`
- VM coverage: `134/163 (82.2%) -> 135/163 (82.8%)`

## Residual Blocker
- `V1VM-PAR-004` remains blocked: `P0 runtime-parity-bug` is still non-zero (`35`).
