# V1U-DOC-004 — High-Risk Docs Policy Contract Refresh

Date: 2026-05-20

## Scope

Add/refresh docs contract tests so key readiness/runtime/deferred policy text cannot silently drift.

## Changes

1. Added `tests/docs_policy_consistency_contract.rs` with cross-doc checks for:
   - canonical pre-1.0 readiness boundary wording,
   - README and parity-matrix agreement on top-level generator divergence,
   - standard-library tier guarantee policy wording,
   - architecture VM-default + interpreter-fallback posture markers.
2. This complements existing targeted docs contracts and adds a single high-signal consistency guard across the most drift-prone policy docs.

## Validation

```bash
cargo test --test docs_examples
cargo test --test docs_policy_consistency_contract
```

Both commands passed.
