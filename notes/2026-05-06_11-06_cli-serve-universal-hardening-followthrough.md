# Kujo Field Notes — CLI serve universal hardening follow-through

**Date:** 2026-05-06
**Session:** 11:06 local
**Branch/Commit:** main / working tree
**Scope:** Completed the universal serve follow-through by moving runtime serving behavior into a dedicated module, expanding HTTP semantics and hardening defaults, wiring new CLI options, and validating with focused tests.

---

## What I Changed
- Added dedicated serve runtime module: `src/serve_http.rs`.
- Routed `Commands::Serve` in `src/main.rs` to `serve_http::run_static_server(...)`.
- Expanded `kujo serve` options:
  - `--hardened`
  - `--cache-max-age`
  - `--access-log`
  - `--tls-cert`
  - `--tls-key`
- Implemented richer static serving behavior:
  - GET/HEAD handling with 405 for unsupported methods
  - conditional requests via ETag/If-None-Match (304)
  - single-range byte serving (206) and range rejection (416)
  - precompressed asset selection (`.br`, `.gz`) based on `Accept-Encoding`
  - deterministic status mapping for read/canonicalization errors (404/403/500)
  - security and cache headers with hardened profile support
  - HSTS emitted only for secure requests
- Preserved existing legacy helper tests in `src/main.rs` as `#[cfg(test)]` while runtime now uses `serve_http`.
- Updated `README.md` with complete serve option docs and behavior summary.

## Gotchas (Read This Next Time)
- Large multi-hunk edits in `src/main.rs` are brittle; module extraction (`src/serve_http.rs`) is safer and easier to validate.
- `tiny_http` header helpers can have lifetime/type quirks; direct case-insensitive string comparison is more robust than relying on `equiv(...)` with dynamic names.
- Keep TLS pair validation strict: if either `--tls-cert` or `--tls-key` is provided, require both.

## Things I Learned
- Rule: when static serving behavior starts accumulating protocol/security logic, keep it in a dedicated module (`src/serve_http.rs`) and keep CLI dispatch thin in `src/main.rs`.
- Rule: confidence for serving changes should include both unit coverage and subprocess-level socket tests, because request/response wiring bugs do not always show up in pure helper tests.
- Rule: treat "unrelated full-suite failures" as explicit evidence in notes, not silent assumptions.

## Debug Notes (Only if applicable)
- **Failing test / error:** `vm::tests::test_sum_int_map_until_local_in_place_missing_key_errors` failed in full-suite run with `Expected runtime error, got value: Int(0)`.
- **Repro steps:** Run `cargo test --bin kujo` from repo root.
- **Breakpoints / logs used:** Focused test isolation with `cargo test --bin kujo serve_http::tests::`, `cargo test --bin kujo build_serve_response`, and `cargo test --test serve_command_integration` to confirm serve changes were not the source.
- **Final diagnosis:** Full-suite failure is an existing VM-path issue unrelated to serve-module changes in this session.

## Follow-ups / TODO
- [x] Added black-box integration tests that start the server and verify HTTP behavior over real sockets.
- [ ] Extend black-box coverage to HTTPS/TLS handshake and certificate/key-path error cases.
- [ ] Add explicit CLI switches for cache-control presets (`--cache-public`, `--cache-private`, `--no-cache`) if preview workflows demand finer control.
- [ ] Consider structured log output mode for `--access-log` to improve machine parsing.

## Links / References
- Files touched:
  - `src/main.rs`
  - `src/serve_http.rs`
  - `tests/serve_command_integration.rs`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `notes/2026-05-06_10-09_cli-serve-command-holistic-preview.md`
