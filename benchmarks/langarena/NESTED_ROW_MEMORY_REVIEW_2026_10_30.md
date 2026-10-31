# Nested-row candidate: acceptance pending

Sections 3 and 4 remain open. This is a verification checkpoint, not an accepted
runtime fix. The candidate is uncommitted; accepted runtime remains `f1893ea`.

The final formatted candidate passed 1,276 library tests (8 ignored), 421 integration
tests (1 ignored), all 150 test-size checksums, 3,048 allocation comparisons,
4,572 semantic checks and 18 boundary checks. Historical and selector controls
passed their independent verifiers. These results do not establish competitive speed.

The original RSS gate covered 288 paired executions in 144 groups. Thirty groups
had positive means; nine increased in both observations. The largest positive
mean was the full-width fresh-snapshot dictionary-row VM case: +1,372,160 bytes
(10.05%). Acceptance remains pending memory review.

A focused follow-up repeated the width-2,048, 2,048-write workload with once/fresh
snapshots, six alternating baseline/candidate pairs per runtime: 36 additional
pairs, all outputs and exits matching. Results remain variable:

| Snapshot | Runtime | Mean RSS change (bytes) | Positive pairs |
| --- | --- | ---: | ---: |
| Once | VM | -17,066.67 | 1/6 |
| Once | Interpreter | -1,510,741.33 | 1/6 |
| Once | JIT | -522,922.67 | 2/6 |
| Fresh | VM | +669,696.00 | 3/6 |
| Fresh | Interpreter | +247,808.00 | 3/6 |
| Fresh | JIT | +535,893.33 | 4/6 |

These positive fresh-snapshot means must not be dismissed as machine load. The
next acceptance step is to isolate live retained memory from allocator/process
variation and explicitly assess the tradeoff before committing the runtime.
[Raw follow-up evidence and verification](nested-row-memory-review-2026-10-30.json)
retain identities, commands, outputs, original group summaries and individual deltas.
The candidate CLI SHA-256 is
`87dd7ffcc199d7cb642408ace58aefd6b81cfb33db681d1ce4e2d32359c7de4b`.

Minimum remaining work groups, including the current one:

1. Resolve memory review and finish acceptance of the nested-row patch.
2. Fix confirmed multiple-key dictionary collection replacement costs and verify.
3. Audit remaining capture/fallback/deeper shapes and verify the combined result.

This is a lower bound, not a promise of three patches. Additional confirmed costs
can require more rounds. Clean-host comparisons and section 5 are separate.
