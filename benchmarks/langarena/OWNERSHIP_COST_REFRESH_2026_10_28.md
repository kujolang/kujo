# Ownership costs after the nested-write fix

Read-only diagnosis on accepted runtime `4d11952`, with exact artifact identities
and independently recomputed coverage/deltas in
[the evidence](ownership-cost-refresh-2026-10-28.json). No runtime change or new
optimization acceptance is claimed by this refresh.

All ten historical small-update collection shapes now have constant added-call
allocation traffic across widths 2, 128 and 2,048 when retaining only the initial
snapshot. These include integer/float dictionary updates, two-key updates,
struct/dictionary/array-owned rows, deeper rows and nested dictionary leaves.
VM and JIT agree. The matrix passed 240 allocation observations and 360 semantic
checks, including separate fresh-snapshot controls.

This closes the diagnosis gap caused by citing old, already-fixed small-update
examples in the remaining-work description. It does not cover insertion,
nonscalar updates, general/deeper loops, arbitrary large batches or escaping
ownership. Fresh-snapshot cases preserve independent values; their width-dependent
traffic alone is not evidence of an avoidable copy.

Oversized captured-local setup still reproduces on this same runtime:

| Operation | Retained bindings | Requests per added call | Requested bytes per added call |
| --- | ---: | ---: | ---: |
| Read | 8,192 | 31 | 311,959 |
| Write | 8,192 | 32 | 311,970 |

Both VM and JIT produce these deltas. The capture matrix passed 48 allocation
observations and 72 semantic checks, covering 0/128/2,048/8,192 retained bindings,
read/write helpers, independent closures and shared aliases. This is the next
confirmed allocation target. The source still reconstructs oversized private
binding metadata on activation; any reuse must preserve membership and binding
kinds when host code changes public owner maps.

These figures measure requested allocation traffic, not elapsed speed or retained
memory. Sections 3 and 4 remain open for the broader cases above, capture
materialization/traversal and representation review. Production-size, platform
and clean-host comparisons remain separate gates.
