# Remaining root-array update shapes

Accepted runtime `ac7f234` passes 192 allocation observations and 288 semantic
checks across six shapes, widths 8/2,048, once/fresh snapshots and 1/2/16/32 calls.
VM and JIT agree on the following requested bytes per added call with one initial
snapshot. Interpreter behavior is checked separately alongside VM/JIT.

| Shape | Width 8 | Width 2,048 | Width penalty |
| --- | ---: | ---: | ---: |
| Scalar then array, two writes | 6,691 | 284,131 | 277,440 |
| Array then scalar, two writes | 6,691 | 284,131 | 277,440 |
| String first, then two scalars | 6,152 | 283,592 | 277,440 |
| Two scalars, then string | 5,260 | 5,260 | 0 |
| Mixed triple with `offset + n` selectors | 7,441 | 284,881 | 277,440 |
| Same mixed triple with literal selectors | 6,549 | 6,549 | 0 |

The scalar-first promotion fix remains effective. This refresh identifies three
specific remaining cases: mixed two-write storage, initial string batch routing,
and arithmetic selector admission. It does not establish a defect in observable
values: all snapshot and final-value checks pass. Fresh snapshots are retained
separately because detachment can be required by semantics.

Source inspection finds that initial string values pass the bounded-batch value
guard but reach scalar journal preparation unless a collection journal is already
pending. The same string placed after two scalar updates can use promotion.
This routing is the next bounded fix to investigate. Mixed two-write storage and
general selector admission require separate profitability and correctness checks.
No runtime change accompanies this baseline and no elapsed-speed claim follows.

[Evidence](array-shape-refresh-2026-10-30.json) includes raw outputs, source and
artifact hashes, exact fixtures, recomputed slopes and the independent verifier.
