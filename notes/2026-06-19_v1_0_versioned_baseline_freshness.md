# V1 Versioned Baseline Freshness Pass - 2026-06-19

## Scope

`V1RR-P1-006` required a freshness pass over versioned baseline labels in
architecture, scope, LSP, tree-sitter, editor install, and editor/tool docs.

Reviewed:

- `docs/ARCHITECTURE.md`
- `docs/V1_SCOPE.md`
- `docs/LSP_RELIABILITY.md`
- `docs/TREE_SITTER_KUJO.md`
- `docs/INSTALLATION_LSP_EDITORS.md`
- `docs/EDITOR_ADAPTER_BASELINES.md`
- `docs/INSTALL_MATRIX.md`
- `tools/vscode-kujo-extension/`
- `tools/kujo-doctor/`

## Changes

- Converted bare `v0.13.0`/`v0.14.0` status labels in editor/LSP docs into
  v1.0.0 release-candidate baseline labels that preserve older-version
  provenance.
- Updated install-matrix distribution guidance from stale `Pre-v1` / `pre-1.0`
  wording to pre-tag `v1.0.0` wording.
- Annotated `docs/V1_SCOPE.md` so `v0.13.0`/`v0.14.0` references are
  historical stabilization baselines.
- Updated `docs/ARCHITECTURE.md` to call the remaining checklist work
  pre-tag `v1.0.0` closure work.

## Intentional Historical Or Component Versions

- `tools/vscode-kujo-extension` remains `0.1.0`; this is the editor extension
  package version, not the Kujo runtime release state.
- Kujo Doctor `0.1.0` schema/package labels remain component-specific versions.
- Historical `v0.13.0`/`v0.14.0` references in the v1 scope doc remain as
  provenance for stabilization baselines.

## Validation

All commands passed:

- `cargo test --test editor_adapter_contracts`
- `cargo test --test tree_sitter_kujo_assets`
- `cargo test --test lsp_reliability_track`
- `cargo test --test v1_scope_docs_alignment`
- `cargo test --test docs_examples`

Logs and status manifest:

- `notes/release_evidence/2026-06-19_p1-006/status.tsv`
