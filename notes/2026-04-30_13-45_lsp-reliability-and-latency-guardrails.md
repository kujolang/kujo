# v0.13.0 LSP Reliability And Latency Guardrails Evidence

Date: 2026-04-30
Track: v0.13.0 Cross-IDE Foundation
Checklist item: Performance, Reliability, And Crash Safety

## Implemented

- Added cancellation handling:
  - notification: `$/cancelRequest`
  - response error shape: code `-32800` (`Request cancelled`)
- Added per-request timeout handling:
  - CLI configuration: `kujo lsp --request-timeout-ms <ms>`
  - response error shape: code `-32001` (`Request timed out after <ms>ms`)
- Added malformed-input resilience behavior/tests:
  - non-object JSON messages are ignored safely without panic
- Added latency guardrail baseline test for representative samples:
  - completion
  - diagnostics
  - hover

## Verification

Commands:

- `cargo test lsp_server::tests::timeout_returns_timeout_error_shape`
- `cargo test --test lsp_latency_guardrails`
- `cargo test`

Results:

- PASS
- Guardrail and reliability tests pass in both focused and full-suite runs.
