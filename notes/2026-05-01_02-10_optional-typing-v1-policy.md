# Optional Typing v1 Policy Publication (P1)

Date: 2026-05-01

## Summary

Completed the first P1 backlog item by turning optional typing from exploratory guidance into an explicit v1 policy with test-backed contract coverage.

Completed in this cycle:

- Converted optional typing documentation into a concrete v1 stance with clear supported/deferred boundaries.
- Added tests that lock parser metadata preservation for type annotations.
- Added tests that lock default dynamic runtime behavior (no implicit runtime type enforcement).

## Code and Docs Changes

### Policy document

- File: `docs/OPTIONAL_TYPING_DESIGN.md`
- Updated to explicit v1 policy language:
  - supported in v1: annotation syntax + metadata preservation
  - deferred post-v1: runtime type enforcement and typed-JIT guarantees
  - compatibility contract: annotations do not change runtime behavior by default

### Contract tests

- File: `tests/optional_typing_v1_contract.rs`
- Added tests:
  - `v1_annotations_are_preserved_as_parser_metadata`
  - `v1_annotations_do_not_enforce_runtime_types_by_default`

### Release tracking

- Updated `ROADMAP.md` to mark the optional-typing P1 item complete.
- Updated `CHANGELOG.md` and `notes/README.md` with this cycle results.

## Verification

Commands run:

1. `cargo test --test optional_typing_v1_contract`
- Result: PASS

2. `bash .github/scripts/check-release-state.sh`
- Result: PASS

3. `bash .github/scripts/check-contract-version-sync.sh`
- Result: PASS

## Follow-Through Context

Remaining open backlog after this cycle:

- P1: native standard library canonical docs coverage
- P1: end-to-end package + module workflow integration tests
- P2: formal deprecation policy
- P2: security posture pass for high-risk native APIs
