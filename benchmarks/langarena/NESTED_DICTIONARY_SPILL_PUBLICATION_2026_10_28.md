# Nested dictionary overflow publication — 2026-10-28

Runtime `a5d07bb` extends proven synchronous nested scalar dictionary batches beyond
four keys. Accepted baseline runtime: `21a0907`.
[Machine evidence](nested-dictionary-spill-publication-2026-10-28.json) preserves
artifact/source identities, matching release measurements, rejected candidates,
regression outputs and checksums.

## Measured allocation results

Requested bytes per added helper call, numeric dictionary owner, width 2,048,
initial-only snapshot, VM mode:

| Distinct writes | Baseline bytes | Candidate bytes |
| --- | ---: | ---: |
| 5 | 634,213 | 6,979 |
| 16 | 638,855 | 11,940 |
| 64 | 659,111 | 33,300 |

The primary gate verifies 384 allocation pairs and 1,152 combined semantic runs.
Eligible 5/16/64-key cases have flat added-call costs across leaf widths 128 and
2,048 for dictionary, struct and array owners, numeric/string keys and VM/JIT
modes. Four-key and fresh-snapshot cases remain controls. Both total requests/bytes
and added-call deltas must not regress. These are requested allocation traffic
measurements, not retained memory or elapsed-speed comparisons with Goose/Rust.

## Implementation and semantics

The first four distinct keys retain inline storage. A fifth key can promote to a
sparse table when a separate guard proves the helper's supported scalar region.
The guard borrows owner/argument values and inspects only touched reads, without
cloning struct names or scanning the entire dictionary. Its capacity hint follows
source work and leaf size; it is not a maximum semantic batch size. Further
insertions grow normally. Existing direct-dictionary batching rules are unchanged.

The first pending write still requires an existing scalar key. Subsequent writes
may insert keys or coalesce repeated keys. Exact scalar values, independent
snapshots, numeric/string key aliases, root replacement and error ordering remain
intact. Unproven bodies, opaque calls, dynamic loops and nonscalar/deeper writes
retain ordinary fallback. JIT-mode checks do not establish that imported helpers
execute entirely as generated native code.

Two earlier candidates were rejected. The [unconditional spill candidate](NESTED_DICTIONARY_SPILL_REJECTION_2026_10_28.md) increased 80 escape
cases by 280 bytes per warmed call. The second candidate passed escape/error
controls but regressed 88 total cases and 44 per-call comparisons through cloned
struct names and table growth. Both rejection records are retained; neither is
presented as accepted runtime behavior.

## Verification

- Library: 1,213 passed; eight existing ignores.
- Expanded permanent tests cover all 257 keys, untouched keys and retained
  snapshots under all three owners, including both loop fallback and a proven
  straight-line body; all three runtimes pass on baseline and candidate.
- Target: 384 paired allocations / 1,152 combined semantics.
- Single-key: 144 pairs; four-key: 480 pairs.
- Three fallback matrices: 240 pairs each; arithmetic errors: 96 pairs.
- Repeated writes, false branches and early return: 48 pairs / 144 combined
  semantic cases, with no total or per-call allocation regressions.
- Root replacement: 36 baseline/candidate cases.
- 388 matching-release integrations pass; one manual profiling test is ignored.
- Historical allocation/capture/callback controls and all 150 unchanged test-size
  checksums pass (50 per runtime).
- Independent coverage, source/artifact identity, recomputed-delta and
  unchanged-baseline negative checks pass, as do formatting and frozen-source
  checks. The only post-build source change expanded the integration fixture;
  its old/new hashes and matching recompiled test results are preserved.

## Remaining scope

Broader capture traversal and prefix materialization, oversized activations,
dynamic/unproven helper bodies, first-write insertion, deeper/nonscalar paths and
larger array-row updates remain open. Required snapshot copies are not defects.
Sections 3/4 and later compiled-execution, production-size, platform/release and
clean-host competitive acceptance remain unfinished.

The current refresh still measures 285,038 requested bytes per added VM call
for a deeper dictionary-owned row at width 2,048, versus 6,782 at width 2.
Five-element dictionary-owned row batches request 286,036 bytes per added call
at width 2,048 versus 11,860 at width 32. These initial-only snapshot cases
identify remaining width-dependent costs; fresh-snapshot controls are separate.
