# Kujo Field Notes — V1-FS-002 file IO safeguards

**Date:** 2026-05-13
**Session:** 00:13 local
**Branch/Commit:** main / 327e1d3
**Scope:** Implemented V1-FS-002 filesystem safeguards: bounded read/write payloads, explicit overwrite semantics, delete-path hardening, and regression coverage updates.

---

## What I Changed
- Added centralized file-operation guardrails in src/interpreter/native_functions/filesystem.rs:
  - 8 MiB max whole-file read limit for read_file/read_lines/read_binary_file/read_file_async
  - 8 MiB max write payload limit for write_file/write_file_sync/write_file_async/write_binary_file/append_file
  - explicit overwrite policy for write_file/write_binary_file via third bool argument overwrite
  - explicit directory-path rejection in delete_file
- Updated type-check signatures in src/type_checker.rs to accept optional overwrite argument for write_file/write_binary_file.
- Updated native dispatch contract assertions in src/interpreter/native_functions/mod.rs for new write arity/error behavior.
- Added integration regressions in tests/native_api_security_boundaries.rs for:
  - oversized read/write failures
  - overwrite denied by default and allowed with overwrite=true
  - fs-delete deny/allow capability behavior
  - non-recursive directory delete behavior
  - boundary-at-limit read/write success
- Updated docs in README.md, ROADMAP.md, CHANGELOG.md, and docs/NATIVE_API_SECURITY_POSTURE.md.

## Gotchas (Read This Next Time)
- **Gotcha:** write_file overwrite behavior is validated before IO, but type-check warning arity must also allow the optional third arg.
  - **Symptom:** Runtime rejected write_file(path, content, true) with "expects 2-2 arguments" warning + write_file arity error even before overwrite logic existed.
  - **Root cause:** src/type_checker.rs still registered write_file as exactly two parameters.
  - **Fix:** Updated write_file and write_binary_file signatures to accept an optional third parameter.
  - **Prevention:** For native signature changes, always update runtime handler + type-check signature + dispatch contract tests together.

- **Gotcha:** cargo fmt can touch unrelated files and inflate commit scope.
  - **Symptom:** Formatting pass modified multiple unrelated source files.
  - **Root cause:** Repo-wide formatting changed files outside the active roadmap item.
  - **Fix:** Restored unrelated files and kept commit scope to V1-FS-002 files.
  - **Prevention:** After formatting, always run git status and explicitly restore incidental changes before committing.

## Things I Learned
- Existing capability enforcement for delete_file/os_rmdir was already mapped to filesystem-delete in src/interpreter/capabilities.rs, so V1-FS-002 delete policy work focused on behavior hardening (directory-path rejection) and regression evidence.
- Centralized helper functions for read/write limits and overwrite parsing in filesystem.rs keep async/sync paths consistent and avoid policy drift.
- The roadmap requirement to run cargo test before marking a P1 item complete can expose unrelated latent test failures; resolve or document blockers before roadmap completion updates.

## Debug Notes (Only if applicable)
- **Failing test / error:** test_scope_chain_lookup failed during cargo test with `left: 0 right: 6`.
- **Repro steps:** cargo test -q; isolated with cargo test --test interpreter_tests test_scope_chain_lookup -- --nocapture.
- **Breakpoints / logs used:** Isolated failing test source in tests/interpreter_tests.rs and reviewed run_code behavior.
- **Final diagnosis:** Test relied on side-effect assignment across nested function scopes; updated test to assert lexical lookup via return-value flow so it validates scope-chain lookup directly.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider adding explicit standard-library docs for overwrite flag parameter shape (write_file/write_binary_file examples) in docs/STANDARD_LIBRARY_REFERENCE.md if this surface is expanded further.
- [ ] If needed, add binary read/write boundary tests in native unit coverage for parity with text-path limit checks.

## Links / References
- Files touched:
  - src/interpreter/native_functions/filesystem.rs
  - src/interpreter/native_functions/mod.rs
  - src/type_checker.rs
  - tests/native_api_security_boundaries.rs
  - tests/interpreter_tests.rs
  - README.md
  - CHANGELOG.md
  - ROADMAP.md
  - docs/NATIVE_API_SECURITY_POSTURE.md
- Related docs:
  - README.md
  - ROADMAP.md
  - docs/NATIVE_API_SECURITY_POSTURE.md
