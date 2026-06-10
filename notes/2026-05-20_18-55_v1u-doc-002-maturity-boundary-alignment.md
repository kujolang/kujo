# V1U-DOC-002 — Maturity/Boundary Wording Alignment

Date: 2026-05-20

## Scope

Align pre-1.0 readiness and deferred-boundary wording across top-level docs.

## Changes

1. Added shared canonical readiness-boundary wording to:
   - `README.md`
   - `docs/V1_SCOPE.md`
   - `docs/LANGUAGE_SPEC.md`
   - `docs/KUJO_FEATURE_INVENTORY.md`
   - `docs/UNFINISHED_AND_MVP_AUDIT.md`
2. Added `tests/v1_maturity_boundary_alignment_contract.rs` to enforce that these docs remain aligned on readiness-boundary language.

## Validation

```bash
cargo test --test docs_examples
cargo test --test v1_maturity_boundary_alignment_contract
```

Both commands passed.
