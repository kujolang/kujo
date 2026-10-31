# Capture storage admission follows aggregate pool budgets

Accepted runtime: `342a653`. Sections 3 and 4 remain open.

The runtime selects compact local binding metadata when projected fresh map
capacity, key bytes, or aggregate scope storage would exceed the existing idle
pool budgets. It considers all scopes together and ignores unused capacity in
public owner maps. Actual retained-storage checks remain authoritative; pool
limits do not increase. Snapshot behavior, immutable bindings, host edits,
independent closures and publication after errors retain their contracts.

Measured requested bytes per added read call, calls 16–32, VM:

| Capture | Before | Candidate |
| --- | ---: | ---: |
| 8,190 names | 2,720,287 | 1,650 |
| 1,024 names with long prefix (128 baseline label) | 466,483 | 1,650 |
| 1,024 names with long prefix (240 baseline label) | 581,171 | 1,650 |
| 6,144 names across three scopes | 2,042,119 | 2,706 |

These are allocation results, not elapsed-time speedups. All 32 targeted
adjacent-call slope comparisons improve bytes and requests. The unchanged
counter negative control is rejected. Small captures and cold totals retain
strict non-increase gates.

Verification against accepted runtime `b401dc6`:

- 1,304 library passes (8 ignored); 424 integration passes (1 ignored).
- All 150 test-size benchmark checksums: 50 each in VM, interpreter and JIT.
- Three additional portable regression tests cover four capture shapes each,
  aliases, independent closures, immutable bindings, snapshots and thrown errors.
  Their promoted source is the exact rustfmt output of the tested harness.
- 3,784 strict allocation pairs; 5,784 semantic observations; 18 historical and
  extended contract executions.
- 72 separately reviewed boundary pairs. All 36 VM pairs match exactly.
  The 36 JIT cases received 108 alternating paired follow-ups; both request and
  byte ranges overlap in every group. Positive observations remain recorded.
  This does not establish exact JIT allocation equivalence or explain variation
  by laptop load.
- 120 live requested-heap pairs retain a scoped metadata-storage tradeoff:
  maximum additional retained bytes 557,202; maximum positive peak delta 42.
  Retained differences agree at calls 1 and 32 and across VM/JIT for all 30
  shape/operation groups. This is not RSS or universal leak-freedom evidence.
- The initially slower unpaired portable suite was followed by two alternating
  paired diagnostics. Candidate wall and CPU times were lower in both pairs;
  the original observations remain. No clean-host throughput claim follows.

RSS: all 360 pairs across 180 groups were verified and reviewed. Ninety-four
initial group means increased; the largest was +6,871,040 bytes (14.795%).
All 35 groups above 5% or 1 MiB received three alternating paired repeats.
Three interpreter groups positive in every repeat received six further pairs
each, yielding both signs and overlapping ranges. Positive means remain in two
of those final groups (about 829 KB and 1.27 MB). All 123 follow-up pairs are
retained. These are explicitly accepted process-memory tradeoffs, not a claim
of universal memory improvement or an attribution to laptop load.

Remaining section-3/4 work: repeated-array reservation and final representation
review. Production-size, platform and clean-host competitive acceptance remain
separate requirements.

[Compressed raw evidence and review receipts](capture-storage-budgets-2026-10-31.json.gz)
retain exact artifact contents and SHA-256 identities. Decompress as gzip and
parse the JSON; every `content` field hashes to its adjacent `sha256`.
