# Kujo Protocol Contracts

Status: v1.0.0 baseline draft (active)

This document defines machine-consumable protocol contracts used by Kujo CLI/LSP surfaces.

## Versioning

Contract version: `1.0.0-draft`

Rules:

- additive optional fields are non-breaking
- field removal/rename/type-change is breaking
- payload-affecting changes must update this document, tests, and changelog

## Diagnostics Contract

Used by:

- `kujo lsp-diagnostics --json`
- LSP `textDocument/publishDiagnostics`

Required fields:

- `line` (number, 1-based for CLI; 0-based inside LSP ranges)
- `column` (number, 1-based for CLI; 0-based inside LSP ranges)
- `severity` (string for CLI, numeric in LSP notification)
- `message` (string)

## Symbol Metadata Contract

Used by:

- `kujo lsp-definition --json`
- `kujo lsp-references --json`
- `kujo lsp-hover --json`
- LSP `textDocument/documentSymbol`
- LSP `workspace/symbol`

Required symbol identity fields by surface:

- symbol `name`/`label`
- location coordinates (line/column or range)
- optional kind metadata (`function`, `variable`, etc)

## Advanced Editor Metadata Contract

Used by:

- LSP `textDocument/semanticTokens/full`
- LSP `textDocument/inlayHint`
- LSP `textDocument/codeLens`

Required advanced-metadata fields:

- semantic tokens: `result.data` encoded in 5-field relative tuples (`deltaLine`, `deltaStart`, `length`, `tokenType`, `tokenModifiers`)
- inlay hints: `position` + `label` (with optional `kind`)
- code lenses: `range` + `command.title` + `command.command`

## Edit Contract

Used by:

- `kujo lsp-rename --json`
- LSP `textDocument/rename`
- LSP `textDocument/formatting`
- LSP `textDocument/rangeFormatting`
- LSP `textDocument/codeAction`

Required edit metadata:

- target location/range
- replacement text
- deterministic edit ordering for identical input

## Error Contract

LSP protocol errors follow JSON-RPC standard envelope:

- `code`
- `message`

Kujo-specific codes currently used:

- `-32601` method not found
- `-32602` invalid params
- `-32800` request cancelled
- `-32001` request timeout

## Validation Sources

Primary validation is enforced by tests:

- `tests/cli_json_contracts.rs`
- `tests/lsp_conformance_harness.rs`

Golden fixture set (shape-locked contracts):

- `tests/lsp_fixtures/all_required_methods_success_error.json`
- `tests/lsp_fixtures/completion_ordering.json`
- `tests/lsp_fixtures/edit_range_stability.json`
- `tests/lsp_fixtures/error_payload_consistency.json`
- `tests/lsp_fixtures/multi_file_workspace_symbol_rename_references.json`

## LSP Method Compatibility Table

The table below is the canonical support matrix for Kujo's LSP server at this contract baseline.

| Method | Status | Contract Notes |
| --- | --- | --- |
| `initialize` | Supported | Returns capability set and server version. |
| `shutdown` | Supported | Returns `null` result and marks clean exit path. |
| `textDocument/didOpen` | Supported | Stores in-memory document and emits diagnostics. |
| `textDocument/didChange` | Supported | Applies latest content change and emits diagnostics. |
| `textDocument/didClose` | Supported | Removes document and emits empty diagnostics. |
| `textDocument/completion` | Supported | Returns deterministic item ordering for identical source/position. |
| `textDocument/hover` | Supported | Returns markdown content and symbol range or `null`. |
| `textDocument/definition` | Supported | Returns location in target document or `null`. |
| `textDocument/references` | Supported | Returns declaration-aware references for the requested document. |
| `textDocument/rename` | Supported | Returns text edits under `result.changes[uri]`. |
| `textDocument/codeAction` | Supported | Returns syntax quick-fix actions derived from diagnostics. |
| `textDocument/semanticTokens/full` | Supported | Returns deterministic token legend encoding in `result.data` (5-field tuple groups). |
| `textDocument/inlayHint` | Supported | Returns deterministic hint list derived from local binding/type-inference heuristics. |
| `textDocument/codeLens` | Supported | Returns deterministic per-function code-lens command entries. |
| `textDocument/formatting` | Supported | Returns full-document edit list or empty list if unchanged. |
| `textDocument/rangeFormatting` | Supported | Currently returns full-document-style edit behavior. |
| `textDocument/documentSymbol` | Supported | Returns symbol list for requested open/resolved document. |
| `workspace/symbol` | Supported | Returns workspace symbols across open documents; output is URI-sorted for deterministic fixtures. |
| `$/cancelRequest` | Supported | Cancellation IDs map to JSON-RPC cancelled error envelope. |
| `initialized` | Supported | Notification accepted as no-op. |
| `exit` | Supported | Triggers process exit code based on shutdown sequence. |

Unsupported method behavior:

- Any unknown request method returns a JSON-RPC error envelope with:
	- `code: -32601`
	- `message: Method '<method>' is not supported`
- Requests with missing required parameters return:
	- `code: -32602`
	- stable method-specific error message (for example: `Missing textDocument.uri`)
