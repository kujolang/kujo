# v0.13.0 Thin Editor Adapter Baselines Evidence

Date: 2026-04-30
Track: v0.13.0 Cross-IDE Foundation
Checklist item: Thin Editor Adapter Baselines

## Implemented

Published canonical thin-adapter setup guidance:

- `docs/EDITOR_ADAPTER_BASELINES.md`

Published adapter descriptor baselines:

- `docs/editor-adapters/vscode-cursor-settings.json`
- `docs/editor-adapters/neovim-lspconfig.lua`
- `docs/editor-adapters/jetbrains-lsp.md`

Each baseline launches official Kujo LSP with:

- executable: `kujo`
- args: `lsp`

## Verification

Command:

- `cargo test --test editor_adapter_contracts`

Result:

- PASS (`3 passed; 0 failed`)

Smoke contract scope:

- descriptor files exist and are parseable/readable
- each adapter baseline points to canonical `kujo lsp` command path
