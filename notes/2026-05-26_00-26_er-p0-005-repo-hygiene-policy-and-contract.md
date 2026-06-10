# ER-P0-005 — Repository hygiene policy and contract

Date: 2026-05-26
Item: ER-P0-005

## Summary

Closed ER-P0-005 by defining a root-surface hygiene policy and enforcing it with a contract test.

## Changes

- Added `docs/REPO_HYGIENE_POLICY.md`:
  - tracked-root allowlist,
  - non-root placement rules for generated/transient artifacts,
  - retention/cleanup expectations.
- Added `tests/repo_hygiene_contract.rs`:
  - validates tracked root files match the canonical allowlist,
  - validates policy doc includes required enforcement markers.
- Updated enterprise readiness checklist with closure evidence.

## Commands and results

- `cargo test --test repo_hygiene_contract` -> PASS (2/2)
- `cargo test --test docs_policy_consistency_contract` -> PASS (1/1)

## Residual risk

- Local untracked scratch artifacts can still exist in developer worktrees; the contract explicitly governs tracked repository surface to preserve production-facing cleanliness.
