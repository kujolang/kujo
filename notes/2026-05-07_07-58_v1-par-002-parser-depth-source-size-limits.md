# Kujo Field Notes — V1-PAR-002 Parser Depth And Source Size Limits

**Date:** 2026-05-07
**Session:** 07:58 local
**Branch/Commit:** main / (in progress)
**Scope:** Implemented roadmap item V1-PAR-002 by adding parser nesting-depth safeguards and CLI source-size guards, with regression coverage and spec/docs updates.

---

## What I Changed
- Added parser safety constants in `src/parser.rs`:
  - `DEFAULT_MAX_SOURCE_BYTES = 1_048_576`
  - `DEFAULT_MAX_EXPRESSION_DEPTH = 256`
  - `DEFAULT_MAX_BLOCK_DEPTH = 128`
- Added parser limit plumbing in `src/parser.rs`:
  - `ParserLimits` and `Parser::new_with_limits(...)`
  - centralized `with_expression_depth(...)` and `with_block_depth(...)` guards
  - centralized `parse_statement_block(...)` helper reused by function/test/control-flow block parsing
  - parse diagnostics for depth-overflow paths
- Added source-size diagnostic helpers in `src/parser.rs`:
  - `source_size_limit_diagnostic(...)`
  - `validate_source_size(...)`
- Added CLI pre-parse source-size enforcement in `src/main.rs`:
  - `read_kujo_source_for_parse(...)` with metadata pre-check and post-read validation
  - wired into parse entrypoints (`run`, `test-run`, `profile`, and `lsp-*` helper commands)
- Added regression tests in `tests/parser_diagnostics_contract.rs`:
  - deep parenthesized expression depth failure
  - deep nested array literal depth failure
  - deep nested `if` block depth failure
  - source-size over-limit failure in `kujo run`
  - source-size boundary success in `kujo run`
- Updated docs and status tracking:
  - `CHANGELOG.md`
  - `README.md`
  - `docs/LANGUAGE_SPEC.md`
  - `ROADMAP.md` (`V1-PAR-002` marked complete with verification notes)

## Gotchas (Read This Next Time)
- **Gotcha:** Depth checks are easy to implement inconsistently if each block parser duplicates brace-loop logic.
  - **Symptom:** Some nested constructs would enforce limits while others silently bypass them.
  - **Root cause:** Many parsing methods had hand-rolled `{ ... }` statement-body loops.
  - **Fix:** Introduced one shared `parse_statement_block(...)` helper and reused it across function/if/loop/test/match/try parsing paths.
  - **Prevention:** When adding new statement-level blocks, route through `parse_statement_block(...)` so block-depth policy and diagnostics stay uniform.

## Things I Learned
- Parser safety limits are easiest to keep consistent when depth accounting wraps semantic recursion points (`parse_expr`, unary recursion, array/dict recursion) rather than every precedence function.
- For CLI source-size protection, a metadata pre-check plus post-read byte-length validation gives deterministic behavior even when file metadata is stale or unavailable.
- `Parser::new_with_limits(...)` makes it practical to write deterministic low-limit regression tests without relying on very deep generated fixtures.

## Debug Notes (Only if applicable)
- **Failing test / error:** New tests required parser limit configuration support not available in baseline parser API.
- **Repro steps:** `cargo test --test parser_diagnostics_contract`
- **Breakpoints / logs used:** Direct compile/test iteration and targeted parser contract assertions.
- **Final diagnosis:** Parser needed explicit limits API (`ParserLimits` + `new_with_limits`) and shared depth guards before depth-limit tests could be asserted precisely.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider whether REPL/module-only parse entrypoints should also apply source-size guard policy explicitly (currently this item scoped CLI command entrypoints).
- [ ] If source-size/depth limits ever become user-configurable, keep unsafe values explicit opt-in and document precedence rules.

## Links / References
- Files touched:
  - `src/parser.rs`
  - `src/main.rs`
  - `tests/parser_diagnostics_contract.rs`
  - `CHANGELOG.md`
  - `README.md`
  - `docs/LANGUAGE_SPEC.md`
  - `ROADMAP.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `ROADMAP.md`
