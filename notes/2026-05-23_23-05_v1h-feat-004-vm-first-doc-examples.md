# V1H-FEAT-004 — VM-first downstream docs/examples cleanup

Date: 2026-05-23
Item: V1H-FEAT-004

## Summary
Removed stale interpreter-preference usage from downstream security docs so examples default to VM execution while keeping explicit `--interpreter` fallback language only for compatibility/debug rationale.

## Documentation updates
- `docs/NATIVE_API_SECURITY_POSTURE.md`
  - Updated safe/unsafe command examples to VM-default:
    - `kujo run --untrusted --allow-fs-read ...`
    - `kujo run --untrusted --allow-net-client ...`
    - `kujo run --untrusted --allow-shell-exec ...`
  - Added explicit caveat that `--interpreter` is for targeted compatibility/debug isolation, not default workflow guidance.
- Kept bounded fallback references where they represent intentional runtime-path rationale.

## Validation
- `cargo test --test security_posture_docs_contract` ✅ (2 passed)
- `cargo test --test readme_contracts` ✅ (1 passed)
- `cargo test --test native_api_security_boundaries` ✅ (48 passed)
- `cargo test --test runtime_security` ✅ (9 passed)

## Notes
- No runtime/parser/resolver semantics changed in this loop.
