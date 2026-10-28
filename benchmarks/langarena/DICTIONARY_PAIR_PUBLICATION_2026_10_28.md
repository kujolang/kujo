# Inline dictionary pairs — 2026-10-28

Runtime `924e949` removes repeated full-dictionary copies for captured helpers
updating two existing canonical numeric keys, including compound assignments.
Baseline: `19a1736e7e081231fe2b3bb9fa1cd60c37cabaa4`.
[Machine evidence](dictionary-pair-publication-2026-10-28.json) preserves source,
artifact and fixture identities, raw measurements, checksums and verification code.

## Measured allocation improvement

Requested bytes per added helper call, compound updates with an initial snapshot:

| Dictionary width | Baseline bytes | Candidate bytes | Baseline requests | Candidate requests |
| --- | ---: | ---: | ---: | ---: |
| 2 | 13,865 | 13,146 | 118 | 114 |
| 128 | 52,421 | 13,146 | 118 | 114 |
| 2,048 | 639,941 | 13,146 | 118 | 114 |

VM and JIT agree. These are allocation measurements, not elapsed speed or retained
memory measurements. The target covers compound and whole-root self-index cases,
three widths, initial/fresh snapshots and 16/32 calls. All 48 allocation pairs and
72 semantic cases pass, with no measured total or added-call allocation regression.
The unchanged baseline passes control mode and correctly fails improvement mode.
The separate 240-pair collection comparison also passes without regressions.

## Semantics and implementation limits

Two signed 29-bit canonical decimal keys and scalar values fit in the existing
48-byte publication payload. This avoids allocating a write table and preserves
capture-frame size. Integer, exact floating-point bits, Boolean and null values
remain supported. Key bounds are an optimization limit only: other keys and
noncanonical spellings keep their ordinary behavior. Larger batches promote to
existing storage only when the existing conservative body proof permits it.
That proof has not been weakened.

Tests cover snapshots, owner identity, callbacks, completed writes before errors,
canonical aliases and range boundaries, promotion, discard and root observations.
Required snapshot copies remain. No public value representation or language
syntax changes are introduced.

## Verification

- 1,203 library tests pass; eight existing tests are ignored.
- 30 targeted integration tests and three reentry tests pass.
- 379 matching-release integration tests pass; one manual profiling test is ignored.
- 48 diagnostic pairs match stdout, stderr and exit status against the baseline.
- All 150 unchanged test-size checksums pass: 50 each in VM, interpreter and JIT.
- Previous multiple-key, escape, operator, boundary, array, scalar, nested-row,
  capture, lookup, cold-call, append and publication controls pass.
- Independent evidence assembly verifies exact coverage, fixture/artifact hashes,
  recomputed allocation deltas and all runtime/checksum gates.
- Formatting, diff and frozen-source identity checks pass.

## Remaining cutoff work

Sections 3 and 4 are still open: broader nested dictionaries, deeper/larger rows,
general capture materialization/traversal, retained-name/prefix costs and oversized
fallback need further isolation and bounded fixes. More than two compound keys
can still retain fallback. This milestone does not certify production-size runs,
platform/release acceptance or competitive speed. Clean-host Goose/Rust comparisons
remain pending; the loaded laptop cannot establish those results.
