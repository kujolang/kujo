# Kujo Field Notes — V1-HTTP-002 Static Response Status/Header Hardening

**Date:** 2026-05-13
**Session:** 13:22 local
**Branch/Commit:** main / a4831cd
**Scope:** Completed roadmap item V1-HTTP-002 by tightening `kujo serve` response status/header behavior, adding central text-response construction, and expanding static-server regression coverage.

---

## What I Changed
- Updated `src/serve_http.rs` to route text/error responses through `static_text_response(...)` instead of ad hoc calls.
- Added method handling split in `build_response(...)`:
  - standard unsupported methods now return `405 Method Not Allowed` with `Allow: GET, HEAD`
  - non-standard methods now return `501 Not Implemented`
- Hardened default response headers by always adding `Referrer-Policy: no-referrer` (with existing `X-Content-Type-Options: nosniff`).
- Added conservative success-path cache fallback when no explicit max-age is configured (`public, max-age=0, must-revalidate`).
- Added unit coverage in `src/serve_http.rs` for conservative-cache fallback, static text response header contract, and deterministic `500` mapping shape.
- Added integration coverage in `tests/serve_command_integration.rs` for:
  - GET response length/type/default-header shape
  - POST `405` + `Allow` behavior
  - non-standard method `501` behavior
  - `403` and `404` error response header shape
- Updated `README.md`, `CHANGELOG.md`, and `ROADMAP.md` for V1-HTTP-002 completion and behavior contract changes.

## Gotchas (Read This Next Time)
- **Gotcha:** `cargo test` accepts only one positional filter argument.
  - **Symptom:** `cargo test --test <file> test_a test_b` fails with `unexpected argument`.
  - **Root cause:** Cargo test CLI only supports a single substring filter token.
  - **Fix:** Use one filter pattern (for example `serve_`) or run tests separately.
  - **Prevention:** Keep repro commands to one positional test filter and use `-- --nocapture`/flags for additional control.

- **Gotcha:** Running repo-wide `cargo fmt` can touch unrelated files and accidentally broaden roadmap-item scope.
  - **Symptom:** Unrelated files became modified after formatting even though implementation touched only static-serve files.
  - **Root cause:** Workspace formatting applies globally and may reformat files unrelated to the current item.
  - **Fix:** Restore unrelated files immediately and re-run targeted tests.
  - **Prevention:** Prefer targeted formatting or verify `git status --short` immediately after formatting before committing.

## Things I Learned
- Static-server response-shape correctness is easiest to maintain when text/error responses are centralized through one helper; it avoids header drift between 403/404/405/500/501 paths.
- For this codebase, integration tests that launch `kujo serve` provide high-signal verification of real HTTP behavior, while unit tests are useful for hard-to-trigger branches (for example synthetic unexpected IO -> 500 mapping).
- `Referrer-Policy` is now baseline behavior for serve responses, not only hardened mode.

## Debug Notes (Only if applicable)
- **Failing test / error:** `serve_method_not_allowed_returns_allow_header` and `serve_non_standard_method_returns_501` failed initially (`Allow` missing and status returned `405` instead of `501`).
- **Repro steps:** `cargo test --test serve_command_integration serve_`
- **Breakpoints / logs used:** Focused on `build_response(...)` method gate in `src/serve_http.rs`.
- **Final diagnosis:** Unsupported-method handling used one generic `405` path and lacked an `Allow` header; no split for non-standard methods.

## Follow-ups / TODO (For Future Agents)
- [ ] Implement `V1-HTTP-003` request-target validation hardening (decode/validation policy, invalid percent handling, URI length limits).
- [ ] Implement `V1-HTTP-004` hidden/private file policy decisions (403 vs 404 contract) with integration coverage.

## Links / References
- Files touched:
  - `src/serve_http.rs`
  - `tests/serve_command_integration.rs`
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `notes/README.md`
  - `notes/GOTCHAS.md`
  - `ROADMAP.md`
