# Direct native array borrowing: performance tradeoff remains

The direct-borrow revision passes correctness gates but is not accepted as a
runtime improvement. It retains the proposed root-table hash mixing and keeps
owned-array reads outside the generic `Cow<Value>` fallback.

All 150 test-size checksums pass across VM, interpreter and JIT. Exact membership
checks confirm 1,311 library tests and 48 targeted integration tests passed, with
eight existing library ignores. Source hashes and frozen artifacts match the run.

Four alternating process pairs produced 576 helper observations and 57.6 million
outer indexed reads. Each external comparison summarizes three internal samples;
all observations remain in the evidence. Relative to accepted `fd7b764`:

| Path | Candidate/baseline median range | Interpretation |
| --- | --- | --- |
| Cached stable scalar reads, 1/64/2,048 roots | 0.987–0.998 | Near baseline, mixed individual pairs |
| Uncached scalar reads, all shapes | 1.100–1.223 | 23 of 24 external pairs slower |
| Cached rotating scalar reads, 2,048 roots | 1.063 | All four pairs slower; one ratio is 1.493 |
| Owned-row lifecycle, 2,048 roots | 0.287–0.340 | 66.0–71.3% lower elapsed helper time |

The generated probe code removes the former conditional `Cow` pointer selection
before the cached array read. However, uncached reads now call
`NativeOwnedValues::get`; the accepted baseline had inlined that lookup. A separate
revision adds a targeted `#[inline]` hint. This is a code-generation hypothesis,
not proof of the entire slowdown's cause. Its verification is pending.

Whole-workload timing was not launched for this unaccepted revision. Loaded-host
helper results are diagnostic; they do not establish competitive, production or
universal no-regression acceptance.

Frozen CLI SHA-256:
`f26c8add3b60016e27de1bb0805030979b1885fcdca6f8ca70b4c5e38486997f`.
Frozen library SHA-256:
`2696d77d56cbca668d50dd82964b0eaf3f7e1b9fc1f2e6dc16d6ee85e16faa95`.
Probe SHA-256:
`c8df70338381273ec2e326f55c8fe716876264834363fe40a8f8c979ad644cbf`.

`native-root-direct-read-2026-10-31.json.gz` preserves 50 digest-verified artifacts,
including raw test output, checksum reports, paired observations, source manifests,
the unaccepted patch, disassembly and verification scripts. Local originals are in
`.audit-evidence/native-root-direct-read-2026-10-31/`.
