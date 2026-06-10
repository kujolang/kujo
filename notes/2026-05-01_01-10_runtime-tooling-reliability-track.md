# v0.14.0 Runtime And Tooling Reliability Evidence

Date: 2026-05-01
Context: local development machine (macOS)
Scope: ROADMAP section "5. Runtime And Tooling Reliability Track"

## Implemented

- Added LSP reliability-track integration test suite:
  - `tests/lsp_reliability_track.rs`
- Added reliability document with lifecycle and latency baseline contracts:
  - `docs/LSP_RELIABILITY.md`
- Added CI matrix coverage for reliability suite:
  - `.github/workflows/lsp-contract-matrix.yml` now runs `cargo test --test lsp_reliability_track`
- Added test-observability accessor for bounded-state assertions:
  - `src/lsp_server.rs` (`open_document_count()`)

## Verification Commands

1. `cargo test --test lsp_reliability_track`
- Result: PASS (`3 passed; 0 failed`)

2. `cargo test --test lsp_conformance_harness`
- Result: PASS (`1 passed; 0 failed`)

3. `cargo test --test lsp_external_clients_smoke`
- Result: PASS (`2 passed; 0 failed`)

4. `cargo test --test lsp_latency_guardrails`
- Result: PASS (`1 passed; 0 failed`)

## Acceptance Mapping

- Malformed message sequence resilience and lifecycle churn are now explicitly tested.
- Repeated request-loop behavior asserts bounded server document-state retention.
- Startup/first-response latency baselines are tracked with conservative guardrails.
- Reliability suite is wired into CI matrix and validated locally.

## Remaining v0.14.0 Checklist Work

- v1.0.0 scope definition gate
