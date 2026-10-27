# Recursive captured-local restoration — 2026-10-27

Runtime `e9e5695c38a33a1b178376c559467c71a7142f7a` reuses the suspended caller's existing local-name and binding-kind
maps when a recursive call returns. It removes absent names, clears private
placeholder values and inserts newly introduced names. Owner-backed values,
retained snapshots, immutable bindings and parameter scopes keep their existing
semantics. General activation setup and pool limits are unchanged.

Baseline runtime: `ad6868058c6f1dd899d304f8edd35011bf59cafa`.
Exact sources, artifacts, build arguments and raw results are in
[the machine evidence](recursive-local-restoration-2026-10-27.json).

## Allocation results

Requested allocations per added outer call, with recursion depth two and writes
to a captured counter. Values are the difference between 16-call and 32-call
runs divided by 16. All extra local names are read and checked by a final call.

| Runtime | Extra locals | Baseline requests | Candidate requests | Baseline requested bytes | Candidate requested bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| jit | 128 | 341 | 81 | 88864 | 4302 |
| jit | 2048 | 6233 | 2133 | 2041233 | 683535 |
| jit | 8192 | 41057 | 24669 | 13598739 | 8161425 |
| vm | 128 | 341 | 81 | 88864 | 4302 |
| vm | 2048 | 6233 | 2133 | 2041233 | 683535 |
| vm | 8192 | 41057 | 24669 | 13598739 | 8161425 |

All 96 paired allocation cases and 144 semantic cases pass. The matrix covers
zero, 128, 2,048 and 8,192 extra locals, reads and writes, recursion depths zero
and two, and one, 16 and 32 calls. No total or per-added-call request/byte
regression occurred. The unchanged baseline correctly failed the strict
improvement gate, confirming compatibility alone cannot pass this gate.

These are requested allocation counts and bytes, not elapsed-speed or
peak-memory measurements. Local timing is diagnostic on the loaded laptop.

## Verification

- 1,178 library tests pass; eight existing tests remain ignored.
- 15 targeted integration tests pass, including new VM/interpreter/JIT contracts.
- The new semantic unit and three runtime contracts also pass the unchanged baseline.
- 342 matching release integration tests pass; one manual profile remains ignored.
- 18 direct baseline/candidate nested-assignment parity cases pass.
- 48 ordinary-local allocation pairs and 72 semantic controls pass without regression.
- 128 module allocation pairs and 192 semantic cases pass, preserving accepted
  width scaling for all four collection shapes at one and eight writes.
- 72 additional nested allocation pairs and 108 semantic cases pass.
- 16 read-key pairs, 12 numeric-key pairs, 36 cold comparisons, eight append
  pairs and three publication-boundary controls pass.
- All 150 test-size checksums pass: 50 each in VM, interpreter and JIT.
- Format, diff, source and artifact identity checks pass.

Regression coverage checks recursive writes, parent parameters, aliases,
independent closures, retained array snapshots, caught errors, immutable
bindings, changed name membership and release of stale private values.

## Remaining scope

Oversized activation setup, captured-prefix materialization, captured-name
traversal and broader collection costs remain open in the
[full checklist](PERFORMANCE_CHECKLIST.md). This change does not establish
production-size, supported-platform or clean-host Goose/Rust acceptance.
No Kostya contact occurred.
