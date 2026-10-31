# Repeated-array reservation by distinct destinations

The candidate bounds reservations by the union of up to eight proven constant
destinations, rather than reserving one entry per repeated write. It preserves
the existing conservative fallback for varying or unsupported selectors and
larger unions. Raw endpoint equality is checked before negative-index
normalization; physically aliasing endpoints alone do not prove a constant
destination. Single-write admission remains unchanged.

Baseline: accepted capture-storage runtime 342a653, incorporated in main
e5394ca. Candidate identity and frozen sources are recorded in build-state.json
and candidate-source-manifest.json. Accepted runtime: `1ed1866`. The scoped acceptance receipt binds the verified
source and review evidence. Sections 3 and 4 remain open for final review.

## Verified evidence

- Full library: 1,307 passed, eight ignored.
- Integration: 430 passed, one ignored; exact membership verified.
- All 50 test-size benchmarks pass in each of VM, interpreter and JIT: 150
  checksum cases. This does not establish production-size acceptance.
- Three portable regression tests cover six runtime/width combinations,
  including imported state, snapshots, aliases, bounds errors and negative
  selectors. Baseline and candidate both preserve the required outputs.
- Sixteen matched allocation pairs and 24 semantic executions pass. At width
  2,048, bytes per added call fall from 101,464 to 90,992 for 64 iterations,
  and from 1,532,824 to 1,364,912 for 1,024 iterations, in VM and JIT.
  The measured width penalties of 9,840 and 167,280 bytes become zero.
- Dense controls: 48 matched allocation pairs and 72 semantic executions per
  version pass; measured allocation slopes remain unchanged.
- A synthetic unchanged-counter control is rejected by the benefit verifier.
- Formatting and diff checks pass.

Historical controls also pass: 2,496 allocation pairs, 3,744 semantic
executions and nine host contracts. No total allocation increases were observed;
96 pairs decreased. The independent verifier validates scaling and exact
coverage against accepted capture-storage observations.

Extended controls pass: 660 allocation pairs, 990 semantic executions and nine
host contracts, including exact coverage and nonincreasing allocation slopes.

## Final control and memory review

The remaining gate passes 612 strict allocation pairs and 1,026 semantic
executions. All 36 VM boundary observations match; 36 JIT boundary cases received
108 matched repeats. Two positive nonoverlap cases received 14 additional pairs,
which showed overlapping ranges and positive/zero/negative deltas. All original
observations remain recorded. This is bounded evidence, not exact JIT allocation
equivalence.

RSS coverage passes 48 pairs across 24 cases. All 11 positive-mean cases received
55 additional pairs; every follow-up range overlaps. The largest positive
follow-up mean is 42,598.4 bytes (0.54893%). The initial largest positive mean of
122,880 bytes (1.58646%) remains in the record. Some low-water VM/JIT observations
remain 16 KiB higher. The review accepts this scoped process-memory tradeoff; it
does not claim zero memory regression or isolate retained heap.

Combined coverage: 3,832 strict allocation pairs, 72 reviewed boundary pairs,
5,856 candidate semantic observations and 18 host-contract executions. Dense
baseline semantics are separate and are not counted as candidate runs.

The scoped acceptance receipt passes all gates. Runtime `1ed1866` is committed
and pushed. Sections 3/4 still require their final representation review.

The first full-library attempt exposed two obsolete reservation expectations
in one representation test. V1 evidence is preserved; V2 changes those expected
capacities to two while retaining independent dictionary-owner assertions.
See V1_REVIEW.md. No runtime change was made between those attempts.

Loaded-laptop elapsed times are diagnostic only. Allocation reductions do not
prove a competitive speedup. Fresh escaped snapshots, public host-edited names,
and unsupported proof shapes retain costs and must remain explicit in the final
review. Clean-host comparisons, production/platform acceptance and subsequent
compiled-execution work are separate requirements.

[Compressed raw evidence and review receipts](repeated-array-fix-2026-10-31.json.gz)
retain exact artifact contents and SHA-256 identities. Decompress as gzip and
parse the JSON; each `content` field hashes to its adjacent `sha256`.
