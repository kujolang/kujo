# Kujo Field Notes — V1-HTTP-001 MIME Policy Unification

**Date:** 2026-05-06
**Session:** 22:32 local
**Branch/Commit:** main / 3f47894
**Scope:** Completed roadmap item V1-HTTP-001 by unifying static-server MIME and active-content fallback behavior in production code and removing duplicate test-only policy logic.

---

## What I Changed
- Centralized static MIME/fallback behavior in `src/serve_http.rs`:
  - added a single known-extension MIME registry (`known_mime_for_extension(...)`)
  - made extension matching case-insensitive
  - enforced fallback to `application/octet-stream` for unknown extensions
  - enforced fallback to `application/octet-stream` for extensionless files
- Added/expanded production-path tests in `src/serve_http.rs` for:
  - required MIME mappings (HTML/CSS/JS/JSON/PNG/JPG/SVG/WASM/font/PDF/text)
  - case-insensitive extension matching
  - unknown-extension fallback
  - extensionless fallback
  - unknown active-content fallback
  - `X-Content-Type-Options: nosniff` on file responses
- Removed duplicated `#[cfg(test)]` static-serve helpers and MIME logic from `src/main.rs` to eliminate test/production policy drift.
- Added live integration coverage in `tests/serve_command_integration.rs` to validate MIME/security behavior through the real `kujo serve` subprocess path.
- Updated roadmap/docs for the completed item in `ROADMAP.md`, `README.md`, and `CHANGELOG.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** MIME policy in `main.rs` test-only helpers can diverge from real serve behavior.
  - **Symptom:** Tests report safe fallback for unknown active content while production server still serves sniffed active MIME (for example `text/html`).
  - **Root cause:** Duplicate MIME/security logic lived behind `#[cfg(test)]` in `src/main.rs` instead of the production serving module.
  - **Fix:** Keep MIME/security logic only in `src/serve_http.rs` and run assertions against production helpers and live serve integration tests.
  - **Prevention:** Avoid cloning request/response policy code in CLI test modules; test through production module APIs or subprocess integration.
- **Gotcha:** The serve subprocess readiness loop can be too short on busy hosts.
  - **Symptom:** Intermittent failure: `kujo serve did not become reachable in time on port ...`.
  - **Root cause:** Startup polling window in `tests/serve_command_integration.rs` was only 40 x 50ms.
  - **Fix:** Increased polling attempts to 100 to reduce flakiness while preserving behavior checks.
  - **Prevention:** Keep integration readiness windows conservative enough for loaded CI/local environments.

## Things I Learned
- Static-server content-type policy is a security contract, not just a UX concern; unknown-extension behavior must be deterministic and centralized.
- For this release phase, extensionless and unknown extensions are intentionally treated as download-safe bytes (`application/octet-stream`) rather than type-inferred content.
- Removing duplicated test helpers improves trust in integration tests because assertions track the real runtime path.

## Debug Notes (Only if applicable)
- **Failing test / error:** `serve_http::tests::guess_content_type_blocks_inferred_active_content_for_unknown_extension` failed initially with `left: "text/html" right: "application/octet-stream"`.
- **Repro steps:** `cargo test guess_content_type_blocks_inferred_active_content_for_unknown_extension`.
- **Breakpoints / logs used:** Focused on `guess_content_type(...)` in `src/serve_http.rs`; compared against old `#[cfg(test)]` helper behavior in `src/main.rs`.
- **Final diagnosis:** Production path allowed inferred MIME for unknown extension; test-only path forced safe fallback.

## Follow-ups / TODO (For Future Agents)
- [ ] When implementing `V1-HTTP-007`, extend `known_mime_for_extension(...)` instead of reintroducing fallback sniffing.
- [ ] If server startup latency grows further, consider replacing fixed retry loops with bounded deadline-based readiness checks.

## Links / References
- Files touched:
  - `src/serve_http.rs`
  - `src/main.rs`
  - `tests/serve_command_integration.rs`
  - `ROADMAP.md`
  - `README.md`
  - `CHANGELOG.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/README.md`
  - `ROADMAP.md`
