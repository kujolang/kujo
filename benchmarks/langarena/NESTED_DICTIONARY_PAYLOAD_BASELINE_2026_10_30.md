# Nested dictionary collection payloads: remaining baseline cost

Accepted runtime `b1bf786` passes 144 allocation observations and 216 semantic
runs across VM, interpreter and JIT. Two array replacements and a three-write
scalar/array/dictionary mix are tested through dictionary, struct-field and deeper
dictionary owners. Leaf widths are 8 and 2,048; call counts are 1, 8 and 16.
Initial retained snapshots and fresh-per-call snapshots are separate controls.

Requested bytes per added call with one initial retained snapshot, using the
8-to-16-call slope, agree between VM and JIT:

| Owner / payload | Width 8 | Width 2,048 | Width penalty |
| --- | ---: | ---: | ---: |
| dict / arrays | 9786 | 634026 | 624240 |
| dict / mixed | 10325 | 634565 | 624240 |
| struct / arrays | 9759 | 633999 | 624240 |
| struct / mixed | 10256 | 634496 | 624240 |
| deep-dict / arrays | 10822 | 635062 | 624240 |
| deep-dict / mixed | 11541 | 635781 | 624240 |

Every fixture verifies the full original leaf, final leaf length, all changed
entries and all untouched final entries. Fresh snapshots verify changed entries
from the preceding call. This is a remaining allocation cost, not a correctness
failure or an elapsed-time comparison.

The root-dictionary candidate currently under verification has not been measured
in this report. Recheck these exact fixtures on its accepted artifact before
implementing the nested-owner follow-up. Its root publication path must not be
assumed to cover nested dictionary leaves. Deeper/escaping review remains open;
this report does not close sections 3 or 4 or make an all-workloads claim.

[Raw results, fixtures and verification](nested-dictionary-payload-baseline-2026-10-30.json)
preserve binary/probe/source identities, exact coverage, stdout, allocation
counters and per-call slopes. No Kostya contact occurred.
