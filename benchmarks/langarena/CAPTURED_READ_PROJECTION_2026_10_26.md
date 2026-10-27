# Captured global reads: verified allocation reduction

Runtime commit `c79010d` projects only the global names actually read by a
proven synchronous, read-only function. It avoids copying unrelated captured
bindings and reuses bounded private map storage. The baseline is `c0e45b3`.
The [machine-readable evidence](captured-read-projection-2026-10-26.json)
contains source/build identities, fixtures, allocation results and checksum reports.

## Measured result

Per-added-call VM allocation traffic for a captured array read:

| Unrelated bindings | Baseline requests / bytes | Candidate requests / bytes |
| --- | ---: | ---: |
| 0 or 128 mutable bindings | 24 / 1,160 | 16 / 1,086 |
| 8,192 mutable bindings | 8,231 / 2,720,272 | 16 / 1,086 |

The candidate remains flat for mutable and immutable owners. The matrix checks
128- and 2,048-element arrays at zero unrelated bindings, and 128-element arrays
at 128 and 8,192 unrelated bindings. JIT entry-path deltas are 16.015625 requests
and 1,086.8125 bytes per added call; raw totals remain in the evidence. These
are process allocation counts, not execution-speed or peak-memory measurements.

All 36 paired cold/oversized comparisons allocate 8,218 fewer requests and
2,717,949 fewer bytes in total. The saving is constant across the tested call
counts and recursive depths: it must not be presented as a per-recursive-call
improvement. Deeper recursive materialization remains open.

Append controls at 512, 1,024, 2,048 and 4,096 elements retain identical request
counts and linear growth in VM/JIT, with a fixed 56 additional requested bytes
per run. Module collection-width copying is not eliminated by this change.

## Semantics and rejected draft

Projection uses actual owner values and parameter names, preserving independent
collection snapshots and binding kinds. Operators and iteration additionally
require primitive arguments and captured dependencies. Assignments, callbacks,
unknown syntax, tasks, active reentry and owner-backed captured locals retain
the existing environment setup. No capture lock is held across user code.

The first draft incorrectly projected captured locals into snapshots. A unit
reproducer showed an ordinary activation observing an owner update from 7 to 8
while the projected value stayed 7. That draft was rejected. The accepted path
requires exactly one owner scope and no live capture; the reproducer now passes.
Its failed result and reason are retained in the evidence.

## Verification

- 1,146 library tests passed; eight ignored tests are explicitly inventoried.
- 292 integration tests passed; one manual profiling test remains ignored.
- Three focused VM/interpreter/JIT contracts passed, including snapshots,
  owner updates, reentry, missing-name errors and bounds errors.
- Explicit interpreter task/field ordering checks passed.
- All 150 test-size checksums passed: all 50 benchmarks in each runtime.
- Imported-helper, captured-read, recursive and oversized-read allocation gates
  passed, alongside module-owner controls, paired cold controls and module-write
  boundary checks. Exact counts and fixtures are in the JSON report.
- Matching release/library/probe identities and frozen source hashes verified;
  formatting and diff checks passed.

This does not complete sections 3 and 4. General capture traversal, deeper
materialization and module collection detachment remain, along with the later
compiled-execution, production-size and platform/release gates. Local laptop
timings remain diagnostic only; clean-server Goose/Rust comparisons are pending.
