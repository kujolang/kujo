# Recursive global transfer: verified allocation reduction

Runtime `2b83f1e` transfers the private global map through direct synchronous
reentry with a global-only capture. It reuses bounded scope storage rather than
copying unrelated global bindings into each recursive activation. The baseline
is `c79010d`. [Machine-readable evidence](recursive-global-transfer-2026-10-26.json)
retains matching build identities, fixtures, rejected-draft results and all gates.

## Measured allocation traffic

VM requests and requested bytes per added outer call, with 2,048 unrelated
bindings:

| Depth / operation | Baseline requests / bytes | Candidate requests / bytes |
| --- | ---: | ---: |
| 0 / read | 42 / 1,555 | 42 / 1,555 |
| 0 / scalar write | 43 / 1,547 | 43 / 1,547 |
| 1 / read | 82 / 4,399 | 68 / 4,266 |
| 1 / scalar write | 98 / 4,510 | 84 / 4,377 |
| 4 / read | 6,397 / 2,050,876 | 148 / 12,783 |
| 4 / scalar write | 6,458 / 2,051,344 | 209 / 13,251 |

Candidate depth-one and depth-four setup is flat across 0, 128 and 2,048
unrelated bindings in both VM and JIT entry paths. All 36 paired comparisons
have no increase in requests or bytes. The separate cold/oversized matrix also
passes all 36 paired comparisons. Raw JIT results remain in the JSON.

Identical-path append controls at 512, 1,024, 2,048 and 4,096 elements show zero
change in requests or bytes in VM and JIT. Comparing different fixture directory
lengths produced a misleading fixed 28-byte difference; the paired controls
remove that source-path storage difference.

These are allocation measurements, not elapsed-speed or peak-memory results.
Flat allocation does not prove constant CPU cost: publishing changed globals
still traverses the owner map. Module collection-width copying remains open.

## Semantics and rejected draft

Only immediate synchronous reentry with a one-scope captured prefix transfers
globals. Captured locals, tasks and indirect reentry retain their existing setup.
The public owner remains accessible and contains real values. Inner returns
publish inherited outer writes at the established boundary, and escaped arrays
retain their independent snapshots. No owner lock is held across user code.
Live-capture, write-journal and observation metadata are retained; pool bounds
remain unchanged.

The first draft passed the correctness suite and depth-scaling gate but increased
depth-one requested bytes by 347 per outer call. It was rejected before commit.
The revised implementation reuses scope-vector capacity instead of allocating
fresh vectors on each recursive entry. A strengthened paired gate reproduces
the rejected draft's increase in 12 of 36 comparisons and passes the accepted
implementation. At depth one, both tested entry paths now save 14 requests and
133 bytes per outer call for read and write cases.

## Verification

- 1,146 library tests passed; eight ignored tests are inventoried in the evidence.
- 295 integration tests passed; one manual profiling test remains ignored.
- Six host-owner tests cover VM/interpreter/JIT, including recursive publication,
  callback lock accessibility and old-array snapshots.
- Eighteen recursive boundary controls passed on baseline and matching release:
  errors after writes, computed callees, callbacks, immutable errors, saved
  returns and caller-parameter isolation.
- Explicit interpreter task/field ordering checks passed.
- All 150 test-size checksums passed: 50 each in VM, interpreter and JIT.
- The recursive gate passed 72 candidate and 72 baseline allocation controls,
  108 semantic controls and 36 per-call comparisons. Imported, captured-read,
  oversized-read, cold, module-owner and module-write boundary gates also passed.
- Matching release/library/probe identities and 819 frozen source hashes verified;
  formatting and diff checks passed.

The tracked `probes/recursive-helper-allocations.rb` takes the candidate probe,
candidate binary and a new output directory. Supply its optional fourth argument,
the matching baseline probe, to enforce the paired no-regression gate. Both probes
run against the same fixture paths; compare matching build configurations.

General and indirect capture materialization, publication traversal, module
collection detachment and the remaining plan are still open. No production-size,
platform/release or clean-server Goose/Rust acceptance is claimed. Loaded-laptop
timings remain diagnostic only.
