# Kujo Field Notes — HTTP Query Parsing and Showcase Hardening

**Date:** 2026-05-05
**Session:** 11:17 local
**Branch/Commit:** main / fe97d0b
**Scope:** I hardened Kujo HTTP routing and request metadata so query strings no longer break route matching, then upgraded the CRUD showcase with production-focused security/performance defaults and validated runtime behavior.

---

## What I Changed
- Updated Kujo runtime HTTP request handling in `src/interpreter/mod.rs` and `src/vm.rs`.
- Added URL splitting before route matching so handlers match on path-only URLs.
- Added request metadata fields for HTTP handlers:
  - `req["query"]` (dict)
  - `req["query_string"]` (raw query string)
  - `req["raw_path"]` (original URL path including query)
- Upgraded showcase app in `../crud-api/main.kujo` with:
  - optional write-route Bearer auth
  - CORS origin config
  - strict security response headers
  - JSON body size limit
  - SQLite pragmas + indexes
  - paginated/filterable `GET /items`
  - safer DB execute/query wrappers
- Updated `../crud-api/README.md` with production-focused setup and API behavior.

## Gotchas (Read This Next Time)
- **Gotcha:** HTTP route matching previously compared registered paths against full request URLs (including query strings).
  - **Symptom:** Routes like `/items` could fail to match for requests like `/items?limit=20`.
  - **Root cause:** Route selection used `request.url()` directly instead of normalized path.
  - **Fix:** Split URL into `(path, query)` first; route-match on path only and expose query map separately.
  - **Prevention:** Keep route matching and query parsing as distinct stages. Any HTTP request object changes should preserve this invariant in both interpreter and VM code paths.

- **Gotcha:** SQLite PRAGMA statements can return rows in Kujo's DB native surface.
  - **Symptom:** `db_execute(...)` returned error: `Execute returned results - did you mean to call query?`.
  - **Root cause:** Some PRAGMA operations are result-returning statements.
  - **Fix:** Run PRAGMAs with `db_query(...)` wrappers and use `db_execute(...)` only for schema/data mutations.
  - **Prevention:** Treat DB setup in two buckets: query-returning statements vs mutation statements.

## Things I Learned
- Query-aware HTTP APIs in Kujo require runtime-level request normalization, not just app-level parsing.
- VM and interpreter parity is critical for HTTP behavior changes; both request builders must stay aligned.
- Production-ish showcase defaults can be implemented in Kujo today using env-driven configuration + strict validation + security headers.
- Optional shared-token auth is a useful baseline but should be documented as a stepping stone to user/role-based auth.

## Debug Notes (Only if applicable)
- **Failing test / error:** `Runtime Error: SQLite execution error: Execute returned results - did you mean to call query?`
- **Repro steps:** Start hardened showcase with PRAGMAs run through `db_execute(...)`.
- **Breakpoints / logs used:** Runtime CLI output from `target/debug/kujo run ../crud-api/main.kujo`.
- **Final diagnosis:** PRAGMA calls needed query semantics; changing PRAGMA initialization to `db_query_safe(...)` resolved startup.

## Follow-ups / TODO (For Future Agents)
- [ ] Add URL-decoding for query keys/values in request parsing (`%20`, `+`, etc.) with parity tests.
- [ ] Add dedicated HTTP route-match tests for query-string paths in both interpreter and VM layers.
- [ ] Add automated API integration tests for the showcase repository and wire into CI.

## Links / References
- Files touched:
  - `src/interpreter/mod.rs`
  - `src/vm.rs`
  - `../crud-api/main.kujo`
  - `../crud-api/README.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `README.md`
  - `ROADMAP.md`
