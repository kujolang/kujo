# Kujo Field Notes — DocGen Workstreams A/B Followthrough

**Date:** 2026-05-18
**Session:** 13:10 local
**Branch/Commit:** main / 72a617a
**Scope:** Completed five consecutive DocGen roadmap tasks (A-3, A-4, A-1, B-1, B-2) with code, tests, strict external validation runs, and roadmap/changelog updates.

---

## What I Changed
- Hardened Kujo adapter visibility in `src/docgen/adapters/kujo.rs` so method/variant visibility respects parent container visibility.
- Added/expanded DocGen visibility and async extraction regressions in `tests/docgen_universal.rs`.
- Added fixture-driven DocGen extraction edge-case fixtures under `tests/fixtures/docgen/` and fixture-backed tests.
- Documented Kujo DocGen visibility policy in `docs/DOCGEN.md`.
- Updated roadmap completion state in `docs/DOCGEN_FEATURE_COMPLETION_ROADMAP.md` for A-1/A-3/A-4/B-1/B-2.
- Updated strict external evaluation deltas in `docs/DOCGEN_EXTERNAL_REPOS_EVALUATION_2026-05-18.md` for each completed task.
- Updated `CHANGELOG.md` and targeted `README.md` references for task-relevant behavior.

## Gotchas (Read This Next Time)
- **Gotcha:** `pub` methods on private structs were still surfacing as `Public` in strict-gate paths before container-aware visibility.
  - **Symptom:** `public_only` strict runs flagged undocumented symbols that were not exported API.
  - **Root cause:** Method visibility was computed from method token only, without parent struct visibility.
  - **Fix:** Compute effective method visibility as `method is pub` AND `parent struct is public`.
  - **Prevention:** For member symbols, never use declaration token visibility alone; always evaluate parent/export reachability.
- **Gotcha:** Kujo adapter initially missed `async func` declarations entirely.
  - **Symptom:** `docgen` `item_count` was `0` for files containing only `async func` declarations.
  - **Root cause:** Function regex only matched `func` without optional `async` prefix.
  - **Fix:** Extended Kujo function regex to accept optional `async` and kept existing visibility semantics.
  - **Prevention:** Add fixture-backed regression files for each supported declaration modifier surface.
- **Gotcha:** Full `cargo test` can occasionally fail on serve integration readiness races.
  - **Symptom:** `serve_large_file_head_returns_content_length_without_body` intermittently reported `Connection refused`.
  - **Root cause:** Subprocess readiness race in socket-bound integration tests under load.
  - **Fix:** Immediate rerun passed; then full-suite rerun passed green.
  - **Prevention:** Keep known flaky serve test behavior documented and rerun full suite when this specific transient appears.

## Things I Learned
- DocGen strict-gate signal quality depends more on effective visibility classification than raw extraction count.
- Fixture-driven edge tests are the fastest way to lock regex-adapter behavior and avoid regression when adding syntax modifiers.
- External strict validation across real repos remained stable (`undocumented=0`, `broken_links=0`, `warnings=0`) after all five loops, so improvements were low-risk to existing repo baselines.

## Debug Notes (Only if applicable)
- **Failing test / error:** `serve_large_file_head_returns_content_length_without_body` intermittent `Connection refused` during full `cargo test`.
- **Repro steps:** `cargo test` full suite under active concurrent workload.
- **Breakpoints / logs used:** Test stdout/stderr and immediate targeted rerun of the failing integration test.
- **Final diagnosis:** Transient readiness race; deterministic code change not required for the completed DocGen tasks.

## Follow-ups / TODO (For Future Agents)
- [ ] Complete Workstream B task 3: parser-backed extraction path (or hybrid) evaluation + documented decision.
- [ ] Continue Workstreams C-F in order with one-task-per-loop execution.
- [ ] Consider adding a dedicated fixture for mixed modifiers once Kujo adds new declaration forms beyond `async`.

## Links / References
- Files touched:
  - `src/docgen/adapters/kujo.rs`
  - `tests/docgen_universal.rs`
  - `tests/fixtures/docgen/kujo_async_visibility.kujo`
  - `tests/fixtures/docgen/kujo_async_visibility.expected.json`
  - `tests/fixtures/docgen/kujo_async_strict_public.kujo`
  - `tests/fixtures/docgen/kujo_async_strict_public.expected.json`
  - `docs/DOCGEN.md`
  - `docs/DOCGEN_FEATURE_COMPLETION_ROADMAP.md`
  - `docs/DOCGEN_EXTERNAL_REPOS_EVALUATION_2026-05-18.md`
  - `CHANGELOG.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `docs/DOCGEN.md`
  - `docs/DOCGEN_FEATURE_COMPLETION_ROADMAP.md`
