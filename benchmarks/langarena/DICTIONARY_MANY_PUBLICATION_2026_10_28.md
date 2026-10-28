# Multiple-key dictionary publication — 2026-10-28

Runtime `19a1736e7e081231fe2b3bb9fa1cd60c37cabaa4` avoids full captured-dictionary
copies for proven synchronous helpers updating multiple keys. The first existing
scalar key stays inline; a second promotes to a compact private write table.
Active tables also retain strings, byte buffers and new keys. Public pre-return
values and actual snapshots keep their existing semantics.

Baseline: `1c3acd427571f1e4964d61991944809af1b7a5ab`.
[Machine evidence](dictionary-many-publication-2026-10-28.json) preserves artifact
identities, fixtures, measurements, verification sources and rejected drafts.
These are local allocation results, not elapsed-speed or retained-memory claims.

## Measured results

The target covers widths 128/2,048, 1/2/4/16/64 writes, integer/mixed values,
initial-only/fresh-per-call snapshots and 16/32 calls. All 160 allocation pairs
and 240 semantic runs pass. Requested bytes decrease in 64 pairs; the other 96
match. No measured total or added-call request/byte regression remains. Eligible
multiple-key costs are flat across dictionary width; VM and JIT deltas agree.
Fresh-snapshot controls preserve required copying and match the baseline.

Integer updates at width 2,048 with an initial snapshot, per added helper call:

| Distinct writes | Baseline bytes | Candidate bytes | Baseline requests | Candidate requests |
| --- | ---: | ---: | ---: | ---: |
| 1 | 7,581 | 7,581 | 88 | 88 |
| 2 | 634,379 | 7,788 | 93 | 91 |
| 4 | 634,471 | 8,074 | 99 | 94 |
| 16 | 635,023 | 9,726 | 135 | 108 |
| 64 | 637,231 | 16,238 | 279 | 158 |

## Regression controls and rejected drafts

Four drafts were rejected before acceptance: an unconditional table increased
snapshot/escape costs; a broader proof admitted operator callbacks and a larger
value table increased dense-write traffic; string/insertion fallback then added
161 bytes per eligible call; byte/index/None boundaries exposed the same overhead.
Exact reproductions and artifact identities are retained in the evidence.

The accepted implementation uses compact values and a conservative cached body
proof. It excludes opaque calls, whole captured-value observations and indexed
operands that could invoke user operators. Contextual `None` keeps its ordinary
path. Computed primitive indices read pending values without speculative copies;
unsupported keys retain exact errors. Root-changing index callbacks retain the
ordinary object-before-index read order. Assignment retains the approved
replacement-root rule. Capture metadata layouts and existing pool limits remain
unchanged.

All 856 supplemental allocation pairs and 1,284 semantic runs pass: original and
expanded escapes, dense updates, operator callbacks, strings/insertions, bytes,
computed indices and invalid key types. Maintained boundary runners accept the
unchanged baseline and reject the known-bad draft. Historical array, scalar,
nested-row, capture, lookup, cold-call, append and publication controls also pass.
The separate 240-pair collection refresh has no allocation regression.

## Verification

- 1,200 library tests pass; eight existing tests are ignored.
- 30 targeted integration tests pass across VM, interpreter and JIT.
- 376 matching-release integration tests pass; one manual profiling test is ignored.
- 48 diagnostic pairs match exact stdout, stderr and exit status across all runtimes.
- All 150 unchanged test-size checksums pass: 50 each in VM, interpreter and JIT.
- Formatting, diff, frozen-source, benchmark and artifact-identity checks pass.
- Independent assembly verifies exact coverage, fixtures, deltas, rejected-draft
  evidence and all recorded runtime/control/checksum gates.

## Remaining scope

Sections 3 and 4 remain open. Indexed compound updates still use fallback because
indexed operands may invoke user operators. Deeper/larger rows, nested dictionaries,
general capture materialization/traversal and oversized fallback remain. Snapshot
copying required by value semantics is not an optimization defect. This change
does not establish production-size completion, platform/release acceptance or
competitive speed against Goose/Rust; clean-host comparisons remain pending.
