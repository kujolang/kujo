# Nested-write path storage — verified allocation reduction

Runtime `4d1195238008792349a8054815c354f0c075d896` removes temporary path allocations
from ordinary nested writes. The VM borrows opcode shape metadata before dispatch
would clone it. VM and native JIT helpers keep resolved paths of up to four steps
in operation-local inline storage; deeper paths retain heap storage. No values
are pooled or retained between operations. Snapshots, root replacement, evaluation
order, errors and public bytecode contracts are preserved.

The accepted baseline is runtime `b4dfdfd`, with priority diagnosis `b95096e`.
[Full evidence](nested-path-storage-2026-10-28.json) records exact source, executable,
library, allocation probe and benchmark identities, outputs and fixture coverage.

The paired row-copy reproducer removes exactly **two requests and 320 requested
bytes per nested VM write**, at both widths 128 and 2,048. Extracted-row VM controls
are unchanged. Native JIT nested-write traffic also falls; its short-run slopes
include compilation variation and are not presented as exact per-write costs.

Eight unchanged priority test-size workloads passed VM/JIT allocation checks and
interpreter checksum checks. Requested bytes for the largest measured reductions:

| Workload/runtime | Baseline bytes | Candidate bytes | Reduction |
| --- | ---: | ---: | ---: |
| MergeSort VM | 3,826,368,130 | 675,117,229 | 82.356% |
| MergeSort JIT | 2,938,169,199 | 259,573,952 | 91.165% |
| NeuralNet VM | 1,927,896,034 | 1,317,317,492 | 31.671% |
| NeuralNet JIT | 1,528,533,006 | 1,009,421,319 | 33.961% |

These are execution-time requested allocation bytes, including workload setup,
warmup and checksum work, excluding probe parsing. They are neither retained
memory nor elapsed-speed improvements. Profile output includes variable timestamps.
No Goose/Rust ranking or clean-host performance claim follows.

QuickSort initially differed by +1 request/+19 bytes in VM. Brainfuck Array initially
increased by 79 requests/19,461 bytes in JIT. Three additional alternating pairs per
mode retain the original observations and show overlapping ranges and changes in
both directions. No consistently separated increase was established in these four
pairs; this limited evidence does not establish universal absence of regressions.

Verification completed:

- 1,226 library tests passed; eight existing ignores. Actual compiled native-write
  coverage includes depths 2/4/5/8, retained metadata and owned-root cleanup.
- 403 distinct release integration tests passed; one manual profiling test ignored.
- New depth 2–8, mixed-path, snapshot, exception, import and root-replacement regressions
  passed on both baseline and candidate in VM, interpreter and JIT.
- All 150 unchanged test-size LangArena checksums passed: 50 per runtime.
- Negative-operand, nested-publication and cold controls passed, alongside eight
  append comparisons. The reusable target passed 32 observations and rejected the
  unchanged baseline in its negative control.
- Independent assembly verified source/artifact identities, fixture coverage,
  exact checksum configurations, allocation reductions and repeat observations.
  Formatting and diff checks passed.

The repository-maintenance regression gate accepts matching allocation probes:

```sh
ruby benchmarks/langarena/probes/nested-path-allocations.rb \
  /path/to/baseline-probe /path/to/candidate-probe /new/output-directory
```

It generates the same measured fixtures and checks VM allocation improvements and
control nonregression. Companion three-runtime semantic coverage is in
`tests/nested_path_storage.rs`. The gate does not certify wall-clock performance.

Sections 3 and 4 remain open for broader capture materialization, module collection
copying and representation review. Full production sizes, platform/release gates,
and clean-host competitive comparisons remain separate work.
