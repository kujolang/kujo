# Kujo VS Code Extension

This guide covers the first-party Kujo VS Code extension in
`tools/vscode-kujo-extension`.

The extension provides:

- Kujo language registration for `.kujo` files
- TextMate syntax highlighting
- Optional LSP integration through `kujo lsp`

## Install From This Repository

Use this path while the extension is not yet published to the Visual Studio
Marketplace, or when testing a local change before publishing.

### Prerequisites

Install:

- VS Code, Cursor, or another VS Code-compatible editor
- Node.js and npm
- Kujo, with the `kujo` binary available on `PATH`

Verify the Kujo LSP entrypoint:

```bash
kujo lsp --help
```

If you built Kujo from source and have not installed it globally yet, use the
absolute path to the built binary in the extension setting shown below.

### Package The Extension

From the repository root:

```bash
cd tools/vscode-kujo-extension
npm install
npm run check
npm install -g @vscode/vsce
vsce package
```

This creates a VSIX file such as:

```text
kujo-language-tools-0.1.0.vsix
```

The exact filename follows the `name` and `version` in
`tools/vscode-kujo-extension/package.json`.

### Install The VSIX In VS Code

Command-line install:

```bash
code --install-extension kujo-language-tools-0.1.0.vsix
```

For VS Code Insiders:

```bash
code-insiders --install-extension kujo-language-tools-0.1.0.vsix
```

For Cursor:

```bash
cursor --install-extension kujo-language-tools-0.1.0.vsix
```

UI install:

1. Open VS Code.
2. Open the Extensions view.
3. Open the `...` menu in the Extensions view.
4. Choose `Install from VSIX...`.
5. Select the generated `kujo-language-tools-*.vsix` file.
6. Reload the editor if prompted.

### Verify Installation

Open a `.kujo` file.

Expected result:

- The language mode is `Kujo`.
- Syntax highlighting is active.
- If `kujo.lsp.enabled` is `true`, the extension starts `kujo lsp`.

If Kujo is not on `PATH`, set an explicit command in VS Code settings:

```json
{
  "kujo.lsp.command": ["kujo", "lsp"]
}
```

To disable LSP startup while keeping syntax highlighting:

```json
{
  "kujo.lsp.enabled": false
}
```

## Publish To The VS Code Marketplace

Publishing makes the extension searchable and installable from the VS Code
Extensions view instead of requiring users to install a VSIX from the repo.

The Marketplace extension ID will be:

```text
kujolang.kujo-language-tools
```

That ID comes from:

- `publisher`: `kujolang`
- `name`: `kujo-language-tools`

### One-Time Publisher Setup

1. Sign in to the Visual Studio Marketplace publisher management page.
2. Create or select the `kujolang` publisher.
3. Confirm the extension manifest uses the same publisher ID:

   ```json
   {
     "publisher": "kujolang"
   }
   ```

4. In Azure DevOps, create a Personal Access Token for publishing.
5. Use `All accessible organizations` for the token organization selection.
6. Give the token the `Marketplace (Manage)` scope.
7. Install the VS Code extension publishing CLI:

   ```bash
   npm install -g @vscode/vsce
   ```

8. Log in with the publisher ID and paste the token when prompted:

   ```bash
   cd tools/vscode-kujo-extension
   vsce login kujolang
   ```

### Pre-Publish Checklist

Before the first public release:

- Run `npm run check`.
- Make sure `package.json` has the correct `publisher`, `name`,
  `displayName`, `description`, `version`, `engines.vscode`, and `categories`.
- Add or confirm Marketplace presentation files:
  - `README.md`
  - `LICENSE`
  - `CHANGELOG.md`
  - optional `SUPPORT.md`
  - optional PNG icon of at least 128x128 pixels, referenced by `icon`
- Keep README and changelog image URLs as `https` URLs.
- Do not use SVG images for the extension icon.
- Keep `keywords` to 30 or fewer if keywords are added.
- Ensure `.vscodeignore` excludes files not needed at runtime.

### Publish

From the extension directory:

```bash
cd tools/vscode-kujo-extension
npm install
npm run check
vsce package
vsce publish
```

To publish and bump the version in one command:

```bash
vsce publish patch
```

You can also publish an exact version:

```bash
vsce publish 0.1.1
```

After publishing, users can install the extension from VS Code by searching for
`Kujo Language Tools`.

### Manual Marketplace Upload

If CLI publishing is not desired:

```bash
cd tools/vscode-kujo-extension
vsce package
```

Then upload the generated VSIX from the Visual Studio Marketplace publisher
management page.

### Verified Publisher Badge

Marketplace publisher verification is separate from publishing. A publisher can
apply for verification after meeting Marketplace requirements, including having
one or more extensions on the Marketplace for at least 6 months and verifying
ownership of an eligible domain whose registration is also at least 6 months
old.

## End-User Install Path

Once the extension is published:

1. Install Kujo and make sure `kujo --version` works.
2. Make sure `kujo lsp --help` works.
3. Open VS Code.
4. Open the Extensions view.
5. Search for `Kujo Language Tools`.
6. Select the extension published by `kujolang`.
7. Click `Install`.
8. Open a `.kujo` file.

If the extension is distributed as a release VSIX instead:

```bash
code --install-extension kujo-language-tools-0.1.0.vsix
```

Then open a `.kujo` file and confirm the language mode is `Kujo`.

## Troubleshooting

If highlighting works but language intelligence does not:

1. Confirm Kujo is installed:

   ```bash
   kujo --version
   ```

2. Confirm the LSP command works:

   ```bash
   kujo lsp --help
   ```

3. If Kujo is not on `PATH`, configure:

   ```json
   {
     "kujo.lsp.command": ["kujo", "lsp"]
   }
   ```

4. If needed, temporarily disable the LSP client:

   ```json
   {
     "kujo.lsp.enabled": false
   }
   ```

## References

- VS Code publishing guide:
  https://code.visualstudio.com/api/working-with-extensions/publishing-extension
- Kujo editor adapter baseline:
  `docs/EDITOR_ADAPTER_BASELINES.md`
- Kujo editor install guide:
  `docs/INSTALLATION_LSP_EDITORS.md`
