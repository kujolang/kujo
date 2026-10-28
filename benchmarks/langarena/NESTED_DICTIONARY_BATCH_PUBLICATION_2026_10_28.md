# Nested dictionary batch publication — 2026-10-28

Runtime `21a0907` extends nested dictionary publication to four scalar leaf
keys under dictionary, struct and array owners. Baseline runtime: `2caf05f`.
[Machine evidence](nested-dictionary-batch-publication-2026-10-28.json) preserves
source and artifact identities, paired measurements, verification and checksums.

## Measured allocation results

Requested bytes per added helper call, numeric dictionary owner, width 2,048,
initial-only snapshot, VM mode:

| Pattern | Baseline bytes | Candidate bytes |
| --- | ---: | ---: |
| one | 5183 | 5183 |
| two | 633076 | 5562 |
| four | 633920 | 6320 |
| five | 634342 | 634213 |
| insert | 633121 | 5607 |

Two-key, four-key and insertion cases have flat added-call costs across widths
128 and 2,048 in the eligible initial-only snapshot cases, across all three
owners and numeric/string keys in VM/JIT modes. Five-key writes retain fallback.
The matrix contains 480 paired allocation measurements and 1,440 combined
baseline/candidate semantic runs. Single-key and fresh-snapshot cases are controls;
required snapshot copies remain intact. No measured total or added-call allocation
regressions were accepted.

These are allocation traffic measurements, not elapsed speed or retained memory.
The loaded laptop cannot establish competitive performance against Goose or Rust.

## Semantics and limits

The first write must target an existing scalar dictionary key. Up to four distinct
keys can then be retained inline, including new keys; repeated keys coalesce even
when all four slots are occupied. Exact integer, floating-point, Boolean and null
values are preserved without increasing assignment-path storage. The fifth distinct
key uses fallback before changing the pending writes.

Snapshots, callbacks, error ordering, numeric/string aliases and replacement-root
assignment retain their behavior. Two separate 240-pair fallback matrices each
include 360 semantic runs; one starts from a single pending key and the other
starts with all four slots occupied. A further 36 baseline/candidate cases cover
root replacement during reads and writes. Imported JIT-mode fallback notices
remain visible; this is not proof that these helpers run entirely as native code.

## Verification

- 1,206 library tests pass; eight existing tests are ignored.
- 21 nested component tests pass, including scalar-bit and storage-layout checks.
- Three new integration tests pass in VM, interpreter and JIT modes across all
  three owners; the same tests also pass on the accepted baseline.
- 385 matching-release integration tests pass; one manual profiling test is ignored.
- All 150 unchanged test-size checksums pass: 50 per runtime.
- The 144-pair single-key control, historical dictionary/array/capture/call controls,
  diagnostics and the 240-pair collection refresh pass without measured regressions.
- Independent evidence assembly verifies exact coverage, fixture/artifact hashes,
  recomputed deltas and the negative control that rejects the unchanged baseline.
- Formatting, diff and frozen-source checks pass.

## Remaining work

More than four leaf keys, first-write insertion, nonscalar leaf writes, deeper
paths, larger rows and broader capture materialization can retain copying costs.
Sections 3 and 4 remain open. Production-size performance, native execution coverage,
platform/release gates and clean-host comparisons are separate unfinished work.
