# Array-owned row publication — 2026-10-27

Runtime `1c3acd427571f1e4964d61991944809af1b7a5ab` extends bounded nested publication to array-owned rows without
adding capture storage or changing value semantics. Positive and negative row
selectors resolve against the root length and coalesce when they identify the
same row. String selectors remain invalid for arrays. The existing four-entry
limit and fallback for larger batches remain in place.

Baseline runtime: `36b8006d1bd3af2883a6f22e6d839efb3c42ea11`.
[Machine evidence](array-row-publication-2026-10-27.json) contains matching artifacts,
source/fixture identities, raw results, deltas and verification scripts. These are
local allocation measurements, not elapsed-speed or peak-memory claims.

## Allocation results

The target covers positive, negative-literal and negative-binding row selectors;
widths 32/128/2,048; 1/2/3/4/5/16 distinct writes; initial-only/fresh-per-call
snapshots; and 16/32 calls. All 432 allocation pairs and 648 semantic cases pass.
One-to-four-entry costs stay flat from width 128 to 2,048, improve against the
baseline and agree between VM and JIT. No total or added-call allocation regression
remains in the target matrix. Larger batches retain ordinary materialization.

Requested bytes per added helper call at width 2,048, initial snapshot retained:

| Distinct writes | Baseline | Candidate |
| --- | ---: | ---: |
| 1 | 285405.0 | 6525.0 |
| 2 | 285742.0 | 6862.0 |
| 3 | 286079.0 | 7199.0 |
| 4 | 286416.0 | 7536.0 |
| 5 | 286753.0 | 286753.0 |
| 16 | 290460.0 | 290460.0 |

Supplemental controls cover root/wrapped/row returns, snapshots before another
write, other-row updates, shared rows, nonscalar values and invalid index operands.
All 192 paired allocation and 288 semantic cases pass. Fresh snapshots and actual
shared rows preserve required copy-on-write detachment.

## Rejected draft and correction

The first candidate passed the ordinary target but regressed eight invalid-index
allocation cases. In `state[state][0]`, the generic indexed-read fast path looked
up the pending root as a possible scalar index, creating a speculative snapshot
before ordinary evaluation materialized it again. Repeating the rejected binary
reproduced the exact excess counts in VM and JIT.

The final candidate excludes that impossible scalar-index lookup: a pending root
is a collection. Ordinary evaluation still determines the error and publication
order. All eight failing cases now pass the unchanged fallback gate. Both the
rejected measurements and reproduction are preserved in the machine evidence.

## Verification

- 1,192 library tests pass; eight existing tests remain ignored.
- Nine targeted integrations pass across VM, interpreter and JIT, including
  aliases, snapshots, shared rows, overflow batches, root replacement and errors.
- All 361 matching-release integrations pass; one manual profile remains ignored.
- All 18 baseline/candidate nested-assignment parity cases pass.
- All 30 invalid-read diagnostic pairs match exact stdout, stderr and exit status
  across VM, interpreter and JIT, with matching fixture identities.
- New array target/fallback controls pass 624 allocation pairs and 936 semantic cases.
- Existing row, scalar dictionary, dictionary escape and invalid-operand matrices
  pass another 696 allocation pairs and 1,044 semantic cases.
- All historical capture, dictionary-read/key, module, nested, cold, append and
  publication-boundary controls pass with their recorded nonregression requirements.
- All 150 unchanged test-size checksums pass: 50 each in VM, interpreter and JIT.
- Formatting, diff, frozen source, benchmark and artifact-identity gates pass.

The independent assembler checks exact target coverage, source/fixture identities,
recomputed deltas, actual and synthetic unchanged-baseline rejection, diagnostic
parity and the retained rejected-candidate reproduction. Control mode for the
already accepted scalar fix preserves nonregression and flat-width requirements;
its default mode still demands improvement.

## Remaining scope

Multiple dictionary keys, nested dictionaries, deeper paths, larger/dense or escaping
updates and broader captured-environment costs remain open in the
[checklist](PERFORMANCE_CHECKLIST.md). Full production-size completion, platform
acceptance and clean-host Goose/Rust comparisons are separate. No Kostya contact occurred.
