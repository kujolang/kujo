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

The dependency check exercises the glob API used by `vscode-languageclient`.
Do not override minimatch to version 11 while the client calls `makeRe()`:
version 11 removes that method. GHSA-g3hv-75wg-m249 remains open pending a
compatible patched dependency or a separately reviewed client adaptation.
The current client uses optimization level 1; the advisory describes level 2
amplification. This limits the demonstrated path, but is not a package fix.

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
