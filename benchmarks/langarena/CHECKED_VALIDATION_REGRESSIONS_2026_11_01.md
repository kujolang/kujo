# Checked-assignment validation candidate, 2026-11-01

Candidate `568dc6127556984833617b3cd64914a2307420a2` removes a duplicate path
validation from `update_checked`. It is pushed separately and is **not yet
accepted or integrated**. Matching workload measurements and the remaining Linux
CI gates are pending. The accepted runtime remains `55e2f683`.

## Reproduction and change

The baseline release regression fails with four validation visits instead of two
for a one-level array write. `update_checked` validates the complete path, reads
depth bounds and releases weak cache references, then previously called
`update_in_place`, which validated the same path again. The candidate calls the
private writer at that point. Cache bookkeeping does not mutate the root.
Initial validation, copy-on-write ownership, depth limits and the exact-depth
repair fallback remain intact. Other `update_in_place` callers are unchanged.

The new structural regression checks one validation traversal at depths 1, 4 and
16 with unique and shared owners, including saved snapshot contents. A separate
integration regression covers mixed struct/dictionary/array writes, negative
indices, snapshot isolation, failed-write atomicity and recovery in interpreter,
VM and JIT modes. JIT mode alone is not proof that every operation executes as
native code; existing native-path regressions also run in the gate.

## Completed verification

- Formatting and whitespace checks pass.
- Release library: 1,316 passed, eight existing ignores, zero failures.
- Release `jit_nested_paths`: 17 passed; `nested_assignment`: 10 passed.
- Both new regressions pass. Exact test membership and executable hashes were
  checked against the completed test log.
- Explicit release CLI build passes. Frozen binary SHA-256:
  `3d6b3ea8e621d6bf8f74ce2cd8f102bc2da0629221b6d3fdc091640ff4e790b3`.
- All 50 unchanged test-size checksums pass separately in VM, interpreter and
  JIT: 150 cases. Independent verification checks ordered membership, complete
  outputs, exact configuration, exit status, checksum lines, and source, binary
  and report hashes.

The [19-artifact evidence bundle](checked-validation-regressions-2026-11-01.json.gz)
contains the failing baseline, passing regression logs, source/binary/commit
bindings, checksum reports and verifier. Every embedded artifact hash was checked
after compression. These are correctness and structural results, not a
production-size or elapsed-speed claim. Loaded-host before/after measurements
are still running; clean-host competitive comparisons remain pending.
