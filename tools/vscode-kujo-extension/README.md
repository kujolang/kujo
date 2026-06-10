# Kujo Language Tools VS Code Extension

This extension provides:

- Kujo language registration for `.kujo` files
- TextMate-based syntax highlighting for Kujo source
- Optional `kujo lsp` client wiring for language intelligence

## Development Setup

```bash
cd tools/vscode-kujo-extension
npm install
```

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

If Kujo is not on PATH, point to an explicit binary path:

```json
"kujo.lsp.command": ["/absolute/path/to/kujo", "lsp"]
```

## Package As VSIX

```bash
npm install -g @vscode/vsce
vsce package
```

Then install the generated `.vsix` in VS Code/Cursor/Codex-compatible editors.

## Extension Settings

- `kujo.lsp.enabled`
- `kujo.lsp.command`
- `kujo.lsp.trace.server`
