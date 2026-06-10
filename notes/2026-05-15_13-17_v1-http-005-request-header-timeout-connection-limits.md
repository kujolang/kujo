# Kujo Field Notes — V1-HTTP-005 Serve Limits Hardening

**Date:** 2026-05-15
**Session:** 13:17 local
**Branch/Commit:** main / working tree
**Scope:** Implemented roadmap item V1-HTTP-005 by adding centralized `kujo serve` request/header/body/concurrency limits, new serve CLI limit flags, and regression coverage for oversized request surfaces.

---

## What I Changed
- Added new `kujo serve` option wiring in `src/main.rs`:
  - `--max-request-line-bytes`
  - `--max-header-bytes`
  - `--max-header-count`
  - `--max-request-body-bytes`
  - `--read-timeout-ms`
  - `--write-timeout-ms`
  - `--max-connections`
- Extended `ServeServerOptions` in `src/serve_http.rs` with the above limit fields.
- Added centralized request-limit checks in `validate_request_limits(...)` for request-line length, combined header bytes, header count, and request body length.
- Added deterministic status mapping for limit failures:
  - request line overflow -> `414`
  - header/body overflow -> `413`
- Added bounded concurrent request handler guard (`try_acquire_request_slot`) with deterministic `503 Service Unavailable` when saturated.
- Switched server loop from `incoming_requests()` to `recv_timeout(options.read_timeout)` to apply explicit receive-loop timeout pacing.
- Added/updated tests:
  - `tests/serve_command_integration.rs`:
    - `serve_oversized_request_line_returns_414`
    - `serve_oversized_headers_return_413`
    - `serve_too_many_headers_return_413`
    - `serve_custom_timeout_flags_still_serve_normal_requests`
  - `src/serve_http.rs` unit coverage for option validation (`validate_server_options_rejects_zero_timeout_and_connection_limits`).
- Updated docs and release tracking:
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md` (`V1-HTTP-005` marked complete with verification evidence)

## Gotchas (Read This Next Time)
- **Gotcha:** Setting socket receive/write timeouts on the listening socket caused flaky server exits.
  - **Symptom:** `serve_command_integration` intermittently failed with `Connection refused` after startup.
  - **Root cause:** Listener-level timeout configuration affected accept-loop behavior under `tiny_http`, producing accept errors that shut down the server loop.
  - **Fix:** Removed listener-level `setsockopt` timeout configuration and used `server.recv_timeout(...)` in the serve loop instead.
  - **Prevention:** Do not apply `SO_RCVTIMEO`/`SO_SNDTIMEO` to the listening socket for `tiny_http` lifecycle control; use server-loop timeout APIs that do not terminate the accept path.

## Things I Learned
- `tiny_http` exposes reliable queue-level timeout control (`recv_timeout`) but not a direct accepted-stream timeout API from Kujo’s integration surface.
- Limit checks are safer when centralized before method/path handling so request-shape failures cannot bypass policy on method variants.
- Existing serve integration harness is good for protocol contract checks, but timeout behavior needs deterministic local assumptions to avoid flaky partial-request tests.

## Debug Notes (Only if applicable)
- **Failing test / error:** Full `cargo test` initially failed in `tests/serve_command_integration.rs` with connection refusals after server startup.
- **Repro steps:** Run `cargo test` after enabling listener-level socket timeout configuration in `run_static_server`.
- **Breakpoints / logs used:** Reviewed serve startup path and accepted-request loop behavior while comparing targeted `serve_` test run vs full suite.
- **Final diagnosis:** Listener socket timeout settings caused accept-loop instability in `tiny_http` context; removing that path restored stable process lifetime.

## Follow-ups / TODO (For Future Agents)
- [ ] Revisit whether per-connection read/write socket deadlines can be safely enforced with current `tiny_http` architecture without introducing accept-loop instability.
- [ ] If stricter timeout semantics become release-critical, consider a server backend abstraction with explicit stream-level timeout control.

## Links / References
- Files touched:
  - `src/main.rs`
  - `src/serve_http.rs`
  - `tests/serve_command_integration.rs`
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `notes/README.md`
  - `notes/GOTCHAS.md`
  - `ROADMAP.md`
