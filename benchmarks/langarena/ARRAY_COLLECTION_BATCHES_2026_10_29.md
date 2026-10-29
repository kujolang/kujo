# General collection batches in captured arrays

Runtime `268add2` retains eligible multi-index array, dictionary and struct payload
replacements in a bounded sparse journal. Admission checks primitive arguments,
array bounds, supported body shape and counted-loop writes. Compact paired storage
promotes when needed; struct batches can start directly. Existing snapshot,
callback and error publication boundaries remain authoritative. Unknown shapes
use ordinary mutation, and initial scalar-first journals are not converted.

Requested bytes per added helper call, owner width 2,048, VM and JIT:

| Array-valued writes per call | Before | After |
| --- | ---: | ---: |
| 3 | 286,634 | 8,302 |
| 8 | 289,559 | 11,719 |
| 64 | 322,319 | 49,071 |

All 18 new hot target groups have flat owner-width scaling across widths 64 and
2,048. These are allocation slopes, not elapsed-time speedups.

Verified against accepted runtime `e445f4a`:

- 1,261 library passes, eight existing ignored tests.
- 411 integration passes, one existing ignored test.
- All 150 test-size checksums: 50 benchmarks in VM, interpreter and JIT.
- 2,960 paired allocation comparisons and 5,304 semantic checks across batch,
  bounds/error, capacity, conditional, pair, fallback, struct and nested matrices;
  no allocation-total or slope regressions in those matrices.
- 15 additional scalar/compound contracts.
- 108 paired peak-memory observations across 54 groups, all reviewed.
- Exact source/artifact identities, complete test-name coverage, formatting and
  diff checks. The final acceptance receipt is embedded in the linked evidence.

All 27 large-owner groups without retained snapshots reduced mean peak RSS by
10.21–13.32%. Some small and retained-snapshot groups increased. The largest mean
percentage increase was mixed payloads/64 writes/interpreter: 182,272 bytes or
2.346%, positive in both observations. The largest retained-snapshot mean increase
was array/interpreter: 571,392 bytes or 1.069%, from paired increases of 1,089,536
and 53,248 bytes. These observations remain recorded as tradeoffs; their cause is
not established. Two local observations per group do not prove leak freedom,
retained-heap bounds, universal memory improvement or clean-host performance.

The first candidate was rejected for a first-use hash seed allocation. Explicit
seeds eliminate that initialization in the private validated-integer journal;
public dictionary hashing is unchanged. A subsequent library run found an invalid
standalone-block unit fixture before its assertion executed. Replacing it with a
valid conditional block preserved the rejection assertion. The failed log and
exact test-only source delta are retained; production code and measured binaries
were unchanged by that fixture correction. The repaired full library suite passed.

[Complete evidence](array-collection-batches-2026-10-29.json) contains observations,
fixtures, identities, tests, checksums and the memory assessment. Compound indexed
arithmetic, captured-scalar batching, broader capture materialization, unsupported
collection paths and representation review remain open. Sections 3/4 are not
complete. Production-size, platform and clean-host comparisons remain separate.
