# v0.14.0 Packaging And Distribution Follow-Through Evidence

Date: 2026-05-01
Context: local development machine (macOS)
Scope: ROADMAP section "3. Packaging And Distribution Follow-Through"

## Implemented

- Added cross-platform release artifact validation script:
  - `.github/scripts/validate-release-artifact.sh`
- Added Linux/macOS CI matrix workflow for artifact validation:
  - `.github/workflows/release-artifact-validation-matrix.yml`
- Added release artifact validation documentation:
  - `docs/RELEASE_ARTIFACT_VALIDATION.md`
- Updated install/docs references and baseline assumptions:
  - `INSTALLATION.md`
  - `README.md`

## Validation Commands (Local macOS)

1. `bash .github/scripts/validate-release-artifact.sh`
- Result: PASS
- Includes:
  - release build (`cargo build --release`)
  - clean-root install (`cargo install --path . --root <temp> --force --locked`)
  - installed binary run checks (`kujo --version`, `kujo run examples/hello.kujo`)
  - checksum generation/verification for `target/release/kujo`
  - repository-independent tarball packaging, checksum verification, extraction, and execution (`artifact-ok` output)

## Notable Output Signals

- Installed binary check output included:
  - `kujo 0.13.0`
  - `Kujo Kujo!`
- Repository-independent extracted artifact run output included:
  - `kujo 0.13.0`
  - `artifact-ok`
- Checksum verification included:
  - `kujo: OK`
  - `kujo-local.tar.gz: OK`

## Acceptance Mapping

- Reproducible checksum generation + verification flow is documented and automated.
- Minimum supported release-validation assumptions are documented (`Rust 1.86+`, Linux/macOS CI baselines).
- Install/run flow has an explicit repository-independent artifact check path.
- Linux/macOS validation workflow is codified in CI matrix and executes the same canonical script.

## Remaining v0.14.0 Checklist Work

- tree-sitter and editor adapter maturity (remaining items)
- runtime and tooling reliability track
- v1.0.0 scope definition gate
