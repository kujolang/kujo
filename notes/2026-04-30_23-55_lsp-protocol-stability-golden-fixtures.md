# v0.14.0 LSP Protocol Stability Evidence

Date: 2026-04-30
Context: local development machine (macOS)
Scope: ROADMAP section "2. LSP Protocol Stability Guarantees"

## Implemented

- Added multi-file golden fixture:
  - `tests/lsp_fixtures/multi_file_workspace_symbol_rename_references.json`
- Updated protocol contract docs:
  - `docs/PROTOCOL_CONTRACTS.md`
  - published supported-method compatibility table
  - documented unsupported-method JSON-RPC error behavior
- Hardened deterministic output for fixture stability:
  - `src/lsp_server.rs` now URI-sorts open documents before `workspace/symbol` aggregation

## Verification Commands

1. `cargo test --test lsp_conformance_harness`
- Result: PASS (`1 passed; 0 failed`)
- Confirms fixture-backed protocol contract expectations pass including new multi-file fixture.

2. `cargo test --test lsp_external_clients_smoke`
- Result: PASS (`2 passed; 0 failed`)

3. `cargo test --test cli_json_contracts`
- Result: PASS (`4 passed; 0 failed`)

## Acceptance Mapping

- Protocol fixtures are now expanded and remain shape-locked under `tests/lsp_fixtures/` + `tests/lsp_conformance_harness.rs`.
- Docs now include method-by-method compatibility and explicit unsupported request behavior.
- Multi-file workspace-symbol and document-scoped rename/reference edge-case behavior is fixture-covered.

## Remaining v0.14.0 Checklist Work

- packaging and distribution follow-through
- tree-sitter and editor adapter maturity (remaining items)
- runtime and tooling reliability track
- v1.0.0 scope definition gate
