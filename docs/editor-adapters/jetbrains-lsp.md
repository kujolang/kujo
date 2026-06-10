# JetBrains LSP Adapter Baseline

Use a generic LSP plugin path (for example plugins that support custom external language servers).

Canonical server command:

- executable: `kujo`
- args: `lsp`

Suggested language mapping:

- file extension: `.kujo`
- language id: `kujo`

Notes:

- Keep this adapter thin. Do not duplicate parsing, linting, or symbol analysis in IDE-specific code.
- The IDE adapter should only launch/configure `kujo lsp` and forward protocol payloads.
