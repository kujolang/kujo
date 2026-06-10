# v0.14.0 Release Process Hardening Evidence

Date: 2026-04-30
Context: local development machine (macOS)
Scope: ROADMAP section "1. Release Process Hardening"

## Implemented

- Added release playbook: `docs/RELEASE_PROCESS.md`
- Added release-state guard script: `.github/scripts/check-release-state.sh`
- Added CI guard workflow: `.github/workflows/release-state-guard.yml`
- Updated release docs index in `README.md`
- Updated release checklist status in `ROADMAP.md`

## Verification Commands

1. `bash .github/scripts/check-release-state.sh`
- Result: PASS
- Output summary:
  - README.md matches Cargo.toml version `0.13.0`
  - ROADMAP.md current crate version matches Cargo.toml version `0.13.0`
  - overall status OK

2. `cargo test --test cli_json_contracts`
- Result: PASS (`4 passed; 0 failed`)

3. `cargo test --test lsp_conformance_harness`
- Result: PASS (`1 passed; 0 failed`)

4. `cargo test --test lsp_external_clients_smoke`
- Result: PASS (`2 passed; 0 failed`)

5. `cargo test --test lsp_latency_guardrails`
- Result: PASS (`1 passed; 0 failed`)

6. `cargo test --test editor_adapter_contracts`
- Result: PASS (`3 passed; 0 failed`)

7. `cargo test --test tree_sitter_kujo_assets`
- Result: PASS (`1 passed; 0 failed`)

## Acceptance Mapping

- Dry-run guidance with explicit, ordered commands and evidence requirements is now documented in `docs/RELEASE_PROCESS.md`, removing manual guesswork from release rehearsal.
- CI guard now enforces Cargo/README/ROADMAP release-status consistency via `.github/workflows/release-state-guard.yml`.

## Remaining v0.14.0 Checklist Work

- LSP protocol stability guarantees
- packaging/distribution follow-through
- tree-sitter and editor adapter maturity (remaining items)
- runtime/tooling reliability track
- v1.0.0 scope definition gate
