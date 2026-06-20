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

If the command prints LSP usage/help, the release artifact includes LSP functionality.

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
```

Extension smoke sequence:

```bash
cd tools/vscode-kujo-extension
npm install
npm run check
```

This validates shipped binary includes LSP entrypoint and adapter descriptors remain canonical.
