# JIT compilation economics: baseline entry evidence

All **64 diagnostic outcomes** pass on the corrected frozen baseline
`bf5bd666143cabb2339a72a5635702a6c735639e25d12663396aea7a87abaed4`
(runtime `5fe52fc5`, evidence base `f12c2e3e`). No runtime policy changed.
`collect-jit-economics.rb` records raw outcomes and existing measurement counters;
`verify-jit-economics.rb` checks exact coverage, invocations, configurations,
outputs, raw measurement hashes and frozen source/binary identities.

Each of 16 cells runs in VM, interpreter, JIT, and function-JIT-disabled mode
with loop JIT retained. Interpreter runs omit unsupported measurements. There
is one observation per cell: these are structural/cost diagnostics, not paired
speed measurements or a clean-host comparison.

## Late tree compilation reproduced

The unchanged official object/arena tree kernels run at depth four with one
warmup and 1, 3 or 30 measured iterations. Only diagnostic configuration changes;
checksums are derived as 31 nodes times warmup plus measured iterations.

| Kernel | Iterations | Compile entries | Native function entries | Inclusive compile wall ms |
| --- | ---: | ---: | ---: | ---: |
| Object | 1 | 1 | 0 | 0.002 |
| Object | 3 | 4 | 50 | 37.184 |
| Object | 30 | 4 | 1,724 | 36.229 |
| Arena | 1 | 1 | 0 | 0.003 |
| Arena | 3 | 3 | 50 | 68.859 |
| Arena | 30 | 3 | 1,724 | 71.204 |

The two recursive functions each cross the current 100-call admission threshold
near the end of the three-iteration case. Counts distinguish the late transition
from repeated recompilation at 30 iterations. Compile entries include attempts
and generated variants, not unique successfully compiled source functions; the
one-entry case has no native function execution. Function-JIT-disabled tree
controls record zero compile entries at all three iteration counts. Disabling
that path changes execution too, so subtracting elapsed times would not isolate
compiler CPU or prove a new policy's benefit.

## Equal kernel work exposes per-job compilation

The worker fixture executes 16 jobs per repetition; each performs the same
2,000-term integer sum. Every individual result, total and input snapshot is
asserted. Concurrency varies independently of job count. Four repetitions mean
64 jobs, not larger jobs.

| Mode | Jobs | Concurrency | JIT compile entries | Native function entries |
| --- | ---: | --- | ---: | ---: |
| Direct | 16 | 1 | 1 | 16 |
| Bytecode parallel_map | 16 | 1, 4, 8, 16 | 16 in each cell | 16 in each cell |
| Direct | 64 | 1 | 1 | 64 |
| Bytecode parallel_map | 64 | 1, 4, 8, 16 | 64 in each cell | 64 in each cell |

This matches `try_parallel_map_with_bytecode` calling `execute_bytecode_mapper`
per element in `src/interpreter/native_functions/async_ops.rs`; each invocation
creates a fresh `VM::with_task_policy`, environment and wrapper. The number of
compiler entries follows job count, not the concurrency limit.

**Delaying whole-function compilation alone will not remove this duplication.**
Function-JIT-disabled controls still compile once per parallel job through loop
JIT. The 64-job/concurrency-four control records 64 cache misses, zero hits and
64 native boundaries; direct 16-job execution records one miss, 15 hits and
16 native boundaries. Native function entries are zero in these loop-only runs.

The collector's scheduler task counters are zero for this bytecode mapper path;
they must not be interpreted as proof that no worker jobs ran. Fixture job counts
and native/compile evidence establish the scope above.

## Boundaries and next experiment

Inclusive compile wall spans can overlap across workers and exceed process wall
time. They exclude metadata retention before the instrumented compilation
boundary. They are neither CPU time nor a valid percentage of elapsed time.
Host variation remains uncorrected. No Rust/Goose performance score is inferred.

These controls cover uncaptured bytecode callbacks. They do not establish safe
VM reuse, shared generated-code ownership, async-mapper behavior, imported or
captured callback behavior, cancellation/error cleanup, or capability isolation.
In particular, LangArena Matmul uses an async mapper; its path needs separate
confirmation before transferring this implementation diagnosis to it.

Next gates before a runtime candidate:

- [x] Reproduce late compilation and fixed-work per-job duplication on corrected baseline.
- [ ] Add call-count/body-size controls around admission and confirm long hot execution.
- [ ] Trace the async mapper route and measure its distinct worker/compiler lifetimes.
- [ ] Attribute compiler/cache costs separately from callback setup and value transfer.
- [ ] Choose one general policy or reuse tactic with explicit failure criteria.
- [ ] Prove state, snapshots, capability, cancellation and code-lifetime isolation.
- [ ] Require all-runtime correctness, all-50 checksums and repeated unchanged-workload gains.

Do not simply disable JIT, raise a threshold to fit one benchmark, or share live
mutable environments. The separate code-lifetime candidate remains excluded.

Reproduce from the repository root with:

```text
ruby benchmarks/langarena/probes/collect-jit-economics.rb FROZEN_BASELINE NEW_OUTPUT
ruby benchmarks/langarena/probes/verify-jit-economics.rb NEW_OUTPUT
```

The adjacent `jit-economics-entry-2026-11-03.json.gz` retains source/configuration,
raw outputs, measurement receipts, collector/verifier and verified summary, with
SHA-256 hashes for every embedded file. Executables are excluded.
