# v0.13.0 Protocol Fixture Matrix Evidence

Date: 2026-04-30

## Added Fixture Coverage

- `tests/lsp_fixtures/all_required_methods_success_error.json`
  - includes success and error-case coverage for required LSP parity methods:
    - completion
    - hover
    - definition
    - references
    - rename
    - codeAction
    - formatting
    - rangeFormatting
    - documentSymbol
    - workspace/symbol

## Harness Update

- Updated `tests/lsp_conformance_harness.rs` to support subset-field matching for protocol fixtures.
- Fixture assertions now remain strict on required contract fields while avoiding brittle full-payload lock-in.

## Verification

- `cargo test --test lsp_conformance_harness`
- PASS
