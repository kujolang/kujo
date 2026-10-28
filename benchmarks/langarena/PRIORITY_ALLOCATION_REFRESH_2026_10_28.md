# Priority allocation refresh — 2026-10-28

Runtime `b4dfdfd`, compared with the preceding accepted runtime `c83ee4b`.
This is a structural diagnosis on the busy local machine, not a Goose/Rust speed
comparison. The eight priority workloads use unchanged port algorithms and test
configuration. Only the argument binding is injected for the allocation probe.
All 32 probe executions and eight interpreter fixture checks passed their checksums.
[Full evidence](priority-allocation-refresh-2026-10-28.json).

## Current allocation traffic

These totals include execution setup, warmup and checksum work, excluding source
parsing/bytecode compilation. They are requested bytes, not retained memory.
Different cases have different inputs and work counts; this table is not an
algorithm-efficiency ranking. MB means 1,000,000 bytes.

| Workload | VM requested MB | JIT requested MB |
| --- | ---: | ---: |
| QuickSort | 488.9 | 111.1 |
| MergeSort | 3,826.4 | 2,938.2 |
| NeuralNet | 1,927.9 | 1,528.2 |
| CacheSimulation | 520.4 | 564.6 |
| GameOfLife | 11.3 | 66.0 |
| TextRaytracer | 800.8 | 464.2 |
| Brainfuck Array | 30.0 | 94.5 |
| Brainfuck Recursion | 31.3 | 76.0 |

The compact captured-name optimization does not establish a broad reduction in
these workload totals. Its verified oversized-capture benefit remains valid, but
substantial work remains in ordinary benchmark execution.

QuickSort JIT initially showed 102 more requests and 20,751 more bytes. Three
additional alternating pairs produced overlapping baseline/candidate ranges and
both directions of change. That initial increase did not reproduce consistently.
Other small differences have not been individually classified by repeat runs.
The repeat driver needed a local variable-scope correction before its first
measurement; its empty failed result is preserved in the evidence.

## Next confirmed target: nested indexed writes

A separate diagnostic copies rows using either `state[1][i] = state[0][i]` or
extracted `src`/`dst` arrays. It holds the original snapshot and verifies every
result element. At widths 128 and 2,048, subtracting two-round from four-round
allocation totals isolates **two additional requests and 320 additional bytes
per nested write in VM mode**. All 16 allocation observations and 24 semantic
runs passed; this is not a rewrite of the benchmark port.

The VM clones each opcode, including its path-shape vector, then allocates a
resolved access vector in `UpdateBindingPath`/`UpdatePath`. Those are concrete
implementation candidates for the measured overhead; an implementation experiment
must establish causality and verify the reduction. JIT micro deltas include
variable compilation traffic and are not used to claim an exact per-write cost.

## Bounded next work

- [ ] Reduce demonstrated temporary path storage/cloning in ordinary nested writes.
- [ ] Preserve snapshots, index evaluation order/root replacement, mixed field/index
  paths, negative indices, error behavior and deep-path fallback in all runtimes.
- [ ] Verify the micro reduction, unchanged cold/fallback costs, and actual
  MergeSort/QuickSort allocation results before accepting the change.
- [ ] Revisit the remaining high-allocation priority paths after that result;
  their totals alone do not identify which allocations are avoidable.

Capture traversal/slab copying, broader environment materialization and dense,
escaping or nonscalar collection paths remain separate sections 3/4 review items.
Required snapshot copying is not itself a defect. Compiled-execution expansion,
production-size completion, platform gates and clean-host comparisons remain open.
