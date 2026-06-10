# Kujo Field Notes — v0.12 Formatter Initial CLI Slice

**Date:** 2026-04-29
**Session:** 22:53 local
**Branch/Commit:** main / c56164a
**Scope:** Implemented the next highest-priority incomplete v0.12.0 track (Formatter) with an initial formatter engine, CLI command, tests, and release evidence updates.

---

## What I Changed
- Added src/formatter.rs with:
  - opinionated spacing normalization for common operators and commas
  - configurable indentation width
  - line-length-aware wrapping for comma-separated expressions
  - leading import-block sorting with optional disable flag
- Added CLI command in src/main.rs:
  - kujo format <file> [--indent <N>] [--line-length <N>] [--no-sort-imports] [--check] [--write]
  - supports check mode for CI and write mode for direct formatting
- Added module declarations in both crate roots:
  - src/main.rs (mod formatter;)
  - src/lib.rs (pub mod formatter;)
- Added focused tests in src/formatter.rs for:
  - spacing/indentation normalization
  - leading import block ordering
  - line-length wrapping behavior

## Gotchas (Read This Next Time)
- **Gotcha:** Generic operator normalization can accidentally split multi-character operators.
  - **Symptom:** `:=` became `: =` in formatted output.
  - **Root cause:** Single-character `=` normalization ran after `:=` normalization.
  - **Fix:** Removed standalone `=` normalization from operator pass to preserve `:=` token contract.
  - **Prevention:** Multi-character operators should be normalized before any overlapping single-character operator logic, or protected from overlap.

## Things I Learned
- A practical first formatter slice can be delivered without full AST formatting by constraining transformations to deterministic text-level rules.
- Check/write dual modes are important to support both CI guardrails and local developer workflows.
- Import sorting should be explicitly bounded (leading block only) until module semantics and broader formatting guarantees are stronger.

## Debug Notes (Only if applicable)
- **Failing test / error:** formatter_normalizes_spacing_and_indentation failed due `:=` token breakage.
- **Repro steps:** cargo test formatter -- --nocapture.
- **Breakpoints / logs used:** test assertion output + formatter smoke output.
- **Final diagnosis:** overlapping operator-normalization rules transformed `:=` incorrectly.

## Follow-ups / TODO (For Future Agents)
- [ ] Move from regex-only formatting to AST-aware formatting where feasible for safer structural formatting.
- [ ] Expand line-wrapping heuristics beyond comma-separated expressions.
- [ ] Add idempotence checks (`format(format(source)) == format(source)`) for stronger formatter contract coverage.

## Links / References
- Files touched:
  - src/formatter.rs
  - src/main.rs
  - src/lib.rs
  - ROADMAP.md
  - CHANGELOG.md
  - README.md
- Related docs:
  - notes/FIELD_NOTES_SYSTEM.md
  - notes/GOTCHAS.md
  - notes/2026-04-29_22-49_lsp-code-actions-cli.md
