# v0.13.0 LSP Parity Follow-Through: Formatting + Symbol Endpoints

Date: 2026-04-30
Track: v0.13.0 Cross-IDE Foundation
Checklist item: LSP Feature Parity

## Implemented

Added Kujo LSP server handlers for:

- `textDocument/formatting`
- `textDocument/rangeFormatting`
- `textDocument/documentSymbol`
- `workspace/symbol`

Implementation notes:

- Formatting handlers reuse `formatter::format_source(...)` with LSP-provided tab-size option mapping.
- Symbol handlers derive function/variable symbol data from shared lexer tokens; no editor-specific analysis pipeline was introduced.

## Verification

Commands:

- `cargo test lsp_server`
- `cargo test`

Results:

- PASS for focused LSP server tests (`7 passed` in lib target and `7 passed` in main target).
- PASS for full suite:
  - `454 passed; 0 failed; 7 ignored` (lib target)
  - `238 passed; 0 failed` (integration target)

## Remaining Parity Work

LSP parity checklist still requires explicit protocol fixture/error-case coverage and output versioning policy hardening for release gate sign-off.
