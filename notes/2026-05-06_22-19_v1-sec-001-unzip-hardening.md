# Kujo Field Notes — V1-SEC-001 unzip hardening

**Date:** 2026-05-06
**Session:** 22:19 local
**Branch/Commit:** main / 59bc30a
**Scope:** Implemented roadmap item V1-SEC-001 to harden `unzip` against path traversal and archive-exhaustion abuse. Added integration regressions and updated release/security docs.

---

## What I Changed
- Added centralized unzip security helpers in `src/interpreter/native_functions/filesystem.rs`:
  - archive entry path sanitization
  - extraction-target containment checks
  - symlink target-path rejection
  - deterministic extraction limits (entry count, per-entry size, total size)
- Replaced direct unzip extraction loop with `extract_zip_archive_with_limits(...)` to keep all unsafe-entry checks centralized.
- Expanded `tests/native_api_security_boundaries.rs` with unzip-specific regression tests for:
  - parent traversal (`../`)
  - absolute paths
  - drive-prefixed entries
  - null-byte names
  - symlink entry metadata
  - entry-count, single-entry-size, and total-size limits
  - safe nested extraction success
- Updated user/security docs in `CHANGELOG.md`, `README.md`, `docs/NATIVE_API_SECURITY_POSTURE.md`, and marked `V1-SEC-001` complete in `ROADMAP.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** `zip` crate `FileOptions::unix_permissions(...)` in `zip = 0.5` masks mode bits to `0o777`.
  - **Symptom:** A test archive entry intended to be a symlink was treated as a normal file, so symlink-rejection tests falsely passed extraction.
  - **Root cause:** The writer API drops file-type bits (including symlink bit) when setting permissions.
  - **Fix:** Built a normal zip entry, then patched central-directory metadata (`version_made_by` host + external attributes) so `ZipFile::unix_mode()` reports symlink mode.
  - **Prevention:** For symlink-flag tests under `zip = 0.5`, do not rely on `unix_permissions`; verify or patch central-directory metadata explicitly.

## Things I Learned
- Unzip hardening is safer when every path/limit check runs through one helper instead of inline branch-by-branch checks.
- Canonical-root checks are still needed after directory creation to catch symlink-based escapes from pre-existing filesystem state.
- For archive limits, checking `ZipFile::size()` before extraction gives deterministic fail-fast behavior without consuming output-disk space.

## Debug Notes (Only if applicable)
- **Failing test / error:** `unzip_rejects_symlink_entries` initially failed with `expected unzip boundary failure with exit code 1, got status=Some(0)`.
- **Repro steps:** `cargo test --test native_api_security_boundaries unzip_ -- --nocapture`.
- **Breakpoints / logs used:** Compared `zip` writer/read behavior and inspected crate source for permission handling.
- **Final diagnosis:** Test fixture did not encode a symlink file type; runtime check was correct for provided metadata but fixture was wrong.

## Follow-ups / TODO (For Future Agents)
- [ ] Reuse or extract the new unzip containment helpers when implementing broader path-boundary hardening in `V1-FS-001`.
- [ ] Consider making extraction limits configurable via explicit trusted-mode policy once capability controls land.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/filesystem.rs`
  - `tests/native_api_security_boundaries.rs`
  - `CHANGELOG.md`
  - `README.md`
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
  - `ROADMAP.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/README.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
