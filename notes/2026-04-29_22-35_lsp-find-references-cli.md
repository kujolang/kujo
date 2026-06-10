# Kujo Field Notes — v0.12 LSP Find-References CLI Slice

**Date:** 2026-04-29
**Session:** 22:35 local
**Branch/Commit:** main / 8659810
**Scope:** Implemented the next highest-priority incomplete v0.12.0 LSP roadmap item (find references) with a new CLI surface, scope-aware resolver, tests, and release-doc updates.

---

## What I Changed
- Added `src/lsp_references.rs` with:
  - cursor-symbol reference lookup
  - lexical scope-path tracking using token-level `{` / `}` transitions
  - declaration extraction for `func`, `let`, `const`, `for`, `except`, and function parameters
  - reference resolution that prefers nearest visible prior declarations with deterministic fallback
  - optional declaration inclusion/exclusion in results
- Added CLI command in `src/main.rs`:
  - `kujo lsp-references <file> --line <N> --column <N> [--include-definition <true|false>] [--json]`
  - plain output: `<definition|reference>\t<file>:<line>:<column>`
  - JSON output: list of `{ line, column, is_definition }`
- Added module declarations in both crate roots:
  - `src/main.rs` (`mod lsp_references;`)
  - `src/lib.rs` (`pub mod lsp_references;`)
- Added focused tests in `src/lsp_references.rs` for:
  - full function reference collection
  - declaration-exclusion behavior
  - shadowed variable scope isolation
  - non-identifier cursor behavior

## Gotchas (Read This Next Time)
- **Gotcha:** Reusing go-to-definition output directly for references can leak shadowed symbols across scopes.
  - **Symptom:** Inner `value` references incorrectly included outer-scope uses.
  - **Root cause:** Prior resolver was order-based and did not model lexical visibility.
  - **Fix:** Introduced scope-path visibility checks and declaration ranking by `(scope_depth, token_index)`.
  - **Prevention:** Keep explicit shadowing tests for both inner and outer usages whenever reference logic changes.

- **Gotcha:** Function parameters need function-body scope, not declaration-site scope.
  - **Symptom:** Parameter references can resolve outside function body if scoped at declaration site.
  - **Root cause:** Parameter tokens are parsed before `{`, while true visibility starts in body scope.
  - **Fix:** Capture body scope from scope-after-token at `{` and assign that scope path to parameter declarations.
  - **Prevention:** Treat parameter declaration scope as a derived scope from function-body entry, not from raw token position.

## Things I Learned
- Token-level scope modeling is enough for a practical first-cut find-references implementation without full parser span metadata.
- Deterministic reference results require two separate policies: nearest visible prior declaration for normal resolution, earliest visible fallback for forward references.
- CLI feature slices remain easier to stabilize when output contracts are intentionally minimal and test-covered.

## Debug Notes (Only if applicable)
- **Failing test / error:** `keeps_shadowed_variables_scoped_to_selected_definition` initially included an out-of-scope reference at line 6.
- **Repro steps:** `cargo test lsp_references -- --nocapture`.
- **Breakpoints / logs used:** assertion diff + inspection of declaration matching strategy.
- **Final diagnosis:** direct reuse of go-to-definition for each occurrence ignored lexical visibility constraints.

## Follow-ups / TODO (For Future Agents)
- [ ] Expand declaration extraction to destructuring declarations for stronger parity with completion symbol discovery.
- [ ] Consider reusing shared declaration/scope infrastructure across `lsp_definition` and `lsp_references` to avoid drift.
- [ ] Add reference contract tests for nested function parameters and `except` bindings in mixed-scope files.

## Links / References
- Files touched:
  - `src/lsp_references.rs`
  - `src/main.rs`
  - `src/lib.rs`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `README.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `notes/2026-04-29_22-28_lsp-go-to-definition-cli.md`
