# Deeper collection publication — 2026-10-28

Runtime `ca3403d` extends deferred scalar publication through deeper mixed collection
paths. Accepted baseline runtime: `a5d07bb`.
[Machine evidence](deep-collection-publication-2026-10-28.json) preserves exact
source/artifact identities, paired allocation measurements, semantic controls and
checksum results. The [baseline isolation](DEEP_COLLECTION_ISOLATION_2026_10_28.md)
remains historical evidence of the reproduced cost.

## Measured allocation results

Requested bytes per added VM helper call, dictionary root, four-access path,
leaf width 2,048 and initial-only snapshot:

| Leaf | Repeated writes | Baseline bytes | Candidate bytes |
| --- | ---: | ---: | ---: |
| Array | 1 | 285,988 | 5,259 |
| Array | 8 | 292,113 | 10,453 |
| Dictionary | 1 | 634,172 | 5,259 |
| Dictionary | 8 | 640,297 | 10,453 |

The primary gate covers 384 allocation pairs and 1,152 combined baseline/candidate
semantic runs. Eligible deeper costs are flat across widths 128/2,048, three root
types and array/dictionary leaves; VM/JIT allocation deltas agree. Both total and
added-call request/byte counts must not regress. Shallow and fresh-snapshot cases
remain controls. Repeated writes target the same leaf key; this does not measure
large distinct-key batches. These are allocation-traffic measurements, not elapsed
speed, retained memory or competitive results against Goose/Rust.

## Implementation and semantics

The existing assignment-path allocation holds the root, intermediate selectors
and existing leaf-write metadata. Admission requires the root to be shared only
with its suspended owner and the traversed intermediate containers/leaf to be
unique. Retained aliases preserve ordinary fallback. No assignment-path or
activation layout expansion is required.

Pure deeper reads match the retained prefix without making a root snapshot or
allocating another path. Effectful or unsupported selectors use ordinary evaluation.
Publication and snapshots apply the path with copy-on-write isolation. Numeric
string keys, negative array aliases, exact scalar bits, callback visibility,
partial writes preceding errors and root replacement retain their behavior.
Existing bounded leaf journals and their overflow rules are preserved.

## Verification

- 1,216 library tests pass; eight existing ignores.
- New component tests cover mixed dictionary/array/struct traversal, path-storage
  reuse, public leaf storage, shared intermediate/leaf rejection, snapshots,
  scalar bits, effectful/different selectors and discard.
- Three permanent release tests pass across VM/interpreter/JIT, each covering
  twelve owner/leaf/depth combinations and independent aliases/snapshots,
  callbacks, errors, read evaluation order and replacement-root writes.
- 512 deep escape/alias allocation pairs and 768 semantic cases pass; unchanged
  baseline calibration also passes exact equality.
- 78 deep diagnostic pairs match exact stdout/stderr/exit, including index effects.
- The main target passes independently recomputed coverage/identities/deltas;
  the unchanged baseline is correctly rejected by 48 improvement/width requirements.
- Shallow single/four/wide dictionary controls, prior fallback/error/branch
  controls and historical capture/array/dictionary controls pass.
- 391 matching-release integration tests pass; one manual profiling test ignored.
- All 150 unchanged test-size checksums pass across VM, interpreter and JIT.
- Independent evidence assembly, formatting and frozen-source checks pass.

## Remaining scope

Larger distinct-key/array-row batches, first-write insertion, nonscalar updates,
shared-owner fallback, general capture traversal/prefix materialization and
oversized activations retain costs. Required snapshot copies are not defects.
Sections 3/4 and later compiled execution, full production, platform/release and
clean-host competitive checks remain open.

## Cutoff for sections 3 and 4

This candidate is complete. The remaining round is broader than its final test
run: captured-environment traversal/name and prefix copying; oversized capture
fallback; larger collection batches/insertion/nonscalar updates; and a final
classification of avoidable copies versus required snapshot detachment.
Each confirmed optimization needs a reproducer, ownership/error tests, matching
allocation comparisons and cross-runtime checks before acceptance. Broad checklist
entries are not equally sized tasks, so checkbox percentages would misrepresent
progress. Several further focused rounds may be needed; no reliable wall-clock
estimate follows from this loaded laptop or the current evidence.

Clean-host speed comparisons and later production/platform work are separate
from this sections 3/4 cutoff. No claim of matching Goose or Rust is made.
