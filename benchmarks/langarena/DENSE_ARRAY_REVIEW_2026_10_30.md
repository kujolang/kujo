# Dense captured-array update review

Accepted runtime `5458132`. Counted helpers replace one quarter or all of a
128/2,048-element captured array with one-element array payloads. The caller
retains either one original snapshot or a fresh snapshot for every call.

48 allocation observations and 72 semantic checks pass, covering VM/JIT
allocation counters and VM/interpreter/JIT output assertions. Checks validate
all original, final and untouched cells, plus fresh snapshot preservation.
Independent verification checks exact matrix coverage, raw counters, source and
artifact identities, outputs and recomputed slopes.

| Owner width | Writes/call | Once snapshot bytes/call | Fresh snapshot bytes/call |
| --- | ---: | ---: | ---: |
| 128 | 32 | 27,665 | 49,734 |
| 128 | 128 | 91,697 | 105,894 |
| 2,048 | 512 | 347,825 | 591,654 |
| 2,048 | 2,048 | 1,372,337 | 1,490,214 |

Slopes use 8/16 calls; one-call controls are retained. VM/JIT values agree.
Every measured once-snapshot point exactly fits `6,321 + 667 × writes` bytes
and `67 + 3 × writes` allocation requests. There is no owner-width term in
these measured points. Fresh-snapshot points fit
`13,606 + 136 × owner_width + 585 × writes` bytes and
`130 + 3 × writes` requests. This linear model describes the measured cases;
it is not a universal complexity proof.

Disposition: no additional owner-width-copying fix is demonstrated for these
dense root-array counted updates. Fresh snapshots must preserve old values;
their separate copy cost remains explicit. Per-written-payload allocation is
still present, so these results do not establish globally optimal representation.
Dictionary/struct paths, deeper loops, escaping updates and broader capture
materialization remain separate audit items. Sections 3/4 remain open.

No elapsed-speed, production-size or clean-host comparison claim follows.
[Raw evidence and reproduction](DENSE_ARRAY_REVIEW_2026_10_30.json).
