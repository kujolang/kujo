# v1.0 JIT and Performance Posture Alignment Evidence

Date: 2026-06-19

## Summary

Closed `V1RR-P1-003` by aligning active runtime/performance docs with the
current source and test contract:

- default execution remains VM-first through `kujo run <file>`;
- `--interpreter` remains an explicit compatibility/debug path;
- JIT is experimental and opt-in through `kujo run --jit <file>`;
- unsupported JIT surfaces fall back to VM execution with deterministic
  messaging;
- current benchmark evidence is limited to committed benchmark artifacts and
  should not be generalized into broad public speed claims.

## Files Updated

- `docs/PERFORMANCE.md`
- `README.md`
- `docs/VM_INSTRUCTIONS.md`
- `docs/V1_0_RELEASE_READINESS_GAP_CHECKLIST.md`

## Evidence

- `src/main.rs` exposes `--jit` as an explicit `kujo run` opt-in flag.
- `src/main.rs` validates JIT-supported surfaces and disables JIT with a
  fallback message when unsupported surfaces are present.
- `tests/jit_execution_contract.rs` covers default no-JIT execution,
  unsupported-surface fallback, and supported-surface opt-in execution.
- `docs/VM_INTERPRETER_PARITY_MATRIX.md` already states that JIT is
  experimental and opt-in.
- Current benchmark evidence is committed in
  `docs/PERF_HOT_PATH_AUDIT_2026-05-26.md`,
  `docs/generated/VM_IMPORT_HEAVY_PERF_COMPARISON.md`, and
  `docs/generated/VM_IMPORT_HEAVY_CACHE_LOOKUP.md`.

## Validation

See `notes/release_evidence/2026-06-19_p1-003/status.tsv` for command output
paths and exit codes.
