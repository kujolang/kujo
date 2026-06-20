# Kujo v1.0 Next Session Actions (2026-06-20)

Status: current handoff after final review hardening passes on 2026-06-20

## Release-Flight Blockers

1. Complete `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md` after release owners
   provide `UNBLOCK_V1_RELEASE`.
2. Record release URLs, per-asset SHA-256 values, published-artifact smoke
   result, and command logs in dated `notes/` evidence.
3. Re-run the human-review verification bundle on the exact commit proposed for
   tag-time review.

## Useful Follow-Up Enhancements

1. Upgrade or replace remaining advisory-bearing transitive dependencies where
   upstream support exists. Current local audit warnings are scoped to optional
   image lockfile metadata, JIT support, and database support.
2. Add `cargo-deny` to the local and CI release workflow once the tool is
   installed for this repo, keeping `cargo audit` as the fallback gate.
3. Expand performance baselines with reproducible before/after numbers for
   parser throughput, VM execution, JIT warmup, and native image operations.
4. Continue moving user-facing examples toward small helper-driven patterns
   when repeated `print(...)` calls obscure the language feature being taught.
5. Keep root-surface hygiene strict: new top-level files should be justified in
   `docs/REPO_HYGIENE_POLICY.md` and covered by `tests/repo_hygiene_contract.rs`.

## Verification Starting Point

Use the release-candidate gate first, then add focused tests for the touched
area:

```bash
KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_candidate_gate.sh --full
```
