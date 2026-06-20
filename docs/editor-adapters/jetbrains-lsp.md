# JetBrains LSP Adapter Baseline

Use a generic LSP plugin path (for example plugins that support custom external language servers).

Status: v1.0 generic LSP configuration path. JetBrains-specific plugin UI
steps vary by installed plugin; keep the Kujo-owned configuration limited to
the external server command below.

Canonical server command:

- executable: `kujo`
- args: `lsp`

Suggested language mapping:

- file extension: `.kujo`
- language id: `kujo`

Notes:

- Keep this adapter thin. Do not duplicate parsing, linting, or symbol analysis in IDE-specific code.
- The IDE adapter should only launch/configure `kujo lsp` and forward protocol payloads.
- Validate the server path with `cargo test --test lsp_external_clients_smoke`
  and descriptor shape with `cargo test --test editor_adapter_contracts`.
