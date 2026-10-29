# Struct literal publication

Runtime `3ac637e` preserves collection-write admission while evaluating a
struct literal. Each field still uses ordinary expression evaluation: callbacks,
owner reads, field ordering and errors retain their publication boundaries.
This removes the unnecessary outer flush before eligible collection replacement.
Nested writes transfer owned payloads through publication; reads and snapshots
retain the copies needed for value semantics.

Requested bytes per added helper call, width 2,048, initial retained snapshot,
16-to-32 calls, VM:

| Owner | Payload form | Before | After |
| --- | --- | ---: | ---: |
| root | literal | 640407 | 5448 |
| root | parameter | 5919 | 5919 |
| nested | literal | 642273 | 5977 |
| nested | parameter | 6456 | 6448 |

VM and JIT initial-only target slopes are flat across tested widths. Argument
payloads are controls; their setup differs from literal construction. Compare
forms against their own baseline. These are allocation results, not elapsed speed.

Verified against accepted nested-replacement runtime `2a5ffbc`:

- 1249 library passes; 8 ignored.
- 408 integration passes; 1 ignored.
- All 150 test-size checksums across VM, interpreter and JIT.
- 864 allocation pairs / 1296 associated semantic checks, with no total or slope regressions.
- 72 memory pairs / 144 executions reviewed.
- Exact source/artifact identities, test sets, fixture coverage, layouts, formatting and diff checks.

Memory assessment: All 72 pairs preserve outputs and snapshots. At width 32,768 without retained snapshots, all 18 paired observations for root dictionaries and nested array/dictionary leaves decrease peak RSS; group means decrease by 5,699,584 to 11,819,008 bytes. Direct root-array collection replacement is an unchanged fallback, with mixed RSS differences; separate allocation isolation confirms its width cost remains. Across all groups, the largest mean increase is 215,040 bytes (0.45%, root array), and the largest percentage increase is 1.97% (145,408 bytes, tiny root array). Retained-snapshot group mean increases are at most 65,536 bytes. These are two alternating local observations per group, not retained-heap attribution or clean-host acceptance. No new cache, pool or publication-layout growth was introduced; owned payload transfer preserves snapshot copies.

[Raw results and verification receipts](struct-literal-publication-2026-10-29.json)
preserve the observations and local audit provenance. Compound indexed arithmetic,
captured-scalar batching, broader capture costs, unsupported collection paths and
final representation review remain open. No clean-host speed, production-size or
platform acceptance follows from this change.
