# 2026-06-19 v1.0 Release-State Reconciliation

## Decision

Kujo is in pre-tag `1.0.0` release-candidate readiness.

`Cargo.toml` remains at `1.0.0` so candidate validation, release-state guards, and downstream artifact checks exercise the intended final crate metadata. This does not mean the final `v1.0.0` release has been tagged, published, or artifact-validated.

## Evidence

- `Cargo.toml` already declares `version = "1.0.0"`.
- `.github/scripts/check-release-state.sh` requires README and ROADMAP to match the `Cargo.toml` version.
- `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` still has tag-time release/publication work open.
- `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md` still requires real published binary assets, checksums, and published-artifact smoke evidence before sign-off.
- `docs/V1_0_RELEASE_READINESS_GAP_CHECKLIST.md` explicitly says not to close tag-time artifact blockers until the real release publication event exists.

## Scope

This note reconciles documentation state only. It does not authorize tagging, publishing, release artifact sign-off, or marking tag-time publication rows complete.

## Validation

Run after the reconciliation edits:

- `cargo test --test readme_contracts`
- `cargo test --test architecture_docs_contract`
- `cargo test --test release_process_docs_contract`
- `cargo test --test docs_policy_consistency_contract`
