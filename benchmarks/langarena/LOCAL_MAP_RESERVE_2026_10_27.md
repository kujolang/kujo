# Fresh local capture-map allocation — 2026-10-27

Fresh private views of captured local scopes now reserve table storage once from
the number of actual keys. Previously, inserting the same keys grew the table
repeatedly. Public owner values, live alias reads, independent snapshots and idle
pool limits are unchanged. Existing reused maps keep their previous behavior.

This is an incremental reduction in oversized capture setup. It does not remove
retained-name copying or make oversized setup constant in the number of names.

## Measured allocation change

Matching normal-release artifacts were compared at identical fixture paths.
Each closure genuinely retains every declared extra local; a final flag-controlled
call validates their values. Separate sibling closures retain independent state,
while aliases share the chosen closure's cells. Per-call results subtract the
16-call total from the 32-call total. One-call cases protect cold setup too.

| Extra retained locals | Operation | Before requests / bytes per added call | After requests / bytes per added call |
| ---: | --- | ---: | ---: |
| 0 / 128 / 2,048 | Read | 25 / 1,266 | 25 / 1,266 |
| 0 / 128 / 2,048 | Write | 26 / 1,277 | 26 / 1,277 |
| 8,192 | Read | 8,233 / 5,357,679 | 8,221 / 2,720,307 |
| 8,192 | Write | 8,234 / 5,357,690 | 8,222 / 2,720,318 |

VM and JIT agree. At 8,192 locals the reduction is 12 allocation requests and
2,637,372 requested bytes per added call, approximately 49% fewer bytes. All 48
paired totals and 16 per-call comparisons avoid allocation regressions. All 72
associated VM/interpreter/JIT semantic controls pass.

These counts measure allocation traffic, not retained memory or elapsed speed.
The loaded laptop cannot establish reliable competitive speed gains. Clean-server
Kujo/Goose/Rust measurements remain pending.

## Component isolation

A counting-allocator probe uses the actual 136-byte `Value` and clones the same
names into `Value::Null` private entries. With 8,192 entries, repeated table growth
requests 5,356,022 bytes versus 2,718,650 with one reserve; both finish with the
same capacity. The 2,637,372-byte difference matches the runtime measurement.
Twenty dense/sparse controls cover 0, 128, 2,048, 8,192 and 32,768 entries. Reserving
from key count avoids inheriting a sparse owner's spare buckets.

The unit regression passes before and after the change. It checks live owner
updates, independent detached snapshots, immutable bindings, public collection
ownership, sparse owner capacity and oversized pool eviction. Thus it protects
semantics rather than asserting the new allocation implementation alone.

## Reproduction and limits

Run `ruby probes/local-capture-allocations.rb MATCHING_PROBE MATCHING_BINARY
NEW_OUTPUT_DIR BASELINE_PROBE` from this directory. This compares both executables
at each identical fixture path and rejects any increased total or per-call
allocation count or bytes. The acceptance pipeline also requires a strict
reduction at 8,192 locals. Without a baseline argument the runner reports
characterization only.

Remaining work includes retained-name copying, general captured-prefix
materialization, publication traversal and mutable module collection-width
copying. Pool limits have not been raised to conceal the oversized boundary.
Production-size benchmarks, platform/release gates and clean-host comparisons
remain separate checklist items.

## Verification

Runtime commit `aee15a2c99b1ef1cea9a4497bef682559cf011da` passed:

- 1,147 library tests; eight profiling/manual tests ignored.
- 301 integration tests; one manual profiling test ignored. Explicit interpreter
  ordering checks also passed.
- All 150 test-size checksums: the exact 50 input configurations in each of VM,
  interpreter and JIT, with source/input identities checked before and after.
- 48 paired local allocation cases and 72 semantic cases. Reversing the probes
  rejects the old runtime for allocation regression after its 72 semantic cases
  pass, confirming the gate detects the old cost.
- All 36 cold/oversized comparisons and eight append comparisons match baseline
  request/byte totals exactly. Direct/shared/foreign recursion, imported helpers,
  captured reads and module-write boundary controls pass.
- 128 module allocation controls and 192 module semantic controls pass.
- Formatting, diff checks, 821 runtime/test/runner source hashes and 58 benchmark
  source/input hashes pass.

The [machine-readable evidence](local-map-reserve-2026-10-27.json) preserves
candidate/library identities, source hashes, component probe source/results,
paired local rows, rejected-baseline comparisons and verification summaries.
The baseline is the accepted `0cd6ed6` runtime archived at documentation commit
`d752761`; comparison binaries are preserved independently of the current build.
