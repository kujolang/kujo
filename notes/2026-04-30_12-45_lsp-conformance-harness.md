# v0.13.0 LSP Conformance Harness Evidence

Date: 2026-04-30
Track: v0.13.0 Cross-IDE Foundation
Checklist item: Conformance Test Harness

## Implemented

- Added fixture-driven protocol harness:
  - `tests/lsp_conformance_harness.rs`
- Added deterministic fixture set:
  - `tests/lsp_fixtures/completion_ordering.json`
  - `tests/lsp_fixtures/edit_range_stability.json`
  - `tests/lsp_fixtures/error_payload_consistency.json`

Coverage focus:

- request/response conformance execution
- deterministic completion ordering assertions
- deterministic rename edit-range assertions
- error payload consistency assertions (`-32602` invalid params, `-32601` method not found)

## Verification

Commands:

- `cargo test --test lsp_conformance_harness`

Results:

- PASS (`1 passed; 0 failed`)

## Notes

- Harness is intentionally fixture-first so future protocol regressions fail with readable diff output.
