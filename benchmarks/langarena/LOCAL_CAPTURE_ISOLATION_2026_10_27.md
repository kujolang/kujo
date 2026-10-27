# Retained local capture isolation — 2026-10-27

The accepted `2b83f1e` runtime has a confirmed oversized local-capture allocation
cost. This is baseline characterization, independent of the foreign-callback
optimization verified separately in [foreign global transfer](FOREIGN_GLOBAL_TRANSFER_2026_10_27.md).

The [evidence](local-capture-isolation-2026-10-27.json) contains artifact identities,
fixture sources, raw rows and per-call deltas. All 32 allocation controls in
VM/JIT and 48 semantic controls in VM/interpreter/JIT passed.

| Retained extra locals | Read requests / bytes per call | Write requests / bytes per call |
| ---: | ---: | ---: |
| 0 | 25 / 1,266 | 26 / 1,277 |
| 128 | 25 / 1,266 | 26 / 1,277 |
| 2,048 | 25 / 1,266 | 26 / 1,277 |
| 8,192 | 8,233 / 5,357,679 | 8,234 / 5,357,690 |

VM and JIT counts agree. These are allocation traffic, not elapsed speed or peak
memory. The measured jump is consistent with the bounded capture-storage pool's
oversized fallback; it does not establish the exact threshold or CPU attribution.

The closure references every extra local in a flag-controlled branch. Repeated
calls skip that branch, isolating setup from work proportional to the retained
values. One final alias call executes the branch and validates the sum of all
retained values. Subtracting 16-call from 32-call runs removes that fixed work.
An earlier exploratory control with unused locals stayed flat even at 8,192:
merely declaring many locals does not prove a closure retains them.

The fixtures also check the documented identity rules: separately created sibling
closures retain independent snapshots, while aliases of one closure share its
cells. No language semantic change is proposed.

Reproduce characterization with `ruby probes/local-capture-allocations.rb
MATCHING_PROBE MATCHING_BINARY NEW_OUTPUT_DIR` from this directory. The current
runner additionally includes one-call controls and accepts an optional
`BASELINE_PROBE` argument for paired allocation regression checks at identical
fixture paths. Without that argument it reports `characterized`.

The [fresh-map reserve follow-up](LOCAL_MAP_RESERVE_2026_10_27.md) isolates repeated
table growth from retained-name copying. General oversized materialization
remains open. Further changes must preserve bounded idle storage, live aliases,
host-owner visibility, task ordering, snapshots and error behavior. Increasing
the pool budget alone would move the boundary and is not a general fix.
