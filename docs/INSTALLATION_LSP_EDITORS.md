# Install/Upgrade Path For Editor LSP Integrations

Status: v1.0.0 release-candidate editor/LSP install baseline; supersedes the original v0.13.0 adapter install note

## Install From Source

```bash
git clone https://github.com/kujolang/kujo.git
cd kujo
cargo build --release
./target/release/kujo --version
```

## Verify LSP Entrypoint

```bash
./target/release/kujo lsp --help
```

If the command prints LSP usage/help, the release artifact includes LSP
functionality.

## Upgrade Path

Repeat build from latest source revision:

```bash
git pull --ff-only
cargo build --release
./target/release/kujo --version
```

Then keep editor adapter command stable:

- executable: `kujo`
- args: `lsp`

## Editor Integration References

- VS Code/Cursor, Neovim, JetBrains baseline docs:
  - `docs/EDITOR_ADAPTER_BASELINES.md`
  - `docs/editor-adapters/`

## VS Code / Cursor / Codex Extension Path

For full VS Code extension packaging, Marketplace publishing, and end-user
install instructions, see `docs/VSCODE_EXTENSION.md`.

Build/install the first-party Kujo extension baseline:

```bash
cd tools/vscode-kujo-extension
npm install
npm install -g @vscode/vsce
vsce package
```

Install generated `.vsix` in your editor.

Example install commands:

```bash
# VS Code
code --install-extension kujo-language-tools-0.1.0.vsix

# Cursor (Codex-compatible fork)
cursor --install-extension kujo-language-tools-0.1.0.vsix
```

If your editor fork does not ship a CLI installer, install the same `.vsix` artifact from the Extensions UI.

After install, opening a `.kujo` file should immediately enable Kujo language mode and syntax colorization.

Optional workspace settings baseline:

- `docs/editor-adapters/vscode-cursor-settings.json`

## Clean-Environment Smoke Validation

Minimal smoke sequence:

```bash
./target/release/kujo lsp --help
cargo test --test editor_adapter_contracts
cargo test --test editor_launch_matrix_contract
cargo test --test lsp_external_clients_smoke
cargo test --test lsp_conformance_harness
cargo test --test lsp_reliability_track
cargo test --test lsp_latency_guardrails
cargo test --test tree_sitter_kujo_assets
```

Extension smoke sequence:

```bash
cd tools/vscode-kujo-extension
npm install
npm run check
```

This validates that the shipped binary includes the LSP entrypoint, adapter
descriptors remain canonical, generic Python/Node LSP clients can complete the
initialize/shutdown lifecycle, protocol fixtures still match, reliability and
latency guardrails hold, and editor grammar assets remain present.

## v1.0 Editor Launch Matrix

| Editor family | Install/configuration path | Current validation path |
| --- | --- | --- |
| VS Code | Install the first-party `.vsix`; leave `kujo.lsp.command` as `["kujo", "lsp"]` | `npm run check`; `cargo test --test editor_adapter_contracts` |
| Cursor / VS Code-compatible forks | Install the same `.vsix` through the fork CLI or Extensions UI | `npm run check`; `cargo test --test editor_adapter_contracts` |
| Neovim | Use `docs/editor-adapters/neovim-lspconfig.lua` with `nvim-lspconfig` | `cargo test --test editor_adapter_contracts` |
| JetBrains | Configure a generic external LSP plugin profile with executable `kujo` and arg `lsp` | `cargo test --test editor_adapter_contracts` |
| Generic LSP clients | Launch `kujo lsp` over stdio JSON-RPC | `cargo test --test lsp_external_clients_smoke`; `cargo test --test lsp_conformance_harness` |

The matrix intentionally avoids claiming latest host-editor version support.
The Kujo-owned contract is the server command, descriptor shape, extension
baseline, protocol responses, and editor asset presence.
