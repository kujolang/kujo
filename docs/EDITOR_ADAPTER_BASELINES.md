# Editor Adapter Baselines

Status: stable v1.0.0 adapter baseline; originally introduced during the v0.13.0 editor-adapter track.

This document defines canonical thin-adapter setup paths for Kujo editor integrations.

Adapter rule:

- editor adapters must launch the official `kujo lsp` server
- adapters must not duplicate parser/analyzer/runtime logic
- shared behavior contracts belong to Kujo server/CLI docs, not per-editor forks

## Adapter Maintenance Policy

Kujo repository responsibilities:

- canonical protocol and machine-readable contracts (`docs/PROTOCOL_CONTRACTS.md`, `docs/CLI_MACHINE_READABLE_CONTRACTS.md`)
- minimal adapter baseline docs and launch examples under `docs/editor-adapters/`
- first-party extension baseline assets under `tools/vscode-kujo-extension/`

Editor-specific repository responsibilities:

- editor UX polish and host-specific packaging details
- editor release cadence/versioning that does not alter Kujo protocol contracts
- optional integrations that remain outside canonical Kujo CLI/LSP guarantees

Policy constraint:

- adapter docs in Kujo must stay thin and must link back to canonical Kujo contracts instead of duplicating protocol semantics.

## VS Code / Cursor / Codex-Compatible Editors

Canonical path:

- extension baseline: `tools/vscode-kujo-extension/`
- command: `kujo lsp`
- sample workspace settings: `docs/editor-adapters/vscode-cursor-settings.json`

Implementation expectations:

- `.kujo` files are mapped to Kujo language id and syntax scope so code is colorized on open
- delegate all language intelligence to Kujo LSP
- keep extension-side logic to launch/config + UX glue only

Notes:

- The first-party extension baseline contributes Kujo language registration, TextMate grammar highlighting, and optional Kujo LSP client startup.
- VS Code forks, including Cursor and other VS Code-compatible builds, can
  consume the same `.vsix` package path when their extension host supports the
  package's declared VS Code engine range.

## Neovim

Canonical path:

- command: `kujo lsp`
- sample lspconfig setup: `docs/editor-adapters/neovim-lspconfig.lua`

Implementation expectations:

- one LSP client instance per Kujo workspace root
- no duplicated Kujo syntax intelligence in Neovim Lua

## JetBrains (Generic LSP Plugin Path)

Canonical path:

- command: `kujo lsp`
- setup guide: `docs/editor-adapters/jetbrains-lsp.md`

Implementation expectations:

- map `.kujo` files to Kujo language id/server profile
- leave semantic behavior to server responses

## Helix

Canonical path:

- command: `kujo lsp`
- sample language-server descriptor: `docs/editor-adapters/helix-languages.toml`

Implementation expectations:

- map `.kujo` files to language id `kujo`
- use the editor's native external-LSP support

## Emacs (Eglot)

Canonical path:

- command: `kujo lsp`
- sample major-mode and Eglot adapter: `docs/editor-adapters/emacs-eglot.el`

Implementation expectations:

- keep the bundled mode intentionally minimal
- delegate diagnostics, navigation, completion, rename, and code actions to Eglot and Kujo LSP

## Generic LSP Clients

Canonical path:

- command: `kujo lsp`
- transport: stdio JSON-RPC with `Content-Length` framed messages
- protocol contract: `docs/PROTOCOL_CONTRACTS.md`

Implementation expectations:

- launch Kujo as an external language server process
- send standard `initialize`/`initialized`/`shutdown`/`exit` lifecycle messages
- reuse server responses instead of reimplementing Kujo parsing or symbol logic

## v1.0 Launch Matrix

This matrix confirms the launch path for each editor family without pinning
fast-moving editor host versions as Kujo release facts.

| Editor family | Launch path | Repo-owned v1.0 status | Validation evidence |
| --- | --- | --- | --- |
| VS Code / Cursor / VS Code-compatible forks | First-party extension baseline plus `kujo lsp` | Supported as a thin extension/configuration layer | `tools/vscode-kujo-extension/package.json`, `docs/editor-adapters/vscode-cursor-settings.json`, `npm run check`, `cargo test --test editor_adapter_contracts` |
| Neovim | `nvim-lspconfig` descriptor launching `kujo lsp` | Supported as a documented setup snippet | `docs/editor-adapters/neovim-lspconfig.lua`, `cargo test --test editor_adapter_contracts` |
| JetBrains | Generic external LSP plugin profile launching `kujo lsp` | Supported as documented generic LSP configuration | `docs/editor-adapters/jetbrains-lsp.md`, `cargo test --test editor_adapter_contracts` |
| Helix | Native `languages.toml` descriptor launching `kujo lsp` | Supported as a documented setup snippet | `docs/editor-adapters/helix-languages.toml`, `cargo test --test editor_adapter_contracts` |
| Emacs / Eglot | Minimal major mode plus Eglot server association launching `kujo lsp` | Supported as a documented setup snippet | `docs/editor-adapters/emacs-eglot.el`, `cargo test --test editor_adapter_contracts` |
| Generic LSP clients | stdio JSON-RPC launching `kujo lsp` | Supported protocol path | `tools/lsp_smoke_clients/python_client.py`, `tools/lsp_smoke_clients/node_client.mjs`, `cargo test --test lsp_external_clients_smoke`, `cargo test --test lsp_conformance_harness` |

Host-specific installation UIs, marketplace publication, and editor-specific
UX polish remain outside the Kujo runtime release contract. They should not
change the canonical server command or protocol semantics.

## Smoke Contract

Baseline adapter descriptors and launch smoke are contract-tested in:

- `tests/editor_adapter_contracts.rs`
- `tests/editor_launch_matrix_contract.rs`
- `tests/lsp_external_clients_smoke.rs`
- `tests/lsp_conformance_harness.rs`
- `tests/lsp_reliability_track.rs`
- `tests/lsp_latency_guardrails.rs`
- `tests/tree_sitter_kujo_assets.rs`

Smoke scope:

- descriptor files exist
- each descriptor explicitly points to `kujo lsp`
- canonical launch path is consistent across editor families
- external Python and Node LSP clients can launch `kujo lsp`
- LSP protocol fixtures, reliability guardrails, latency guardrails, and
  Tree-sitter editor assets remain valid
