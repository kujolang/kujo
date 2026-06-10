# V1U-DOC-003 — Standard Library Tier Policy Clarification

Date: 2026-05-20

## Scope

Clarify which `docs/STANDARD_LIBRARY_REFERENCE.md` tier labels are v1 guarantees vs non-guaranteed surfaces.

## Changes

1. Added explicit `v1 contract policy for tiers` section in `docs/STANDARD_LIBRARY_REFERENCE.md`:
   - `stable`: guaranteed for v1 compatibility commitments.
   - `preview`: in-scope but non-frozen/non-guaranteed until promoted.
   - `experimental`: explicitly non-guaranteed for v1 compatibility.
2. Added canonical readiness/deferred-boundary references to keep this policy aligned with `ROADMAP.md`, `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`, and `docs/V1_SCOPE.md`.
3. Added `tests/stdlib_reference_policy_contract.rs` to lock this policy text.

## Validation

```bash
cargo test --test docs_examples
cargo test --test stdlib_reference_policy_contract
```

Both commands passed.
