# Recursive capture storage verification

Runtime: `c0e45b3`, compared with accepted runtime `6228389`. The [machine-readable report](recursive-capture-pool-2026-10-26.json) preserves build/source identities, fixtures, allocation counts, semantic results and the rejected first draft.

Recursive helpers now reuse bounded private capture-prefix storage and refill global maps without repeatedly allocating unchanged keys. Parameter scopes remain separate. Fresh dense maps use bulk cloning; sparse maps reserve only the required entries instead of inheriting unused buckets. Pool limits and public-owner publication boundaries are unchanged.

## Measured allocation changes

These are requested allocation bytes and requests per added outer call, computed from 16/32-call controls with 2,048 unrelated bindings. They are not elapsed-time or peak-memory measurements.

| VM operation | Recursion depth | Baseline requests / bytes | Candidate requests / bytes |
| --- | ---: | ---: | ---: |
| Read | 0 | 42 / 1,555 | 42 / 1,555 |
| Scalar write | 0 | 43 / 1,547 | 43 / 1,547 |
| Read | 1 | 2,149 / 683,810 | 82 / 4,399 |
| Scalar write | 1 | 4,227 / 703,380 | 98 / 4,510 |
| Read | 4 | 8,470 / 2,730,575 | 6,397 / 2,050,876 |
| Scalar write | 4 | 16,779 / 2,808,879 | 6,458 / 2,051,344 |

Depth-one candidate setup is identical across 0/128/2,048 unrelated bindings in both VM and JIT. Depth four can exceed the bounded idle pool and still materializes maps; this is not universal constant-cost recursion.

The first draft increased allocation traffic for owners too large to retain. It was rejected, not shipped. The paired cold gate detects that draft in 24 of 36 comparisons. The final candidate reduces both requests and bytes in all 36 comparisons: 8,192 unrelated bindings, read/write operations, depths 0/1/4, and 1/16/32 calls, through both VM and JIT. Three-runtime semantic checks also pass. First-call and sustained oversized behavior are explicitly covered.

Append controls at 512/1,024/2,048/4,096 operations retain linear growth, with a constant reduction of two requests and 1,852 requested bytes versus the accepted baseline in VM and JIT.

## Verification

- 1,142 default-feature library tests passed; eight ignored tests are inventoried in the report.
- 289 distinct matching-release integration tests passed; one manual profiling test remains ignored. Explicit interpreter task/field ordering checks also passed.
- All 150 unchanged test-size checksums passed: 50 each in VM, interpreter and JIT.
- Recursive helper gate: 72 allocation controls and 108 semantic controls passed; the original baseline fails the depth-one allocation invariant.
- Cold/oversized gate: 72 paired allocation controls and 54 semantic controls passed, including the regression-detection run against the rejected draft.
- Existing imported and captured-read gates passed: 32 allocation controls each, with 48 semantic controls each (imported semantics recorded separately).
- Module-owner controls passed: 128 allocation and 192 semantic controls. Additional boundary fixtures preserve completed writes before errors, read-after-write indexing and escaped aliases in all three runtimes.
- Formatting and diff checks passed. Source hashes remained fixed throughout verification.

## Remaining work

Map traversal, deeper/oversized snapshot materialization and module collection-width copying remain open. This change does not introduce a collection representation or bypass public-owner snapshots. The tests preserve real owner values, callback accessibility, recursive parameter isolation, binding permissions, sparse-map behavior and value semantics.

The laptop is variably loaded. No competitive speed claim follows from these allocation reductions. Full production-size verification, later compiled-execution work, platform/release acceptance and clean-server Goose/Rust comparisons remain pending under the full performance plan.
