# Kujo Field Notes — V1-LEX-001 Structured Lexer Diagnostics

**Date:** 2026-05-06
**Session:** 23:22 local
**Branch/Commit:** main / uncommitted
**Scope:** Implemented `V1-LEX-001` by replacing optimistic lexer behavior with structured diagnostics, wiring strict tokenization through runtime/tooling call paths, and adding regression coverage for malformed lexical input.

---

## What I Changed
- Reworked `src/lexer.rs` tokenization API to return `Result<Vec<Token>, Vec<LexerDiagnostic>>` for strict paths and added `tokenize_with_diagnostics(...)` for recovery-oriented diagnostic consumers.
- Added `LexerDiagnosticKind` + `LexerDiagnostic` metadata (line, column, byte offset, optional file path).
- Added validation and diagnostics for:
  - invalid characters
  - null bytes
  - unterminated strings
  - unterminated block comments
  - invalid escapes
  - malformed numeric literals and numeric overflow
  - identifier/string/numeric literal length limits
- Removed numeric parse fallbacks (`unwrap_or(0)` / `unwrap_or(0.0)`) in favor of diagnostics.
- Updated strict tokenization call paths in CLI/module/REPL/bench/linter/LSP helpers to surface lexical failures explicitly.
- Updated `src/lsp_diagnostics.rs` to merge lexer diagnostics into LSP diagnostic output.
- Added/updated tests in:
  - `src/lexer.rs`
  - `src/lsp_diagnostics.rs`
  - helper tokenization call sites in test modules

## Gotchas (Read This Next Time)
- **Gotcha:** LSP symbol tooling depends on legacy identifier token column semantics.
  - **Symptom:** `lsp_definition`, `lsp_hover`, `lsp_references`, `lsp_rename`, and `lsp_server` symbol tests failed after lexer refactor.
  - **Root cause:** Identifier token `column` was changed to start-column, but existing LSP helpers compute start as `column - identifier_len`.
  - **Fix:** Preserve legacy identifier `column` behavior (end/exclusive-style) while introducing structured diagnostics.
  - **Prevention:** If token location semantics are changed, update all LSP helper math and test fixtures in the same patch.

- **Gotcha:** Full `cargo test` is required before finalizing P0 roadmap completion notes.
  - **Symptom:** Focused lexer tests passed, but broader suite revealed LSP regressions.
  - **Root cause:** Token location contracts are cross-cutting and not isolated to lexer tests.
  - **Fix:** Add targeted LSP reruns after lexer refactors and still run full suite.
  - **Prevention:** Treat lexer location fields as shared contracts for parser/LSP/CLI tools.

## Things I Learned
- The best shape for this phase was strict tokenization for execution paths plus recovery-aware tokenization for diagnostic surfaces (`tokenize_with_diagnostics`).
- Even when behavior changes are lexical, token position metadata is effectively part of the editor protocol contract.
- Keeping optional file-path metadata in lexer diagnostics made module/CLI error surfacing cleaner without introducing a separate diagnostics framework yet.

## Debug Notes (Only if applicable)
- **Failing test / error:** Multiple failures in `lsp_definition`, `lsp_hover`, `lsp_references`, `lsp_rename`, and LSP server symbol tests during full `cargo test`.
- **Repro steps:**
  - `cargo test`
  - observe LSP failure set after lexer refactor.
- **Breakpoints / logs used:** Focused reruns with `cargo test <single filter> --lib` for each LSP module and call-site inspection for `identifier_start_column(...)` assumptions.
- **Final diagnosis:** Identifier token `column` contract drifted from legacy expectations; restoring compatibility fixed all regressions.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider centralizing token span semantics (`start`, `end`) so future diagnostics/LSP work does not rely on implicit per-token conventions.
- [ ] Revisit whether numeric token `column` semantics should be normalized once parser/LSP span handling is centralized under `V1-ERR-001`/`V1-PAR-004`.

## Links / References
- Files touched:
  - `src/lexer.rs`
  - `src/lsp_diagnostics.rs`
  - `src/main.rs`
  - `src/module.rs`
  - `src/repl.rs`
  - `src/linter.rs`
  - `src/lsp_completion.rs`
  - `src/lsp_definition.rs`
  - `src/lsp_hover.rs`
  - `src/lsp_references.rs`
  - `src/lsp_rename.rs`
  - `src/lsp_server.rs`
  - `src/benchmarks/runner.rs`
  - `src/vm.rs`
  - `src/parser.rs`
  - `src/interpreter/legacy_full.rs`
  - `tests/interpreter_tests.rs`
  - `tests/vm_interpreter_parity_surfaces.rs`
  - `tests/image_conversion_integration.rs`
  - `tests/optional_typing_v1_contract.rs`
  - `tests/parser_type_annotation_regressions.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `docs/LANGUAGE_SPEC.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `docs/LANGUAGE_SPEC.md`
  - `notes/GOTCHAS.md`
