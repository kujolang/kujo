# V1U-DOC-001 — Architecture Doc Refresh

Date: 2026-05-20

## Scope

Replace stale architecture documentation that still described Kujo as interpreter-primary and VM-experimental.

## Changes

1. Replaced `docs/ARCHITECTURE.md` with a current architecture view aligned to pre-v1 reality:
   - VM default runtime path,
   - explicit interpreter fallback path,
   - `kujo test --runtime dual|vm|interpreter` strategy,
   - command/runtime-path references and release posture,
   - explicit generator divergence boundary.
2. Added `tests/architecture_docs_contract.rs` to lock:
   - required modern markers,
   - absence of known stale v0.8/v0.9 phrasing.

## Validation

```bash
cargo test --test architecture_docs_contract
cargo test --test docs_examples
```

Both commands passed.
