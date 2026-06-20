# V1 Optional Typing Cluster Deferral - 2026-06-19

## Scope

`V1RR-P1-002` asked to either finish or explicitly defer the remaining optional
typing follow-up cluster from
`notes/2026-06-08_22-00_v1x-type-001-import-signature-resolution-and-loop-scope-fix.md`.

The cluster is deferred, not implemented, for `v1.0.0`:

- destructuring inference
- module existence checks
- struct field type lookup
- Promise unwrap typing
- permissive callable fallback policy tightening

## Decision

These items are checker precision work, not runtime correctness blockers for
the v1 dynamic execution contract. Kujo v1 keeps optional typing additive:
interpreter mode may emit non-fatal warnings, and default VM mode does not run
a mandatory static type gate.

## Evidence

- `docs/OPTIONAL_TYPING_DESIGN.md` now includes a
  `Post-v1 Type-Checker Follow-Up Cluster (V1RR-P1-002)` section with each
  deferred item named.
- `docs/V1_SCOPE.md` lists the same optional-typing precision follow-ups under
  non-blocking post-1.0 candidates.
- `tests/v1_scope_docs_alignment.rs` asserts that both docs keep those markers.
- Required validation logs are stored under
  `notes/release_evidence/2026-06-19_p1-002/`.
