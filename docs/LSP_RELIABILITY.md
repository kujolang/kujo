# Kujo LSP Reliability Track

Status: stable v1.0.0 reliability baseline; originally introduced during the v0.14.0 reliability track

## Lifecycle Churn Contract

Kujo LSP lifecycle notifications are expected to tolerate churn in this order:

- `textDocument/didOpen`
- repeated `textDocument/didChange`
- `textDocument/didClose`

Resilience expectations:

- `didChange` with full document text received before `didOpen` creates document state and publishes diagnostics
- malformed request payloads return JSON-RPC error envelopes (for example `-32602` invalid params)
- repeated open/change/close loops do not leak document state across iterations

Validation source:

- `tests/lsp_reliability_track.rs` (`malformed_sequences_and_lifecycle_churn_are_resilient`)

## Bounded-Memory Request Loop Contract

Repeated request loops (completion/hover/diagnostics-adjacent requests) must keep server document state bounded.

Validation source:

- `tests/lsp_reliability_track.rs` (`repeated_completion_and_diagnostics_requests_keep_document_state_bounded`)

The reliability contract uses `LspServer::open_document_count()` to assert document-state bounds under high iteration count loops.

Each open document owns one immutable analyzed-program snapshot. Diagnostics,
hover and completion reuse it until `didChange` replaces it; `didClose` evicts
it. Imported-module parse results use a 128-entry content-hashed cache, so source
changes invalidate safely even when file size and timestamp granularity collide.
`LspServer::analysis_count()` and module cache statistics provide deterministic
regression assertions for reuse and invalidation.

The stdio transport limits each header block to 8 KiB, including its terminating blank line. It rejects incomplete headers and repeated `Content-Length` fields, and rejects payloads larger than 8 MiB before allocating the message buffer.

Stdio dispatch is synchronous. A cancellation remembered before its matching
request is consumed once. Kujo retains up to 1,024 cancellation IDs and 8 MiB
of combined string ID data, evicting the oldest entries first.

## Position Encoding Contract

All incoming LSP positions and outgoing ranges use UTF-16 code units, the protocol default. Positions that split a surrogate pair are rejected as invalid. Semantic-token starts and lengths are derived from the original source spans, so quoted strings, numeric spelling, and non-BMP characters retain protocol-correct ranges. Token positions advance through the source in order. Other outgoing ranges use an index of line lengths and surrogate-pair positions.

## Startup And First-Response Latency Baselines

Reliability guardrails track startup and first-response latency using conservative averages over repeated runs:

- startup average guardrail: `< 20ms`
- first completion response average guardrail: `< 80ms`

Validation source:

- `tests/lsp_reliability_track.rs` (`startup_and_first_response_latency_stay_within_guardrails`)
- `tests/lsp_latency_guardrails.rs` (full-file analysis and repeated cached
  completion/diagnostic/hover guardrails)
- `src/analyzed_program.rs` (project-import cache reuse and content-change
  invalidation)

These thresholds are intended to detect severe regressions while staying stable on shared CI hosts.
