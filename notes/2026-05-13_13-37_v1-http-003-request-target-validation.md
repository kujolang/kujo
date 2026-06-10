# Kujo Field Notes — V1 HTTP 003 Request Target Validation

**Date:** 2026-05-13
**Session:** 13:37 local
**Branch/Commit:** main / a7c4da3
**Scope:** Hardened static-server request-target parsing/validation for `kujo serve` and added regression coverage for malformed/encoded request-target cases.

---

## What I Changed
- Added centralized request-target validation in `src/serve_http.rs`.
- Enforced request-target rules before filesystem resolution:
  - Reject fragments in request target.
  - Parse path separately from query.
  - Percent-decode exactly once.
  - Reject malformed percent encoding (`400`).
  - Reject decoded null bytes (`400`).
  - Reject unsafe traversal/invalid decoded paths (`403`).
  - Reject oversized targets above `4096` bytes (`414`).
- Added new unit tests in `src/serve_http.rs` for validator behavior (decode-once, invalid encoding, null bytes, fragment, traversal, URI length, query isolation).
- Added integration regressions in `tests/serve_command_integration.rs` for malformed and traversal-oriented request targets.
- Updated `CHANGELOG.md`, `README.md`, and `ROADMAP.md` to document the new contract and marked `V1-HTTP-003` complete.

## Gotchas (Read This Next Time)
- **Gotcha:** Reusing legacy URL-encoded traversal helpers can change status-code semantics.
  - **Symptom:** Null-byte request test returned `403` instead of expected `400`.
  - **Root cause:** `reject_url_encoded_parent_traversal(...)` classifies some malformed decoded paths as traversal-style failures.
  - **Fix:** Keep malformed-percent/null-byte handling in dedicated request-target validation before traversal classification.
  - **Prevention:** Preserve this rule: malformed target syntax (`%` decode failures, decoded null bytes, fragments) is `400`; only decoded-path containment/safety violations are `403`.

- **Gotcha:** `cargo fmt` can introduce broad unrelated file churn in this repo workflow.
  - **Symptom:** Multiple unrelated files showed modified after formatting.
  - **Root cause:** Repository-wide formatting touched files outside the selected roadmap item scope.
  - **Fix:** Revert unrelated files and keep commits scoped to the roadmap item.
  - **Prevention:** Prefer scoped edits/formatting for targeted roadmap work, and verify `git status` before committing.

## Things I Learned
- The safest shape for static server request-target handling is a single, centralized validation function that maps each failure mode to a deterministic HTTP status.
- Keeping decode-once behavior explicit avoids accidental double-decoding while still preventing path escapes.
- Query-string path isolation is worth testing explicitly even when the implementation appears straightforward.

## Debug Notes (Only if applicable)
- **Failing test / error:** `serve_rejects_invalid_percent_encoding_with_400` initially failed with expected `400`, got `404`.
- **Repro steps:** `cargo test --test serve_command_integration serve_rejects_invalid_percent_encoding_with_400 -- --nocapture`
- **Breakpoints / logs used:** Regression test assertions and targeted reruns for individual serve tests.
- **Final diagnosis:** The server used raw path splitting + path sanitization without explicit malformed-percent rejection, so malformed `%` sequences fell through to path lookup and returned `404`.

## Follow-ups / TODO (For Future Agents)
- [ ] Implement `V1-HTTP-004` hidden/private file policy and ensure it composes cleanly with `V1-HTTP-003` status-code contracts.
- [ ] Consider whether request-target limit should become configurable in later server-hardening items (`V1-HTTP-005`) and document precedence/defaults.

## Links / References
- Files touched:
  - `src/serve_http.rs`
  - `tests/serve_command_integration.rs`
  - `src/path_security.rs`
  - `CHANGELOG.md`
  - `README.md`
  - `ROADMAP.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `ROADMAP.md`
