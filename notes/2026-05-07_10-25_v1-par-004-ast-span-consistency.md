# Kujo Field Notes — V1 PAR 004 Span Consistency and LSP Column Compatibility

**Date:** 2026-05-07
**Session:** 10:25 local
**Branch/Commit:** main / ea69b11
**Scope:** Implemented `V1-PAR-004` span plumbing across parser/lexer diagnostics and parser AST span publication metadata. Added regression tests and resolved an LSP compatibility regression caused by token column semantics.

---

## What I Changed
- Added shared span model in `src/errors.rs`:
  - `SourceSpan { start, end, start_byte, end_byte }`
  - byte-offset to line/column conversion helper
  - span conversion unit tests
- Extended lexer tokens in `src/lexer.rs` with `byte_offset` and routed all token emission through a single helper to keep offset wiring consistent.
- Updated parser diagnostics in `src/parser.rs`:
  - `ParseDiagnostic` now carries `span` in addition to legacy `line`/`column`
  - parser now records `ast_spans` metadata (`Statement`/`Expression`) in `ParseOutput`
  - parser diagnostic conversion to unified diagnostics uses span-backed location source
- Updated LSP token destructuring callsites for `Token` struct shape compatibility:
  - `src/lsp_definition.rs`
  - `src/lsp_references.rs`
- Added/updated regression tests:
  - `tests/parser_diagnostics_contract.rs` for diagnostic span integrity and parser AST span publication
  - lexer token-offset monotonicity test
  - span conversion tests in `src/errors.rs`

## Gotchas (Read This Next Time)
- **Gotcha:** Identifier token column semantics are an LSP contract surface.
  - **Symptom:** Full `cargo test` failed with many LSP definition/hover/references/rename/server tests returning "No identifier found" or empty symbol results.
  - **Root cause:** I initially changed identifier token columns to start-column semantics, but existing LSP tools compute identifier starts via `column - name_len` and depend on legacy end-column behavior.
  - **Fix:** Restored legacy lexer behavior for identifier-like tokens (column points just after token text), then updated parser span reconstruction to derive span-start columns correctly from legacy end-column semantics.
  - **Prevention:** When touching token location fields, run at least one LSP symbol test immediately and preserve existing `Token.column` contract unless all LSP consumers are migrated together.

## Things I Learned
- Parser span plumbing can be introduced without invasive AST enum rewrites by publishing span metadata in `ParseOutput` and keeping legacy diagnostic fields stable.
- `byte_offset` on tokens is the least risky central hook for consistent parser/LSP/tooling span reconstruction.
- Reconstructing start/end columns in parser must account for token-specific column conventions (especially identifier-like tokens).

## Debug Notes (Only if applicable)
- **Failing test / error:** `cargo test` initially failed with 14 LSP-related tests (`lsp_definition`, `lsp_hover`, `lsp_references`, `lsp_rename`, `lsp_server`).
- **Repro steps:**
  - Run `cargo test`
  - Observe symbol-location tests failing after lexer identifier column change.
- **Breakpoints / logs used:**
  - Compared old/new lexer identifier token column assignments.
  - Ran focused LSP unit tests to verify fix (`cargo test lsp_definition::tests::finds_function_definition_for_call_site --lib`, etc.).
- **Final diagnosis:** Regression came from violating legacy `Token.column` expectations, not from parser span type additions.

## Follow-ups / TODO (For Future Agents)
- [ ] Expand parser AST span publication to explicit declaration/pattern categories if roadmap follow-through needs more granularity than current statement/expression buckets.
- [ ] When runtime surfaces adopt AST-carried spans directly, migrate `Expr::location()`/`Stmt::location()` away from unknown defaults.

## Links / References
- Files touched:
  - `src/errors.rs`
  - `src/lexer.rs`
  - `src/parser.rs`
  - `src/lsp_definition.rs`
  - `src/lsp_references.rs`
  - `tests/parser_diagnostics_contract.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `ROADMAP.md`
