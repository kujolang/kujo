# Row overflow isolation — 2026-10-28

The accepted `ca3403d` runtime still copies wide rows when a helper updates more
than four distinct elements. Both explicit assignments and actual compiler-emitted
loops reproduce the cost. This is a baseline for the next fix, not a new runtime
optimization or elapsed-speed claim.

[Machine evidence](row-overflow-isolation-2026-10-28.json) contains the exact
artifacts, generated sources, measurements, build arguments and verification logs.

## Measurements

Requested bytes per added call, dictionary owner, initial-only snapshot, VM:

| Distinct writes | Source form | Width 128 | Width 2,048 |
| ---: | --- | ---: | ---: |
| 4 | Loop | 6,326 | 6,326 |
| 4 | Explicit | 6,320 | 6,320 |
| 5 | Loop | 24,916 | 286,036 |
| 5 | Explicit | 24,909 | 286,029 |
| 16 | Loop | 29,569 | 290,689 |
| 16 | Explicit | 29,551 | 290,671 |

The wider-row penalty is 261,120 requested bytes per added call in each overflow
case. The 4-entry inline control stays flat. VM/JIT deltas agree across the full
matrix; loop bookkeeping does not explain the width-dependent copy.

## Verification and boundaries

- 288 same-build paired allocation measurements across struct/dictionary/array
  roots, widths 128/2,048, batches 4/5/16, loop/explicit bodies, initial/fresh
  snapshots and 16/32 calls. Every repeated measurement matches exactly.
- 432 VM/interpreter/JIT semantic cases pass. Fresh-snapshot copies are separate
  controls; they must not be removed merely to improve allocations.
- Independent verification checks matrix coverage, artifacts, fixture/source
  identity, recalculated deltas, VM/JIT agreement and reproduced overflow cost.
- Three permanent release regression tests cover 54 owner/batch/source/runtime
  combinations, including batches 64. They check all 128 elements, untouched
  elements, retained leaf aliases, fresh snapshots, callback visibility, writes
  before an out-of-bounds error and replacement-root assignment semantics.

The implementation currently stores four row entries inline. Unlike dictionary
writes, rows have no overflow representation. The existing dictionary profitability
guard also rejects local mutation and loops and requires a dictionary leaf.
Therefore adding row storage alone, or only admitting explicit assignments, does
not resolve the whole reproduced workload.

Next work must preserve the inline allocation/layout behavior, extend storage for
larger row batches, and establish safe profitable admission for both measured
source forms. Recheck snapshot/callback/error/alias fallback costs before accepting
it; an unconditional spill may add work to paths that still need a real copy.
No runtime changes were made for this isolation. Sections 3/4 remain open.
