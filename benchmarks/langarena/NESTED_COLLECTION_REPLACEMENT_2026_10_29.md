# Nested collection replacement

Runtime `2a5ffbc` retains eligible collection replacements in the existing
assignment-path storage. It avoids detaching captured owners solely to replace an
existing array element or dictionary entry, including through deep and mixed paths.
Canonical keys and resolved indices preserve aliases; scalar batching stays separate.

Snapshots and externally shared intermediate owners retain copy-on-write behavior.
Different-key, scalar, invalid and unsupported writes retain ordinary publication
and fallback. The regression contract covers callbacks, returned values, aliases,
errors, type transitions and index expressions that replace the target root.

Requested bytes per added helper call at width 2,048 with an initial snapshot,
measured over 16-to-32 calls:

| Payload | Path | Before | After |
| --- | --- | ---: | ---: |
| array | deep | 635759 | 6303 |
| array | mixed | 634324 | 5997 |
| array | arrayleaf | 284888 | 5644 |
| dictionary | deep | 635925 | 6469 |
| dictionary | mixed | 634490 | 6163 |
| dictionary | arrayleaf | 285054 | 5810 |

Initial-only target slopes are flat across tested widths in VM and JIT. These
measure allocation traffic, not elapsed speed. Real snapshots remain observable.

Verification at this exact revision:

- 1247 library passes; 8 ignored.
- 407 integration passes; 1 manual profiling test ignored.
- All 150 test-size checksums passed across VM, interpreter and JIT.
- 3968 allocation pairs / 5952 associated semantic checks passed.
- 108 paired memory observations / 216 executions verified and reviewed.
- Exact fixture coverage, source/artifact identities, test names/statuses, checksum
  output, allocation totals/slopes, layout guards, formatting and diff checks passed.

Memory assessment: Reviewed all 54 groups / 108 pairs. At width 32,768 without retained snapshots, the candidate used less peak RSS in all 36 paired comparisons; group mean reductions range from 4,368,384 to 11,634,688 bytes. Tiny and retained-snapshot cases include increases: largest group mean +180,224 bytes (2.33%, tiny array/struct interpreter), largest retained-snapshot mean +112,640 bytes. These increases remain recorded, not erased or attributed conclusively to host noise. Two alternating local observations per group cannot establish retained-heap attribution or clean-host memory behavior. Existing layout guards pass and the completed allocation gates show no total/slope regressions. No observed increase in this matrix blocks the bounded change; broader memory claims remain unproven.

[Machine-readable evidence](nested-collection-replacement-2026-10-29.json) preserves
target/fallback observations, memory observations and review, verification receipts,
source identities and the local audit directory with supplemental raw reports.

This accepts eligible nested replacements only. Struct-literal construction,
broader compound-index/captured-scalar batching, remaining capture traversal and
fallback costs, and final collection representation review remain in sections 3/4.
New-key and other unsupported collection paths keep ordinary mutation; this result
does not certify every collection operation as optimized. No production-size,
platform or clean-host performance acceptance follows.
