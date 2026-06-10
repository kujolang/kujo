# V1H-SEC-001 — Outbound destination policy layer for network client APIs

Date: 2026-05-23
Item: V1H-SEC-001

## Summary
Implemented a deterministic outbound destination policy layer for HTTP/TCP/UDP client-native surfaces so operators can deny private/link-local/loopback/multicast/unspecified destinations without breaking existing defaults.

## Implementation
- Added destination policy evaluation to `src/network_policy.rs`:
  - `enforce_host_port_destination_policy(host, port, surface)`
  - `enforce_http_url_destination_policy(url, surface)`
- Added policy controls:
  - `KUJO_NET_DESTINATION_POLICY`:
    - `allow_all` (default, backward compatible)
    - `deny_private` (blocks private/loopback/link-local/multicast/unspecified)
  - `KUJO_ALLOW_PRIVATE_NETWORK_DESTINATIONS=true` override for trusted workflows.
- Wired enforcement into outbound client paths:
  - HTTP builtins (`http_get`, `http_post`, `http_put`, `http_delete`, `http_get_binary`, `http_get_stream`, `oauth2_get_token`)
  - Interpreter HTTP native functions (`run_ai_request`, `parallel_http`, `http_request`)
  - Interpreter network native functions (`tcp_connect`, `udp_send_to`).

## Tests Added
- Unit tests in `src/network_policy.rs` for:
  - default allow-all behavior
  - deny-private loopback block behavior
  - deny-private public-target allow behavior
  - strict-mode override behavior
  - invalid mode deterministic diagnostic
  - URL-based destination enforcement
- Integration tests in `tests/native_api_security_boundaries.rs`:
  - deny-private blocks loopback HTTP client
  - deny-private blocks loopback TCP client
  - trusted override allows loopback HTTP client

## Validation
- `cargo test --test native_api_security_boundaries` ✅ (46 passed)
- `cargo test --test runtime_security` ✅ (9 passed)
- `cargo test` ✅ (full suite passed; no failures)

## Backward Compatibility
- Default mode remains permissive (`allow_all`), preserving existing valid outbound behavior unless operators opt into strict mode.
- Existing `http`/`https` call shapes and runtime API contracts unchanged.
