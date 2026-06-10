# Kujo Field Notes — V1-PAR-001 Structured Parser Diagnostics

**Date:** 2026-05-06
**Session:** 23:48 local
**Branch/Commit:** main / 8826175
**Scope:** Implemented roadmap item V1-PAR-001 by adding structured parser diagnostics, explicit delimiter expectations, statement-level recovery, and parse-diagnostic propagation through CLI/LSP/module/REPL paths.

---

## What I Changed
- Added parser diagnostics surfaces in `src/parser.rs`:
  - `ParseDiagnostic`
  - `ParseOutput`
  - `Parser::parse_with_diagnostics()`
  - statement-level synchronization and explicit `expect_*` helpers.
- Hardened parser delimiter/keyword checks in core statement/expression paths (functions, loops, conditionals, calls/indexing, array/dict literals, type annotations).
- Added assignment-target validation and parse diagnostics for invalid assignment targets.
- Preserved legacy assignment/declaration compatibility by accepting both `:=` and `=` in parser assignment operator handling.
- Added explicit `else if` support in `parse_if` so parser hardening did not regress existing conditional syntax usage.
- Wired parser diagnostics through user-facing execution paths:
  - `src/main.rs` (`run`, `test-run`, `profile`) now exits non-zero on parser diagnostics.
  - `src/module.rs` reports parser failures as module-load errors.
  - `src/repl.rs` surfaces parser diagnostics directly.
  - `src/lsp_diagnostics.rs` now consumes parser diagnostics directly instead of parser panic probing.
  - `src/lsp_completion.rs` uses parse output that can preserve partial symbol extraction.
- Added regression suite `tests/parser_diagnostics_contract.rs` for success, delimiter failures, invalid assignment target, EOF behavior, multi-error recovery, and CLI non-zero exits.
- Updated `CHANGELOG.md`, `README.md`, and `ROADMAP.md` for V1-PAR-001 completion status and behavior changes.

## Gotchas (Read This Next Time)
- **Gotcha:** Tight parser expectation checks can silently regress tolerated compatibility forms.
  - **Symptom:** Full `cargo test` initially failed in async VM tests with `Null` outputs, and one interpreter test failed in an `else if` conditional.
  - **Root cause:** Parser hardening initially required strict `:=` in declarations and strict `else { ... }` blocks, while existing tests and behavior relied on accepted `=` declarations and `else if` chains.
  - **Fix:** Added centralized assignment-operator compatibility (`:=` and `=`) and explicit `else if` parsing support in `parse_if`.
  - **Prevention:** When replacing permissive `advance()` parsing with explicit `expect_*` checks, run compatibility-heavy suites early and audit for previously tolerated syntax that should remain supported.

## Things I Learned
- Parser hardening needs a split mindset: strict diagnostics for malformed input plus explicit compatibility for accepted legacy syntax.
- `parse_with_diagnostics()` is a safer migration seam than an abrupt parse-signature rewrite across the whole codebase.
- Statement synchronization should treat identifier/literal-led expressions as restart points or multi-error recovery will collapse into single-error reporting.

## Debug Notes (Only if applicable)
- **Failing test / error:**
  - VM async tests returned `Null` (e.g., `vm::tests::test_async_await_basic` expected `Int(42)`), and `test_type_introspection_in_conditional` failed.
- **Repro steps:**
  - `cargo test`
- **Breakpoints / logs used:**
  - Reviewed failing test fixtures in `src/vm.rs` and `tests/interpreter_tests.rs`.
  - Verified parser behavior through targeted test reruns (`cargo test vm::tests::test_async_ --lib`, `cargo test --test interpreter_tests test_type_introspection_in_conditional`).
- **Final diagnosis:**
  - Strict parser changes unintentionally removed compatibility for `=` declarations and `else if` chains; restoring both resolved regressions while preserving new diagnostics behavior.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider migrating additional parser call sites to `parse_with_diagnostics()` where partial-AST fallback should be explicit and documented.
- [ ] During `V1-ERR-001`, consolidate parse diagnostics into the shared diagnostic code model without losing current line/column behavior.

## Links / References
- Files touched:
  - `src/parser.rs`
  - `src/main.rs`
  - `src/lsp_diagnostics.rs`
  - `src/module.rs`
  - `src/repl.rs`
  - `src/lsp_completion.rs`
  - `tests/parser_diagnostics_contract.rs`
  - `CHANGELOG.md`
  - `README.md`
  - `ROADMAP.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `notes/GOTCHAS.md`
