# v0.13.0 CLI Machine-Readable Contract Hardening Evidence

Date: 2026-04-30
Track: v0.13.0 Cross-IDE Foundation
Checklist item: CLI And Machine-Readable Contract Hardening

## Implemented

- Added `--json` support for:
  - `kujo format`
  - `kujo docgen`
- Retained and validated JSON surfaces for:
  - `kujo lint`
  - `kujo lsp-complete`
  - `kujo lsp-definition`
  - `kujo lsp-references`
  - `kujo lsp-hover`
  - `kujo lsp-diagnostics`
  - `kujo lsp-rename`
  - `kujo lsp-code-actions`
- Added machine-readable contract documentation:
  - `docs/CLI_MACHINE_READABLE_CONTRACTS.md`

## Verification

Commands:

- `cargo test --test cli_json_contracts`

Results:

- PASS
- `4 passed; 0 failed`

Contract-gating coverage includes schema assertions for:

- `format --json`
- `lint --json`
- `docgen --json`
- all current LSP CLI `--json` helper commands

## Notes

- Exit-code policy and automation error-shape behavior are now explicitly documented for scripting/tooling consumers.
