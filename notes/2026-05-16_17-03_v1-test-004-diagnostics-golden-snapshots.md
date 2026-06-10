# Kujo Field Notes — V1-TEST-004 diagnostics golden snapshots

**Date:** 2026-05-16
**Session:** 17:03 local
**Branch/Commit:** main / 746316e
**Scope:** Implemented diagnostics golden snapshot coverage for lexer/parser/semantic/runtime/CLI/serve diagnostics with cross-platform newline normalization and explicit snapshot update workflow.

---

## What I Changed
- Added `tests/diagnostics_golden.rs` golden test harness.
- Added fixture inputs in `tests/fixtures/diagnostics/`:
  - `lexer_invalid_escape.kujo`
  - `parser_missing_paren.kujo`
  - `semantic_invalid_assignment.kujo`
  - `runtime_undefined_identifier.kujo`
- Added paired snapshot files (`*.human.golden`, `*.json.golden`) for:
  - lexer invalid escape diagnostics
  - parser missing delimiter diagnostics
  - semantic invalid assignment-target parser diagnostics
  - runtime undefined-identifier diagnostics
  - CLI invalid invocation diagnostics
  - serve invalid header-limit diagnostics
- Added `KUJO_UPDATE_GOLDENS=1` update mode in the harness to regenerate snapshots intentionally.
- Added CRLF normalization helper in the harness and a direct regression test for line-ending normalization behavior.
- Updated `README.md` testing section with golden test and update commands.
- Updated `ROADMAP.md` and `CHANGELOG.md` for `V1-TEST-004` completion.

## Gotchas (Read This Next Time)
- **Gotcha:** Snapshot updates should be explicit, not automatic.
  - **Symptom:** Golden tests fail after intentional diagnostic text changes.
  - **Root cause:** Snapshot files lock exact rendered output.
  - **Fix:** Re-run with `KUJO_UPDATE_GOLDENS=1 cargo test --test diagnostics_golden`.
  - **Prevention:** Keep update mode env-gated so accidental text drift does not silently overwrite baselines.

- **Gotcha:** File-path stability matters for diagnostics snapshots.
  - **Symptom:** Snapshots drift if diagnostics embed temp/absolute paths.
  - **Root cause:** Path values vary per machine and test run.
  - **Fix:** Build fixture diagnostics using stable fixture filenames, not temp absolute paths.
  - **Prevention:** Keep fixture-root inputs and normalized rendering in snapshot harnesses.

## Things I Learned
- Existing diagnostics unit tests validated shape/contracts, but a separate fixture-backed golden suite was still needed to lock full rendered text output.
- Parser semantic misuse cases (invalid assignment target) are best treated as parser diagnostics snapshots rather than runtime snapshots.
- CRLF normalization in the harness is enough to keep snapshots stable across Linux/macOS/Windows line-ending differences.

## Debug Notes (Only if applicable)
- **Failing test / error:** Initial golden files were absent by design.
- **Repro steps:** Run `cargo test --test diagnostics_golden` before generating snapshots.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** Harness needed deterministic update mode and fixtures first; once generated, locked snapshots pass deterministically.

## Follow-ups / TODO (For Future Agents)
- [ ] If CLI adds machine-readable diagnostic mode for `run`, capture end-to-end runtime JSON diagnostics in this golden suite.
- [ ] Keep snapshot categories aligned with new diagnostic subsystems/codes added in future roadmap items.

## Links / References
- Files touched:
  - `tests/diagnostics_golden.rs`
  - `tests/fixtures/diagnostics/*`
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
- Related docs:
  - `notes/README.md`
  - `ROADMAP.md`
  - `README.md`
