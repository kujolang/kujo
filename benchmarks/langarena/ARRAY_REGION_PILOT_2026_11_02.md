# Array-region candidate v1: paired screening

All 54 samples passed: three rounds of QuickSort, MergeSort and NeuralNet across
accepted VM/JIT, candidate VM/JIT, Rust and Goose. Source/binary/configuration
hashes, rotating order, checksums, raw timers, exit/truncation flags and RSS
receipts were independently verified. No build overlapped timing. This remains
a busy-laptop screening run, not independent-session or clean-host acceptance.

Accepted runtime: `782c712d`. Candidate CLI: `c95dfe17…`; exact identities and
the candidate patch are in the correctness receipt and archived evidence.

## Results

Measured-loop medians, seconds (not complete process time):

| Workload | Accepted JIT | Candidate JIT | Rust | Goose |
| --- | ---: | ---: | ---: | ---: |
| QuickSort | 1.716132 | 1.602916 | 0.017 | 0.018 |
| MergeSort | 7.415285 | 7.596014 | 0.022 | 0.017 |
| NeuralNet | 7.105223 | 6.236677 | 0.003 | 0.003 |

Competitor timers are rounded to milliseconds, especially limiting precision for
NeuralNet. These three workloads do not supply an all-suite score.

The median of **paired accepted/candidate ratios** is a different statistic from
the ratio of the two medians above. Greater than one means the candidate took
less time within that round:

| Workload | JIT paired ratios, rounds 1–3 | Median JIT ratio | Median VM ratio |
| --- | --- | ---: | ---: |
| QuickSort | 1.124, 0.977, 0.996 | 0.996 | 1.019 |
| MergeSort | 1.021, 0.931, 1.073 | 1.021 | 1.099 |
| NeuralNet | 1.180, 1.215, 1.063 | 1.180 | 0.893 |

QuickSort and MergeSort do not show consistent wins. NeuralNet is promising in
all three JIT pairs, but three observations do not establish a stable gain.
VM controls also vary materially despite unchanged VM source; do not assume
either a hardware cause or a runtime regression without further evidence, and
do not use VM ratios as a correction multiplier. The approximately 0.893 VM ratio
for NeuralNet remains an investigation item before acceptance.

## Disposition: revise once, do not accept v1

Source review identifies avoidable work introduced by this candidate:
`jit_indexed_read_region` refreshes through `borrow_owned`, and its fallback calls
`indexed_read` with no cache. Accepted repeated reads used
`borrow_owned_cached`, which validates a remembered root slot without repeating
the root hash lookup. Thus v1 discards an existing optimization on refresh and
non-array/non-integer paths. This is a confirmed call-path difference, not a
measured attribution of the table's timing changes.

Use the first experiment's one permitted revision to retain the validated root
cache alongside the borrowed view. Add a lookup-count regression for fallback
and repeated invalidation, retain all snapshot/callback/bounds controls, and
repeat actual-workload comparisons. A view must still be invalidated before
mutation or callbacks; preserving a lookup hint must not preserve a stale data
pointer. If the revised tactic lacks repeatable benefit, reject/defer it rather
than expanding this experiment indefinitely.

The wider native-access area remains open regardless of this tactic's outcome.
Neither helper elimination nor Rust/Goose parity is established. Corrected
library-unit, remaining integration, memory, production and clean-host gates
remain pending; the 150 test-size checksum passes are recorded separately.

## Evidence

`array-region-pilot-2026-11-02.json.gz` retains the complete manifests, sample
JSON, raw stdout/stderr, timed sources/configurations, identities, verifier and
driver. Every embedded hash was decoded and verified. Reproduction verification:

```sh
ruby benchmarks/langarena/probes/verify-array-region-pilot.rb EVIDENCE_DIRECTORY
```

The verifier requires the original binaries and sources to remain unchanged.
Before revising, the exact v1 CLI and library were retained locally as
`region-v1-kujo` and `region-v1-libkujo.rlib` under the campaign evidence directory.
The archive identifies binaries by hash; it does not embed executable artifacts.
