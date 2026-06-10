# Kujo Field Notes — DocGen A-2 Kujo Visibility Classification

**Date:** 2026-05-18
**Session:** 12:32 local
**Branch/Commit:** main / ddc3714
**Scope:** Completed DocGen Workstream A task 2 by updating Kujo adapter top-level function visibility and adding regression coverage for strict/public-only gate behavior.

---

## What I Changed
- Updated `src/docgen/adapters/kujo.rs` so Kujo `func` symbols are `Public` only when explicitly declared with `pub`.
- Added visibility/gate regressions in `tests/docgen_universal.rs` for:
  - top-level non-`pub` helper visibility
  - explicit `pub` top-level function visibility
  - struct method visibility (`func` private, `pub func` public)
  - strict gate behavior with private undocumented helpers vs explicit undocumented public symbols.

## Gotchas (Read This Next Time)
- **Gotcha:** Existing strict-gate test assumptions depended on top-level Kujo functions being auto-public.
  - **Symptom:** `docgen_strict_gates_fail_as_expected` started passing strict checks unexpectedly after the visibility fix.
  - **Root cause:** The fixture used non-`pub` functions, which are now private and skipped by gap counting.
  - **Fix:** Updated strict-gate fixture functions to `pub func` so the test still validates gate-failure behavior.
  - **Prevention:** Any visibility-policy change should audit strict-gate fixtures for explicit `pub` usage.

## Things I Learned
- Gap generation (`src/docgen/gaps.rs`) already skips private symbols, so changing symbol visibility directly changes strict undocumented counts without touching gate logic.
- The strict-gate failure message still says "undocumented public symbols", so visibility correctness is critical for signal quality.

## Debug Notes (Only if applicable)
- **Failing test / error:** `docgen_kujo_visibility_tracks_top_level_functions_and_struct_methods` expected `internal_helper` visibility `Private` but got `Public`.
- **Repro steps:** `cargo test --test docgen_universal docgen_kujo_visibility_tracks_top_level_functions_and_struct_methods -- --nocapture`
- **Breakpoints / logs used:** N/A (assertion failure was direct and deterministic).
- **Final diagnosis:** `extract_symbols` used `caps.get(1).is_some() || !is_method`, forcing all top-level functions public.

## Follow-ups / TODO (For Future Agents)
- [ ] Complete Workstream A task 3 and ensure internal/private helpers remain non-public across additional Kujo patterns.
- [ ] Keep external-repo strict delta checks updated as visibility/extraction behavior evolves.

## Links / References
- Files touched:
  - `src/docgen/adapters/kujo.rs`
  - `tests/docgen_universal.rs`
  - `docs/DOCGEN_FEATURE_COMPLETION_ROADMAP.md`
  - `docs/DOCGEN_EXTERNAL_REPOS_EVALUATION_2026-05-18.md`
  - `CHANGELOG.md`
  - `README.md`
- Related docs:
  - `docs/DOCGEN.md`
  - `docs/DOCGEN_FEATURE_COMPLETION_ROADMAP.md`
  - `docs/DOCGEN_EXTERNAL_REPOS_EVALUATION_2026-05-18.md`
