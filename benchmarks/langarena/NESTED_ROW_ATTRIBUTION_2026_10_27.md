# Nested-row allocation isolation — 2026-10-27

The remaining nested-row width cost is consistent with one complete row-buffer
allocation per imported helper call. This is a diagnostic on runtime `868dfd8`
(checkpoint `87a5cbc`), not a runtime fix or a speed measurement.

The optional `row-buffers` mode in the allocation probe counts requests matching
128- and 2,048-element `Value` buffers. `Value` is 136 bytes on this local target.
These are allocation-size matches, not stack traces. Attribution is supported by
the call-count/width slopes, direct-array controls, snapshot controls and source
ownership paths; unrelated allocations could share a size.

All 128 instrumented VM/JIT runs pass their existing value/snapshot assertions.
Their total allocation requests and bytes exactly match the corresponding
previously verified runtime measurements. Instrumentation introduces no measured
allocation traffic. [Machine evidence](nested-row-attribution-2026-10-27.json)
records fixture, module, runner, probe and linked-library identities.
Eight additional default-mode controls preserve the original output and totals.

## Added 32 helper calls

One write per call, retaining the original snapshot once:

| State shape | Row width | Total requested bytes | Row-sized allocations | Row-sized bytes | Other bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| Direct array | 128 | 289,312 | 0 | 0 | 289,312 |
| Direct array | 2,048 | 289,312 | 0 | 0 | 289,312 |
| Struct → array | 128 | 885,344 | 32 | 557,056 | 328,288 |
| Struct → array | 2,048 | 9,241,184 | 32 | 8,912,896 | 328,288 |
| Dictionary → array | 128 | 887,232 | 32 | 557,056 | 330,176 |
| Dictionary → array | 2,048 | 9,243,072 | 32 | 8,912,896 | 330,176 |

VM and JIT have identical slopes throughout the measured matrix. After subtracting
row-sized allocations, the remaining bytes are independent of width in all
direct-array and nested-row cases. At width 2,048, row buffers account for about
96% of the added-call bytes in the nested one-write cases above.

Eight writes per helper still add **32**, not 256, row-sized allocations. This
points to detachment across helper calls rather than copying on every write.
Taking a fresh snapshot before every call adds 32 row buffers for direct arrays
as well as nested rows. That control must retain its value semantics; eliminating
all copies indiscriminately would be incorrect.

## Next implementation gate

The next optimization must reduce nested-row detachment across eligible helper
calls while keeping real public pre-return values and retained snapshots intact.
Current sparse publication handles direct arrays and existing integer dictionary
entries; nested assignment uses ordinary validated copy-on-write traversal.

Before accepting a nested publication design:

- Establish equivalent behavior for snapshots taken before and during calls,
  escaped rows, callbacks/reentry, owner replacement and completed writes before
  later errors.
- Preserve root-after-index ordering, immutable bindings, bounds, numeric values
  and fallback behavior for unsupported paths.
- Measure publication as well as mutation. A smaller helper allocation that moves
  the copy elsewhere does not close the cost.
- Check small rows, dense writes, multiple rows and fresh snapshots for added
  allocation; do not grow common runtime structures or retention silently.
- Prove improved added-call scaling in the actual runtime, then run all three
  runtimes' regressions and test-size benchmark checks.

No nested publication design is accepted by this diagnostic. Nested-row copying,
broader capture costs, production-size verification and clean-server comparisons
remain open in the [full checklist](PERFORMANCE_CHECKLIST.md).

## Reproduction

Build `probes/captured-append-scaling/allocation_probe.rs` against the verified
runtime library using the recorded Rust arguments (including `-C lto=thin` on this
host). Then run:

```text
ruby benchmarks/langarena/probes/nested-row-attribution.rb ROW_PROBE VERIFIED_MODULE_GATE OUTPUT_JSON
```

The gate directory contains the paired report and its unchanged generated
fixtures. The runner rejects source drift, changed allocation totals, failed
fixture assertions and overwritten outputs. Runtime source was unchanged, so the
previous checkpoint's 336 integration tests and 150 checksum results are prior
evidence, not newly repeated tests for this diagnostic.
