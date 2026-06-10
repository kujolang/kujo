# Kujo Field Notes — Release Hardening Modular Dispatch Gap Closures

**Date:** 2026-02-15
**Session:** 17:13 local
**Branch/Commit:** main / f977495
**Scope:** Implemented iterative v0.10 release-hardening modular dispatch closures for remaining high-impact declared builtins (`contains`/`index_of`, `io_*`, HTTP APIs, and `db_*`), expanded dispatcher hardening tests, and synchronized roadmap/changelog/readme updates with incremental commits.

---

## What I Changed
- Closed string polymorphic dispatch gaps for `contains` and `index_of` in `src/interpreter/native_functions/strings.rs`.
- Migrated advanced IO APIs (`io_read_bytes`, `io_write_bytes`, `io_append_bytes`, `io_read_at`, `io_write_at`, `io_seek_read`, `io_file_metadata`, `io_truncate`, `io_copy_range`) into `src/interpreter/native_functions/io.rs`.
- Migrated declared HTTP request/response/server APIs into `src/interpreter/native_functions/http.rs`.
- Migrated declared database APIs (`db_connect`, `db_execute`, `db_query`, pool APIs, transaction APIs, `db_last_insert_id`) into `src/interpreter/native_functions/database.rs`.
- Expanded release-hardening dispatcher contracts in `src/interpreter/native_functions/mod.rs`:
  - added migrated APIs to critical anti-fallback coverage,
  - removed migrated APIs from expected known legacy dispatch gaps,
  - added argument-shape/error-shape contract tests per migrated module.
- Added comprehensive native-function tests for each migrated module (string, IO, HTTP, database).
- Updated docs after each major implementation slice:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Used incremental commits and pushes per major step (feature/test commit + docs commit for each slice).

## Gotchas (Read This Next Time)
- **Gotcha:** Dispatch migration is not complete until the hardening drift ledger is updated in the same change set.
  - **Symptom:** `test_release_hardening_builtin_dispatch_coverage_for_declared_builtins` fails even when the handler implementation works.
  - **Root cause:** The release-hardening test intentionally tracks known unmigrated builtins in `expected_known_legacy_dispatch_gaps`; migrated APIs must be removed from that list and usually added to recent critical coverage.
  - **Fix:** Update both hardening lists in `src/interpreter/native_functions/mod.rs` in the same commit as handler migration.
  - **Prevention:** Treat migration as a 3-part atomic unit: implement handler(s), add module tests, update dispatch-ledger tests.

- **Gotcha:** `cargo fmt` can touch unrelated modified files in an already-dirty tree.
  - **Symptom:** `git status --short` showed unrelated changes in `async_ops.rs`, `json.rs`, and `system.rs` after validation.
  - **Root cause:** Workspace already had unstaged edits before running formatter/validation.
  - **Fix:** Stage only target files for each milestone commit (e.g., `git add src/interpreter/native_functions/database.rs src/interpreter/native_functions/mod.rs`).
  - **Prevention:** Always verify staged scope with `git status --short` before commit and keep atomic commits file-scoped when the workspace is dirty.

- **Gotcha:** SQLite tests are the safest cross-environment baseline for database modular dispatch validation.
  - **Symptom:** Postgres/MySQL paths can be environment-dependent and flaky without external services.
  - **Root cause:** DB integrations for Postgres/MySQL require external connectivity/credentials not guaranteed in CI or local sandbox.
  - **Fix:** Add comprehensive SQLite-backed tests for core dispatch and contract behavior; preserve implementation parity for other DB backends.
  - **Prevention:** Keep backend-generic logic in handlers, but anchor regression tests on deterministic SQLite workflows.

## Things I Learned
- The release-hardening tests in `src/interpreter/native_functions/mod.rs` are a migration ledger, not just smoke tests; they encode roadmap progress state.
- For Kujo modular native migration work, “done” requires both runtime behavior and dispatch-contract metadata consistency.
- For DB module migration, helper extraction (`map_sqlite_value`, `map_mysql_value`, `to_mysql_value`, runtime builder) reduces handler duplication and keeps contracts readable.
- Committing feature/tests and docs separately keeps hardening progress auditable and matches the project’s incremental commit requirement.

## Follow-ups / TODO (For Future Agents)
- [ ] Migrate next remaining known dispatch gaps from the hardening list (`Set`, image, compression/crypto, process, network APIs).
- [ ] Add focused release-hardening contract tests for each new migrated cluster (argument-shape + non-fallback coverage).
- [ ] Keep `CHANGELOG.md`, `ROADMAP.md`, and `README.md` synchronized per migrated cluster commit cycle.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/strings.rs`
  - `src/interpreter/native_functions/io.rs`
  - `src/interpreter/native_functions/http.rs`
  - `src/interpreter/native_functions/database.rs`
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `README.md`
