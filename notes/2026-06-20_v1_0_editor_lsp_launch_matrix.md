# Kujo Field Notes - v1.0 Editor/LSP Launch Matrix

Date: 2026-06-20
Checklist item: `docs/V1_0_RELEASE_READINESS_GAP_CHECKLIST.md` `V1RR-P2-004`
Status: completed

## Decision

Kujo v1.0 editor support is defined as thin adapter launch guidance around one
canonical server command:

```bash
kujo lsp
```

The launch matrix covers VS Code, Cursor and VS Code-compatible forks, Neovim,
JetBrains through a generic external LSP plugin path, and generic stdio LSP
clients. Kujo does not claim latest host-editor version support as a runtime
release fact; the repo-owned contract is the server command, descriptor shape,
extension baseline, protocol responses, reliability guardrails, latency
guardrails, and Tree-sitter/editor asset presence.

## Changes

- Added a `v1.0 Launch Matrix` to `docs/EDITOR_ADAPTER_BASELINES.md`.
- Expanded `docs/INSTALLATION_LSP_EDITORS.md` with the current clean-environment
  smoke sequence and editor-family matrix.
- Clarified `docs/editor-adapters/jetbrains-lsp.md` as a generic LSP
  configuration path rather than a pinned JetBrains plugin/version claim.
- Added `tests/editor_launch_matrix_contract.rs` to guard launch-matrix
  coverage and canonical `kujo lsp` descriptors.

## Validation

Command logs and exit codes are recorded in
`notes/release_evidence/2026-06-20_p2-004/status.tsv`.
