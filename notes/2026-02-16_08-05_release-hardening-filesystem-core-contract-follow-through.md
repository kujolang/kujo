# Kujo Field Notes — Release hardening filesystem core contracts

**Date:** 2026-02-16
**Session:** 08:05 local
**Branch/Commit:** main / b587034, b8e6551
**Scope:** Implemented the next highest-priority incomplete roadmap slice under v0.10.0 P1 Release Hardening by adding comprehensive contract coverage for core filesystem builtins. Synchronized release docs and validated the full test suite.

---

## What I Changed
- Added release-hardening critical dispatcher coverage entries in `src/interpreter/native_functions/mod.rs` for:
  - `read_file`, `write_file`, `append_file`, `file_exists`, `read_lines`, `list_dir`, `create_dir`
  - `file_size`, `delete_file`, `rename_file`, `copy_file`
  - `read_binary_file`, `write_binary_file`
- Added `test_release_hardening_filesystem_core_contracts` in `src/interpreter/native_functions/mod.rs` with:
  - argument-shape/error-shape checks for all APIs above
  - end-to-end text file lifecycle checks (`write_file` → `read_file` → `append_file` → `read_lines`)
  - directory/listing behavior checks (`create_dir`, `list_dir`)
  - file mutation/cleanup checks (`rename_file`, `copy_file`, `delete_file`, `file_exists`)
  - binary round-trip checks (`write_binary_file`, `read_binary_file`)
- Updated release docs:
  - `CHANGELOG.md` (new Unreleased P1 hardening entry)
  - `ROADMAP.md` (completed milestone bullet)
  - `README.md` (project status hardening bullet)
- Validation executed:
  - `cargo test test_release_hardening_filesystem_core_contracts -- --nocapture`
  - `cargo test release_hardening_builtin_dispatch_coverage -- --nocapture`
  - `cargo test`

## Gotchas (Read This Next Time)
- **Gotcha:** `read_file` currently emits a `read_file_sync`-named error string on bad shape.
  - **Symptom:** `read_file()` with missing/non-string argument returns error text containing `read_file_sync requires a string path argument`.
  - **Root cause:** `read_file` and `read_file_sync` share one match arm in `src/interpreter/native_functions/filesystem.rs`, and the branch returns a fixed `read_file_sync` message.
  - **Fix:** Contract test now asserts the actual current runtime message to avoid false negatives.
  - **Prevention:** When adding contract tests for alias/shared-handler APIs, verify exact emitted error text from implementation rather than assuming canonicalized naming.

- **Gotcha:** Broad test-list edits can silently produce compile breaks if patch context anchors the wrong spot.
  - **Symptom:** Compile error: `expected one of '.', ';', '?', '}', or an operator, found ','` in `src/interpreter/native_functions/mod.rs`.
  - **Root cause:** A string-literal list fragment was inserted outside the `critical_builtin_names` array due incorrect patch anchor.
  - **Fix:** Removed the stray literals from `test_unknown_native_function_returns_explicit_error` and re-applied insertion at the array block.
  - **Prevention:** After editing large static arrays, run a targeted compile/test immediately and inspect nearby function boundaries before proceeding.

## Things I Learned
- Release-hardening in this repo is an iterative contract-expansion workflow: extend critical coverage list + add behavior/shape tests + sync docs in the same session.
- `critical_builtin_names` is an explicit drift guard and remains intentionally verbose/manual for readability during regression triage.
- For this workflow, full-suite validation (`cargo test`) is still required even when targeted hardening tests pass.

## Debug Notes (Only if applicable)
- **Failing test / error:** `error: expected one of '.', ';', '?', '}', or an operator, found ','` at `src/interpreter/native_functions/mod.rs:121:24`.
- **Repro steps:**
  - Inserted additional filesystem builtin strings in wrong location during patching.
  - Ran `cargo test test_release_hardening_filesystem_core_contracts -- --nocapture`.
- **Breakpoints / logs used:**
  - Inspected the affected range in `src/interpreter/native_functions/mod.rs` around `test_unknown_native_function_returns_explicit_error` and `critical_builtin_names`.
- **Final diagnosis:**
  - Misplaced string-literal fragment outside array initializer caused parser-level Rust compile failure.

## Follow-ups / TODO (For Future Agents)
- [ ] Normalize `read_file` bad-shape error text to mention `read_file` (or alias-aware wording) for clearer user-facing consistency.
- [ ] Consider extracting `critical_builtin_names` assembly into a helper to reduce manual list-edit risk while preserving explicit hardening coverage intent.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
