# Nested-assignment dictionary keys — 2026-10-27

Runtime commit `868dfd8` changes dictionary-key handling in `src/interpreter/assignment.rs`.
Validation and nested traversal borrow string keys and format integer keys on the
stack. Existing leaf entries retain their stored keys. New entries still allocate
owned key storage. Validation-before-mutation, copy-on-write snapshots and
canonical integer spelling remain in place.

Baseline runtime: `bb0bd0e` (successful dictionary-read fix), repository checkpoint
`c692833`. Exact executable/library/source identities and raw paired results are in
[the machine evidence](assignment-key-runtime-2026-10-27.json).

## Allocation result

Added 128 nested writes at fixed row width, after subtracting the 128-iteration
case from the 256-iteration case:

| Runtime | Key bytes | Baseline requested bytes | Candidate requested bytes | Candidate allocation requests |
| --- | ---: | ---: | ---: | ---: |
| vm | 8 | 45184 | 43136 | 384 |
| vm | 1024 | 305280 | 43136 | 384 |
| jit | 8 | 45184 | 43136 | 384 |
| jit | 1024 | 305280 | 43136 | 384 |

The candidate removes key-length-dependent allocation from this measured update
path. Remaining loop and helper allocations are not zero. The same strict gate
rejects baseline-as-candidate with `Candidate key-length allocation remains`.
The runner defaults to its earlier read gate; optional `write` selects assignment
slopes while retaining both read/write controls.

## Verification

- All 16 paired read/write allocation cases and 24 semantic runs pass.
- All 336 release integration tests pass; one manual byte-growth profile remains
  ignored. The three expanded imported-key tests also pass on the baseline.
- All 18 direct baseline/candidate nested-assignment runs pass across the three
  runtimes, covering nested insertion/replacement, errors and evaluation order.
- All 150 test-size benchmark checksums pass: 50 each in VM, interpreter and JIT.
- All 128 module matrix pairs and 192 semantic runs pass; 32 pairs reduce
  requested bytes and 96 match, with no request/byte regressions.
- All 12 numeric-key pairs, 36 cold comparisons, eight append comparisons and
  three publication-boundary controls pass.
- Format, diff, source, benchmark-source, binary, library, probe and integration
  harness identity checks pass.

Regression coverage includes long and empty keys, i64 limits, `1` versus `01` and
`+1`, insertion and replacement, retained snapshots, missing and invalid keys,
invalid row indices, and preserving completed writes before a later failure.
Full library tests were not repeated for this narrow change; the matching release
integration suite covers the affected surface. The previous checkpoint's library
result is not presented as a result for this candidate.

An initial build was rejected by the source-identity gate because a formatting
change overlapped its start. The accepted rebuild used frozen sources; all results
above belong to that rebuilt binary. The machine evidence retains both outcomes.

## Limits and remaining work

These are allocation/correctness results on a loaded laptop, not elapsed-speed,
peak-memory, clean-server, production-size or platform acceptance. Nested-row
publication and broader helper/capture costs remain open in the
[full checklist](PERFORMANCE_CHECKLIST.md). No Kostya contact occurred.
