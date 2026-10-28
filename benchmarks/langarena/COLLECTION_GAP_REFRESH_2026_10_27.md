# Remaining collection costs after scalar publication — 2026-10-27

Runtime `36b8006d1bd3af2883a6f22e6d839efb3c42ea11` passes all 240 allocation
measurements and 360 semantic checks in the unchanged collection-gap matrix.
[Machine evidence](collection-gap-refresh-2026-10-27.json) retains exact inputs,
binary/probe identities, raw counts, recomputed deltas and the independent verifier.
These are allocation measurements, not elapsed-speed or peak-memory claims.

Requested bytes per added helper call, initial snapshot retained, VM:

| Shape | Width 128 | Width 2,048 |
| --- | ---: | ---: |
| One integer dictionary key | 4,382 | 4,382 |
| One float dictionary key | 4,382 | 4,382 |
| Two integer dictionary keys | 43,657 | 631,177 |
| One struct-row element | 4,802 | 4,802 |
| Two struct-row elements | 5,141 | 5,141 |
| Two dictionary-row elements | 5,183 | 5,183 |
| Array-owned row element | 22,342 | 283,462 |
| Deeper dictionary-owned row element | 23,918 | 285,038 |
| Dictionary inside dictionary | 44,755 | 632,275 |
| Dictionary inside struct | 44,760 | 632,280 |

The matrix covers all ten shapes, widths 2/128/2,048, initial-only and fresh-per-call
snapshots, and 16/32 calls. VM/JIT allocation measurements and VM/interpreter/JIT
semantic checks preserve original snapshots and final values. Fresh snapshots
remain separate controls; their required copying must not be optimized away.

Single-key scalar and bounded row paths now have flat measured width costs.
Multiple-key writes, array-owned/deeper rows and nested dictionaries still grow
with width. This refresh supplies the accepted baseline for the next fixes; it
does not establish that every remaining copy can be removed without tradeoffs.
Larger/dense updates and general capture materialization also remain open in the
[checklist](PERFORMANCE_CHECKLIST.md).
