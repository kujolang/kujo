# v0.13.0 Release Checklist + CI Matrix Evidence

Date: 2026-04-30

## Implemented

- Added protocol contract policy doc:
  - `docs/PROTOCOL_CONTRACTS.md`
- Added install/upgrade and clean-environment smoke guide:
  - `docs/INSTALLATION_LSP_EDITORS.md`
- Added release artifact checklist:
  - `docs/RELEASE_ARTIFACT_CHECKLIST_V0_13_0.md`
- Added Linux/macOS contract matrix workflow:
  - `.github/workflows/lsp-contract-matrix.yml`

## Purpose

- enforce protocol/JSON contract tests across Linux and macOS
- verify release artifact includes `kujo lsp` entrypoint
- provide explicit install/upgrade + smoke validation guidance for editor integrations
