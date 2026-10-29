# General array batch candidate: first-use allocation rejection

The first candidate removes the owner-width penalty in all 18 hot target groups,
but is **rejected** for an extra allocation on the first use of its private index
hash table. The accepted runtime remains `e445f4a`; this report accepts no runtime
change. [Machine evidence](array-batch-rejection-2026-10-29.json) retains every
paired observation, semantic result, source identity and rejection.

## Measured results

- Target matrix: 288 paired allocation observations and 432 semantic checks;
  three payload shapes, 3/8/64 writes, widths 64/2,048, initial/fresh snapshots,
  1/2/16/32 calls, VM/JIT allocations and VM/interpreter/JIT semantics.
- Boundary matrix: 432 paired allocation observations and 1,296 semantic checks;
  three payload shapes, widths 3/8/2,048, early/late RHS errors, invalid indices,
  reads, callbacks and scalar transitions. Both baseline and candidate semantics
  are checked in all three runtimes.
- All semantic checks pass. The unchanged allocation gates reject 36 target
  totals plus 12 cold slopes, and 36 boundary totals plus 12 cold slopes.
  Every rejection is one additional allocation; none is a requested-byte increase.
- For three array writes, hot requested bytes per added call are 8,302 at both
  widths, versus 16,810/286,634 before. This is allocation traffic, not a speedup.

## Isolated cause and next gate

A separate counting-allocator executable, linked against the same `ahash`
artifact, records one request / 64 bytes for the first `AHasher::default()` and
zero for its second use. Explicit seeds record zero requests and bytes on both
uses. The library's default constructor invokes lazy fixed-seed initialization.
The first captured snapshot prevents optimization on call one, so this one-time
cost appears in the 1→2-call slope. The measured net allocation-count regressions occur only in array payload
cases; this does not attribute every allocation in other payload paths.

The correction uses an explicit-seed builder only for the bounded private map
of validated integer array indices. Public dictionary hashing is unchanged.
The corrected candidate is building and must repeat the target, boundary,
historical fallback, library/integration, 150-checksum and memory gates before
acceptance. The obsolete first candidate's library prebuild was canceled after
rejection; cancellation is not recorded as a test failure or pass.

Raw evidence and rejected source snapshot:
`.audit-evidence/array-batch-v1-2026-10-29/`. Corrected candidate:
`.audit-evidence/array-batch-v2-2026-10-29/`. Sections 3 and 4 remain open.
