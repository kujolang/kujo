# Kujo Language Tools VS Code Extension

This extension provides:

- Kujo language registration for `.kujo` files
- TextMate-based syntax highlighting for Kujo source
- Optional `kujo lsp` client wiring for language intelligence

## Development Setup

```bash
cd tools/vscode-kujo-extension
npm install
npm run check
```

The extension pins `vscode-languageclient` 10.1.0 and overrides its minimatch
dependency to 11.0.4, fixing GHSA-g3hv-75wg-m249. The client previously called
the removed `makeRe()` API only to validate patterns. `scripts/patch-languageclient.js`
adapts those two checks to the parsed pattern set, preserving empty/comment
rejection and constructor errors while leaving matching to minimatch.

The adaptation verifies complete upstream file hashes before writing, rejects
unexpected versions/edits, and is idempotent. It runs during installation,
`npm run check`, and VSIX prepublication, including after an install with
`--ignore-scripts`. Do not bypass the prepublication check. When upgrading the
client, review/remove this adaptation and rerun the actual-client regression
tests; do not restore a global `makeRe()` shim. Tests cover both caller paths,
file-operation filters, URI patterns, errors and bounded pattern amplification.

## Run Locally In Extension Host

1. Open this extension folder in VS Code.
2. Press `F5` to launch an Extension Development Host.
3. Open any `.kujo` file in the host window.

Expected result:

- Language mode shows `Kujo`
- Syntax highlighting is active

## LSP Configuration

Default command:

```json
"kujo.lsp.command": ["kujo", "lsp"]
```

If Kujo is not on PATH, install it or set this command to the installed binary
name/path used on that machine:

```json
"kujo.lsp.command": ["kujo", "lsp"]
```

## Package As VSIX

```bash
npm install -g @vscode/vsce
vsce package
```

Then install the generated `.vsix` in VS Code/Cursor/Codex-compatible editors.

For full installation, Marketplace publishing, and end-user setup instructions,
see `../../docs/VSCODE_EXTENSION.md`.

## Extension Settings

- `kujo.lsp.enabled`
- `kujo.lsp.command`
- `kujo.lsp.trace.server`
