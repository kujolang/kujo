# Captured dictionary publication — 2026-10-27

Runtime `bdb232d` passes the allocation, correctness and test-size checksum gates
below. This is a verified source change, not a new packaged release.

This change removes repeated width-dependent copies for eligible updates to one
existing captured-global dictionary key. It measures requested allocation traffic,
not elapsed speed or peak memory. The laptop remains variably loaded; clean-server
Goose/Rust comparisons remain pending.

## Measured allocation result

The baseline is the frozen `d337cfa` release, including the accepted sparse-array
optimization. Both probes execute identical fixture paths. The matrix covers
arrays, struct-contained arrays, dictionary-contained arrays and dictionaries;
widths 128/2,048; one/eight updates per call; 32/64 calls; and retained-once/fresh
snapshots.

All **128 paired comparisons** pass: 16 reduce requested bytes, 112 match, and none
increase requests or requested bytes. All 192 corresponding semantic executions
pass across VM, interpreter and JIT.

For a 2,048-key dictionary, 64 calls, one update per call, and a retained original
snapshot:

| Mode | Baseline requests | Candidate requests | Baseline bytes | Candidate bytes |
| --- | ---: | ---: | ---: | ---: |
| VM | 28,536 | 28,032 | 46,385,627 | 6,894,770 |
| JIT | 28,626 | 28,122 | 46,395,792 | 6,904,935 |

The added 32 calls isolate recurring costs from initial setup:

| Dictionary width | Baseline added bytes | Candidate added bytes |
| --- | ---: | ---: |
| 128 | 1,553,696 | 295,488 |
| 2,048 | 20,354,336 | 295,488 |

With eight updates per call, candidate added bytes are also flat at 305,792.
These results match in VM and JIT. Initial setup and the first shared copy remain.
Fresh snapshots and the other collection shapes retain their prior behavior.

## Rejected draft and numeric-key regression gate

The first draft passed the original matrix, which used fixed-width string keys,
but regressed integer-key writes with fresh snapshots: 32 calls of eight writes
added 256 allocation requests and 4,864 bytes in both VM and JIT. Attempted
optimization formatted the integer key into a heap string before rejecting the
shared owner, then ordinary assignment formatted it again.

The repaired candidate checks owner eligibility first and uses a fixed 20-byte
stack buffer with standard decimal formatting for numeric keys. It handles all
`i64` values, including the minimum value, without treating `"01"` or `"+1"` as
the same key as integer `1`. Pending reads use the same canonical lookup.

The new gate covers integer/string keys, fresh/once snapshots and alternating
keys: **12 allocation pairs and 18 semantic executions** pass. Fresh-snapshot
requests and bytes match baseline exactly for both key kinds and both measured
runtimes. The other eight allocation pairs improve. This prevents accepting the
narrower matrix while overlooking numeric fallback costs.

## Implementation and semantics

A private 48-byte journal holds the existing root name, dictionary base, existing
key and one integer value. Repeated writes to that key replace the pending value.
The existing key's `Arc` and root-name storage are reused; journal creation does
not allocate. The capture enum's existing layout accommodates the payload, with
no new public `Value` variant or environment/frame field.

Admission requires an eligible synchronous, nonrecursive, global-only capture,
an unshadowed mutable binding and two base handles. The public owner remains a
real unchanged dictionary. Publication checks actual owner identity and uses
ordinary copy-on-write, so late observers and owner replacement retain value
semantics. The handle count is an eligibility check, not an unsafe uniqueness
assumption.

Whole-value observation, calls, task transitions and unsupported operations use
ordinary materialization. A second distinct key, insertion or noninteger value
falls back. Array and dictionary journals cannot overwrite each other when a
helper writes multiple roots. Snapshots, rollback, error ordering, mutability,
recursion and callback transfers remain covered.

## Verification

- 1,162 library tests pass; eight manual profiling tests remain ignored.
- 322 release integration tests pass; one manual byte-growth test remains ignored.
- 21 indexed-publication contracts and three additional dictionary reentry
  contracts pass on the candidate across VM/interpreter/JIT. The latter also pass
  on the frozen baseline and were committed separately as `26c961b`.
- The 128-pair module and 12-pair numeric-key gates pass as described above.
- All 36 cold allocation comparisons and 54 cold semantic controls pass.
- Eight append pairs have identical requests and bytes; three publication-boundary
  controls pass, one per runtime.
- All **150 test-size LangArena checksums** pass, 50 each in VM, interpreter and
  JIT, on the exact matching release candidate.
- Formatting, diff checks and source/binary/library/probe identity verification
  pass.

[Machine evidence](dictionary-publication-runtime-2026-10-27.json) retains source
hashes, compiler arguments, allocation pairs, checksum outputs, ignored-test names
and the rejected numeric-key draft. The separately committed reentry contracts
are verified against the candidate as well as the baseline; their source hash and
candidate identity are included.

Reusable gates, run from the repository root with Ruby:

```text
benchmarks/langarena/probes/indexed-publication-allocations.rb BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT_DIR dict
benchmarks/langarena/probes/dictionary-key-allocations.rb BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT_DIR
```

The workload fixtures are Kujo. Ruby maintains paired process/allocation evidence.

## Still open

Nested-row/struct publication, broader dictionary value types, multiple-key,
dense/escaping patterns, retained-local/name copying and general capture traversal
remain open. This change does not close sections 3/4 as a whole. Production-size
acceptance, compiled-path expansion, platform checks and clean-host comparisons
remain separate checklist items. No Kostya contact or upstream submission occurred.
