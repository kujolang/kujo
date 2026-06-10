# VM Destructuring Parity Follow-Through (v1.0.0 P0)

Date: 2026-05-01

## Summary

Executed another cycle on the open v1.0.0 parity P0 item and closed the VM destructuring binding mismatch that was causing parity failures.

Completed in this cycle:

- Implemented VM-side array/dict destructuring bindings for parity-covered scenarios.
- Added top-level binding propagation for VM pattern-bound identifiers so destructured values are available in global script scope.
- Updated parity tests to assert destructuring alignment and keep spread semantics as an explicit remaining VM capability gap.
- Updated roadmap/matrix/changelog/notes to reflect narrowed remaining parity work.

## Code Changes

### VM pattern matching and binding

- File: `src/vm.rs`
- Changes:
  - `match_pattern(...)` now recursively binds identifiers inside array patterns.
  - Added dict-pattern binding support for both `Value::Dict` and `Value::FixedDict`.
  - Added rest capture support for array and dict patterns.
  - Added `bind_pattern_name(...)` helper to write bindings to frame locals and top-level globals when appropriate.

### Parity tests

- File: `tests/vm_interpreter_parity_surfaces.rs`
- Changes:
  - Updated spread/destructuring surface test to assert:
    - interpreter: destructuring + spread checks pass
    - VM: destructuring checks pass
    - VM spread behavior remains a tracked capability gap (`spread_ok == false`)

## Documentation Updates

- Updated `docs/VM_INTERPRETER_PARITY_MATRIX.md` status for spread/destructuring to reflect partial alignment (destructuring aligned, spread gap remains).
- Updated `ROADMAP.md` parity checklist to mark VM destructuring binding gap complete and narrow remaining gaps to spread literals + tag-style match bindings.
- Updated `CHANGELOG.md` and `notes/README.md` with this cycle's outcomes.

## Verification

Commands run:

1. `cargo test --test vm_interpreter_parity_surfaces`
- Result: PASS
- Passed tests: 4
- Failed tests: 0

## Remaining Work For This P0 Item

Still open (tracked in roadmap and matrix):

- close VM spread-literal capability mismatch against interpreter semantics
- close tag-style match-binding capability gap

This P0 item remains open until those two capability gaps are closed.
