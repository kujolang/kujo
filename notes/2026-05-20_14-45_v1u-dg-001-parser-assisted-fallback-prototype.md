# V1U-DG-001: Parser-Assisted Kujo Extraction Fallback Prototype

Date: 2026-05-20
Checklist item: `V1U-DG-001`

## Summary

Implemented an opt-in Kujo DocGen parser-assisted extraction prototype while preserving deterministic regex fallback behavior on lexer/parser diagnostics.

## Changes

1. Added opt-in extraction path wiring:
   - `src/main.rs`: new `kujo docgen --kujo-parser-assisted` flag.
   - `src/docgen/core.rs`: new `DocgenExtractionOptions` and `run_with_link_validation_and_options(...)` entrypoint.
2. Added parser-assisted Kujo extraction:
   - `src/docgen/adapters/kujo.rs`: `extract_symbols_with_parser_fallback(...)` with explicit strategy outcomes:
     - `ParserAssisted`
     - `RegexFallbackLexerDiagnostics`
     - `RegexFallbackParserDiagnostics`
3. Added fixture-backed coverage:
   - `tests/fixtures/docgen/kujo_parser_assisted_success.kujo`
   - `tests/fixtures/docgen/kujo_parser_assisted_success.expected.json`
   - `tests/fixtures/docgen/kujo_parser_assisted_fallback.kujo`
   - `tests/fixtures/docgen/kujo_parser_assisted_fallback.expected.json`
   - `tests/docgen_universal.rs` parser-success and parser-fallback contracts.

## Validation

Commands run:

1. `cargo test --test docgen_universal docgen_kujo_parser_assisted_fixture -- --nocapture`
2. `cargo test --test docgen_universal docgen_external_validation_blocks_redirects_to_non_allowlisted_hosts -- --nocapture`
3. `cargo test --test docgen_universal -- --nocapture`

Result: PASS. Full `docgen_universal` suite passed with parser-assisted fixture tests included.
