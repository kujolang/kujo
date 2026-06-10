# V1H-SEC-002 — HTTP URL scheme/host validation hardening

Date: 2026-05-23
Item: V1H-SEC-002

## Summary
Added explicit, deterministic URL-target validation for HTTP native calls so malformed URLs and unsupported schemes are rejected before request execution while preserving valid `http`/`https` behavior.

## Implementation
- Updated `src/network_policy.rs`:
  - Added explicit scheme allowlist (`http`, `https`) to `enforce_http_url_destination_policy`.
  - Added deterministic rejection message for unsupported schemes.
  - Kept malformed URL path with stable `invalid URL` diagnostic prefix.
- This shared pre-validation path is already invoked by builtin and interpreter-native HTTP client surfaces, so behavior is centralized and consistent.

## Tests Added/Updated
- Unit test:
  - `network_policy::tests::outbound_policy_http_url_evaluation_rejects_unsupported_scheme`
- Integration tests (`tests/native_api_security_boundaries.rs`):
  - `network_http_client_rejects_unsupported_url_scheme_before_request_execution`
  - `network_http_client_rejects_malformed_url_before_request_execution`

## Validation
- Focused development checks:
  - `cargo test network_policy::tests::outbound_policy_http_url_evaluation_rejects_unsupported_scheme` ✅
  - `cargo test --test native_api_security_boundaries network_http_client_rejects_unsupported_url_scheme_before_request_execution` ✅
- Required loop checks:
  - `cargo test --test native_api_security_boundaries` ✅ (48 passed)
  - `cargo test --test runtime_security` ✅ (9 passed)
  - `cargo test` ⚠️ blocked by unrelated perf guardrail failure:
    - `tests/lsp_latency_guardrails.rs`
    - `diagnostics average latency exceeded guardrail: 156.446247ms` (rerun: `158.450357ms`)

## Backward Compatibility
- Existing valid `http` and `https` flows remain supported.
- Unsupported schemes now fail deterministically before network execution.
- Destination-policy behavior from `V1H-SEC-001` remains unchanged.
