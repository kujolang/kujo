# Kujo Field Notes — V1-FS-001 path security centralization

**Date:** 2026-05-12
**Session:** 23:06 local
**Branch/Commit:** main / 8044501
**Scope:** Implemented `V1-FS-001` by introducing one reusable path security module and wiring it into archive extraction, static serving, and module resolution. Added regression tests for encoded traversal and symlink escape paths.

---

## What I Changed
- Added `src/path_security.rs` with shared helpers for:
  - lexical path sanitization (`null byte`, empty path, parent traversal, absolute path, drive-prefix rejection)
  - root-bounded join/canonical containment checks
  - symlink target rejection helper
  - URL-encoded traversal guard for serve request paths
- Updated `src/interpreter/native_functions/filesystem.rs` to replace archive path/symlink/containment ad hoc checks with shared helper usage.
- Updated `src/serve_http.rs` to reject URL-encoded traversal and route request path normalization/root containment through shared helpers.
- Updated `src/module.rs` to bound resolved module files to canonical search roots and reject symlink escapes outside allowed roots.
- Added regression tests in `tests/serve_command_integration.rs`:
  - `serve_rejects_url_encoded_parent_traversal`
  - `serve_rejects_symlink_escape_target` (unix)
- Added regression test in `src/module.rs`:
  - `load_module_rejects_symlink_escape_outside_search_root` (unix)
- Added helper-unit tests in `src/path_security.rs` for success/failure path normalization and encoded-traversal behavior.

## Gotchas (Read This Next Time)
- **Gotcha:** Canonical root checks alone did not reject encoded traversal payloads.
  - **Symptom:** `GET /%2e%2e/secret.txt` returned `404` instead of deterministic traversal rejection.
  - **Root cause:** Existing static-server flow checked containment after filesystem canonicalization and never decoded traversal sequences in the request target.
  - **Fix:** Added pre-resolution URL-encoded traversal guard in shared path helper and enforced it in `src/serve_http.rs`.
  - **Prevention:** Treat request-target path validation as a separate security boundary before any filesystem join/canonicalize logic.
- **Gotcha:** Module loader accepted symlinked module files outside search roots.
  - **Symptom:** Importing a module through a symlink under a search root loaded source from outside that root.
  - **Root cause:** Resolution used `search_path.join(filename).exists()` without canonical candidate-vs-root containment enforcement.
  - **Fix:** Canonicalized both search root and candidate module path, then rejected non-contained candidates.
  - **Prevention:** For any root-bound policy, canonicalize both root and resolved candidate before `starts_with` containment checks.

## Things I Learned
- Path policy should be centralized even when call sites need caller-specific error messages.
- Security-critical path checks need both lexical validation and canonical containment.
- URL/path validation for HTTP and filesystem/module access should share primitives but keep caller-specific response/error behavior.

## Debug Notes (Only if applicable)
- **Failing test / error:** `serve_rejects_url_encoded_parent_traversal` expected `403`, got `404`.
- **Repro steps:** `cargo test --test serve_command_integration serve_rejects_url_encoded_parent_traversal -- --nocapture`
- **Breakpoints / logs used:** Focused on `build_response` request-path handling in `src/serve_http.rs`.
- **Final diagnosis:** Encoded traversal was not guarded pre-resolution; canonicalization-based checks were too late to enforce deterministic traversal rejection.

## Follow-ups / TODO (For Future Agents)
- [ ] Reuse `src/path_security.rs` in `V1-FS-002` for write/delete bounds and overwrite policy plumbing.
- [ ] Extend server request-target parsing with single-decode + invalid-percent handling under `V1-HTTP-003`.

## Links / References
- Files touched:
  - `src/path_security.rs`
  - `src/interpreter/native_functions/filesystem.rs`
  - `src/serve_http.rs`
  - `src/module.rs`
  - `tests/serve_command_integration.rs`
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `notes/GOTCHAS.md`
