# VM/Interpreter Parity Matrix And Runtime Follow-Through (v1.0.0 P0)

Date: 2026-05-01

## Summary

Executed the next v1.0.0 parity-focused cycle for the open P0 runtime parity item.

Completed in this cycle:

- Added a dedicated parity matrix doc for tracked surfaces.
- Added parity/gap-tracking integration tests for the four required surfaces.
- Aligned interpreter `Expr::MethodCall` behavior with VM for user-defined struct methods (explicit `self` method path).
- Updated roadmap/spec/README to reflect current parity status and remaining capability gaps.

## Code Changes

### Runtime behavior alignment

- File: `src/interpreter/mod.rs`
- Change: `call_method(...)` now executes user-defined struct methods from `StructDef` method maps.
- Behavior:
  - supports explicit `self` first parameter binding
  - supports backward-compatible field-in-scope method form
  - preserves return/error handling path used by interpreter function execution

### Parity and gap tests

- File: `tests/vm_interpreter_parity_surfaces.rs`
- Added/maintained surface tests for:
  - struct method behavior parity (aligned)
  - spawn semantics parity (aligned)
  - spread/destructuring gap tracking (VM-path error contract currently documented)
  - tag-style match-binding capability gap tracking (current deterministic no-binding behavior in tested script shape)

## Documentation Updates

- Added `docs/VM_INTERPRETER_PARITY_MATRIX.md` with explicit surface-by-surface parity status.
- Updated `README.md` known-boundary bullets to reference parity matrix and current runtime status.
- Updated `docs/LANGUAGE_SPEC.md` to reference parity matrix for runtime-path parity/capability tracking.
- Updated `ROADMAP.md` parity-item substeps to reflect:
  - parity matrix/test baseline complete
  - docs alignment complete
  - remaining capability closures still open

## Verification

Commands run:

1. `cargo test --test vm_interpreter_parity_surfaces`
- Result: PASS
- Passed tests: 4
- Failed tests: 0

## Remaining Work For This P0 Item

Still open (tracked in roadmap and matrix):

- close VM-path destructuring capability gap surfaced in parity test scenario
- close tag-style match-binding capability gap tracked in parity matrix/test scenario

This item remains open in `ROADMAP.md` until those capability gaps are implemented and parity matrix status moves to fully aligned/complete.
