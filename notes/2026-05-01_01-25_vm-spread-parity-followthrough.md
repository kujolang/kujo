# VM Spread Parity Follow-Through (v1.0.0 P0)

Date: 2026-05-01

## Summary

Executed another cycle on the open v1.0.0 parity P0 item and closed the VM spread-literal mismatch for parity-covered array/dict scenarios.

Completed in this cycle:

- Added marker-based bytecode/VM collection opcodes for spread-aware array and dict construction.
- Updated compiler emission for spread-containing literals to use marker-based collection paths.
- Updated parity tests and matrix to assert spread/destructuring alignment across interpreter and VM.
- Narrowed the remaining parity capability gap to tag-style match bindings.

## Code Changes

### Bytecode + compiler + VM spread literal construction

- Files:
  - `src/bytecode.rs`
  - `src/compiler.rs`
  - `src/vm.rs`
- Changes:
  - Added `OpCode::MakeArrayFromMarker` and `OpCode::MakeDictFromMarker`.
  - Compiler now emits marker-based construction for spread-containing literals:
    - arrays with spread emit `PushArrayMarker` + `MakeArrayFromMarker`
    - dicts with spread emit marker sentinel pair + `MakeDictFromMarker`
  - VM now collects spread-expanded values until marker sentinel(s), preserving order and key/value pairing semantics.

### Parity tests

- File: `tests/vm_interpreter_parity_surfaces.rs`
- Changes:
  - Updated spread/destructuring parity test to require `spread_ok == true` on both interpreter and VM paths.

## Documentation Updates

- Updated `docs/VM_INTERPRETER_PARITY_MATRIX.md` to mark spread/destructuring surface as aligned.
- Updated `ROADMAP.md` parity checklist to mark VM spread-literal capability gap complete.
- Updated `CHANGELOG.md` and `notes/README.md` with this cycle's outcomes.

## Verification

Commands run:

1. `cargo test --test vm_interpreter_parity_surfaces`
- Result: PASS
- Passed tests: 4
- Failed tests: 0

2. `cargo build`
- Result: PASS

## Remaining Work For This P0 Item

Still open (tracked in roadmap and matrix):

- close tag-style match-binding capability gap

This P0 parity item remains open until match-binding capability is closed.
