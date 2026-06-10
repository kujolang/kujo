# Kujo Field Notes — V1-TEST-003 runtime and native security regressions

**Date:** 2026-05-16
**Session:** 16:59 local
**Branch/Commit:** main / 37e1818
**Scope:** Expanded Phase 9 security regression coverage for malicious source execution, module boundary enforcement, and static-server abusive request handling under roadmap item `V1-TEST-003`.

---

## What I Changed
- Added a new integration suite at `tests/runtime_security.rs`.
- Added malicious-source diagnostics regressions for:
  - invalid escape sequences
  - oversized string literals (`DEFAULT_MAX_STRING_LITERAL_LENGTH + 1`)
  - oversized array literals (`DEFAULT_MAX_COLLECTION_LITERAL_ITEMS + 1`)
- Added runtime misuse regressions for `break`/`continue` outside loops with deterministic runtime-error assertions.
- Added interpreter recursion-depth regression that exceeds `DEFAULT_MAX_INTERPRETER_CALL_DEPTH` and asserts deterministic call-stack-limit errors.
- Added end-to-end module-cycle regression (CLI `run --interpreter`) asserting explicit cycle chain diagnostics.
- Added Unix-only end-to-end module symlink-escape regression asserting module-search-root containment rejection.
- Extended `tests/serve_command_integration.rs` with `serve_request_body_over_limit_returns_413_before_method_dispatch` to assert request-body limit enforcement precedes method-level `405` handling.
- Updated roadmap/docs/changelog surfaces for `V1-TEST-003` completion metadata.

## Gotchas (Read This Next Time)
- **Gotcha:** Static-server request-size checks run before method dispatch checks.
  - **Symptom:** Oversized `POST` requests can return `413` instead of `405` even though only `GET`/`HEAD` are supported.
  - **Root cause:** `enforce_request_limits(...)` runs before request method validation in `src/serve_http.rs`.
  - **Fix:** Assert `413` contract directly for oversized body tests and avoid expecting method-specific status first.
  - **Prevention:** For serve hardening tests, model request evaluation as: limits/target validation first, method semantics second.

- **Gotcha:** Module symlink-escape regression is OS-specific.
  - **Symptom:** Symlink boundary tests cannot run on non-Unix platforms with `std::os::unix::fs::symlink`.
  - **Root cause:** The current integration harness uses Unix symlink APIs.
  - **Fix:** Gate symlink-escape regression with `#[cfg(unix)]`.
  - **Prevention:** Keep cross-platform suites deterministic by isolating platform-specific filesystem primitives.

## Things I Learned
- The project already had broad native API security tests in `tests/native_api_security_boundaries.rs`; the missing coverage for this roadmap item was malicious source/runtime and explicit runtime security aggregation.
- A dedicated `tests/runtime_security.rs` suite provides a clearer security regression entrypoint than expanding unrelated test files further.
- Runtime limits in `src/runtime_limits.rs` are stable to consume directly in tests and make boundary test generation deterministic.

## Debug Notes (Only if applicable)
- **Failing test / error:** None after first implementation pass.
- **Repro steps:** N/A.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** Existing code paths already enforced expected security behavior; work was primarily test-surface expansion and contract locking.

## Follow-ups / TODO (For Future Agents)
- [ ] Add non-Unix equivalent module-escape regression strategy if Windows-first CI coverage becomes required.
- [ ] When `V1-TEST-004` lands, include runtime-security diagnostics fixtures in golden output snapshots where feasible.

## Links / References
- Files touched:
  - `tests/runtime_security.rs`
  - `tests/serve_command_integration.rs`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `README.md`
- Related docs:
  - `notes/README.md`
  - `notes/GOTCHAS.md`
  - `ROADMAP.md`
