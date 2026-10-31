# Native root lookup inlining: not accepted

The experiment is preserved in `04a9321edc5ed718404c083d73d63549ee1ac98a` on
`codex/native-root-hash-experiment-20261031`. It is not merged into the accepted
runtime. Root hash mixing reduces large owned-row lifecycle costs, but the final
revision still slows uncached scalar reads and records a consistent MergeSort VM
process-tree CPU increase. Accepted runtime source remains `fd7b764`.

The change combines root-table hash mixing, direct owned-array borrowing, and an
inlining hint for `NativeOwnedValues::get`. Handles, stale-root checks, frame
ownership and value semantics remain intact. Added tests cover retained-root key
comparison counts and object-before-index error validation.

## Verification

- 1,311 library tests and 48 targeted integration tests pass; eight existing
  library tests remain ignored. Exact membership of all 11 suites is verified.
- All 150 test-size checksums pass across VM, interpreter and JIT.
- Four alternating helper process pairs contain 576 observations and 57.6 million
  outer indexed reads. Values, kinds, error behavior and root release checks pass.
- Four alternating workload pairs contain 48 verified observations: QuickSort,
  MergeSort and NeuralNet, each in VM and JIT. All matching checksums pass.
- Source, binary and input hashes, exact observation order, raw timing checkpoints
  and process-tree CPU deltas verify. Formatting and diff checks pass.

## Helper tradeoffs

Cached stable scalar-read medians are 4.8–7.0% lower, with all 12 external pairs
faster. At 2,048 live roots, owned-row lifecycle medians are 67.3–71.1% lower.
However, uncached scalar-read medians are 7.3–16.8% higher, with all 24 external
pairs slower. Cached rotating reads at 2,048 roots have a 5.0% higher median;
three of four pairs are slower.

Disassembly confirms that the formerly outlined lookup is now inlined into the
symbolized probe's indexed-read helper. That removes one generated call but does
not eliminate the measured uncached-read penalty. The evidence does not justify
another inlining change without further isolation.

## Whole-workload comparisons

Percentages below are medians of four candidate/baseline paired ratios. Negative
values mean less time. They are local diagnostics, not clean-host speed scores.

| Workload | Runtime | Kernel elapsed change | Process-tree CPU change |
| --- | --- | ---: | ---: |
| QuickSort | VM | -1.3% | -0.8% |
| QuickSort | JIT | -1.8% | -2.7% |
| MergeSort | VM | -1.9% | +6.0% |
| MergeSort | JIT | -2.6% | -2.4% |
| NeuralNet | VM | -3.2% | -1.7% |
| NeuralNet | JIT | -1.8% | -3.3% |

QuickSort JIT improves in all four elapsed and CPU pairs. MergeSort VM elapsed
results split two faster/two slower, while all four CPU pairs are slower
(ratios 1.093, 1.028, 1.190, 1.006). NeuralNet results are mixed; its lower medians
do not erase the slower pairs. No observations were discarded or repeated until
favorable. CPU counters include runner/child startup, compilation, warmup and
workload, not only kernel execution. The cause of the VM CPU increase is not
established by these results.

## Disposition and provenance

The combined tradeoff is not accepted. Further lookup work must cover retained,
reused and stale handles, cached and uncached reads, and owned-result lifecycle
costs. Clean-host review, unchanged production inputs, platforms and competitive
comparisons remain pending. No Kostya contact or release occurred.

Candidate CLI SHA-256:
`07af63497d2ee4c071a983de175698a46ca52713cc2aae065150a9097ce1bd68`.
Candidate library SHA-256:
`7904f4f97e691ed33a87724839f119ce277563dcd1fdc04b1fe2dad406821088`.
Baseline CLI SHA-256:
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`.

`native-root-inline-read-2026-10-31.json.gz` preserves 149 digest-verified artifacts,
including raw outputs, manifests, verifiers, all pairs and the acceptance review.
CPU counter calibration is explicitly reused from the prior same-host control;
it is not represented as a new observation. Local originals are in
`.audit-evidence/native-root-inline-read-2026-10-31/`. The current working source
has been restored to the accepted runtime; verify candidate manifests against
the preserved experimental commit, not the restored working tree.
