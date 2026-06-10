# Kujo Field Notes — V1-NET-001 Network Timeout and Size Controls

**Date:** 2026-05-13
**Session:** 12:56 local
**Branch/Commit:** main / 1cff039
**Scope:** Implemented `V1-NET-001` network hardening with centralized timeout/body-size policy and regression coverage for HTTP/TCP/UDP boundary behavior.

---

## What I Changed
- Added shared network guardrail module: `src/network_policy.rs`.
- Wired default timeout + max-body controls into network/HTTP surfaces:
  - `src/interpreter/native_functions/network.rs`
  - `src/interpreter/native_functions/http.rs`
  - `src/builtins.rs`
- Added dedicated regression tests:
  - `src/interpreter/native_functions/mod.rs` (`test_release_hardening_network_module_size_limit_contracts`)
  - `tests/native_api_security_boundaries.rs` (`network_http_get_rejects_oversized_response_body`, `network_http_request_timeout_is_reported_deterministically`)
- Updated roadmap/docs/changelog/readme contracts for completed `V1-NET-001`.

## Gotchas (Read This Next Time)
- **Gotcha:** `reqwest::blocking` can panic when executed on a Tokio runtime thread in Kujo execution paths.
  - **Symptom:** Runtime exited with panic text like `Cannot drop a runtime in a context where blocking is not allowed`.
  - **Root cause:** Blocking HTTP clients were created/dropped in execution contexts that can already be inside Tokio runtime workers.
  - **Fix:** Added `network_policy::run_blocking_http_task(...)` to run blocking HTTP work in a dedicated OS thread and route HTTP paths through it.
  - **Prevention:** For blocking HTTP in Kujo native APIs, always isolate execution from runtime worker threads (thread offload or fully async path), then enforce timeout/size policy in the shared helper.

## Things I Learned
- Centralizing network limits in one module (`network_policy`) avoids drift between `builtins` and `native_functions` HTTP/TCP/UDP surfaces.
- Size-limit checks need both `Content-Length` precheck and streaming bounded-read enforcement to cover missing/incorrect length headers.
- Local one-shot HTTP fixtures are reliable for timeout/oversize tests, but server-thread joins can deadlock when clients abort early after boundary errors.

## Debug Notes (Only if applicable)
- **Failing test / error:** `network_http_get_rejects_oversized_response_body` initially failed with exit `101` and panic message `Cannot drop a runtime in a context where blocking is not allowed`.
- **Repro steps:** `cargo test --test native_api_security_boundaries network_http_get_rejects_oversized_response_body -- --nocapture`
- **Breakpoints / logs used:** Read stderr from integration test output and traced HTTP call flow through `builtins.rs` / `http.rs`.
- **Final diagnosis:** Blocking reqwest execution context was unsafe; moving blocking HTTP work to a dedicated thread resolved the panic and kept policy checks deterministic.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider applying the same timeout/body-size policy abstraction to async HTTP native helpers for full parity (`async_http_get` / `async_http_post`).
- [ ] If future work adds per-call network policy overrides, keep the centralized defaults as strict floor constraints instead of reintroducing unbounded behavior.

## Links / References
- Files touched:
  - `src/network_policy.rs`
  - `src/builtins.rs`
  - `src/interpreter/native_functions/http.rs`
  - `src/interpreter/native_functions/network.rs`
  - `src/interpreter/native_functions/mod.rs`
  - `tests/native_api_security_boundaries.rs`
  - `ROADMAP.md`
  - `README.md`
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
  - `CHANGELOG.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
