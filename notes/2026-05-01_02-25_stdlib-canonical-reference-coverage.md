# Standard Library Canonical Reference Coverage (P1)

Date: 2026-05-01

## Summary

Completed the second P1 backlog item by publishing canonical builtin reference coverage and locking runtime/doc alignment with tests.

Completed in this cycle:

- Added a canonical v1 standard library reference for major builtin categories.
- Added per-function entries with stability tiers and examples.
- Added contract tests that ensure documented functions stay aligned with runtime builtin inventory.

## Code and Docs Changes

### Canonical reference doc

- File: `docs/STANDARD_LIBRARY_REFERENCE.md`
- Includes:
  - major builtin categories (core IO, strings, arrays, dicts, math/time/random, filesystem, environment/process/concurrency, network/auth, database/compression/crypto/image)
  - per-function tier labels (`stable`/`preview`/`experimental`)
  - per-function usage examples
  - explicit dispatch/coverage maintenance guidance

### Contract tests

- File: `tests/stdlib_reference_contract.rs`
- Added test: `stdlib_reference_documents_runtime_builtins`
- Guarantees:
  - broad documented function coverage floor
  - every documented function must exist in `Interpreter::get_builtin_names()`
  - key cross-category builtin names must remain documented

### Release tracking

- Updated `ROADMAP.md` to mark the standard-library documentation P1 item complete.
- Updated `CHANGELOG.md` and `notes/README.md` with this cycle outcomes.

## Verification

Commands run:

1. `cargo test --test stdlib_reference_contract`
- Result: PASS

## Follow-Through Context

Remaining open backlog after this cycle:

- P1: end-to-end package + module workflow integration tests
- P2: formal deprecation policy
- P2: security posture pass for high-risk native APIs
