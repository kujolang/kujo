# Function work and admission controls

All 120 diagnostic outcomes pass on corrected frozen runtime `5fe52fc5`, binary
`bf5bd666143cabb2339a72a5635702a6c735639e25d12663396aea7a87abaed4`.
Runtime source is unchanged. Thirty generated programs run in VM, interpreter,
JIT and function-JIT-disabled/loop-JIT-retained modes. Each has independent
straight-line call sites, avoiding admission or native dispatch from an outer
caller loop. Every total is checked against an independently calculable sum.

Three straight-line kernels contain 1, 16 or 128 integer increments. Three
loop controls execute 0, 8 or 2,000 iterations of an integer sum. Each kernel is
called 1, 99, 100, 101 or 1,000 times. Generated programs and exact invocation,
output and measurement coverage are independently verified. This does not test
all bytecode sizes, recursion, ownership or native-to-native admission paths.

## Observed admission

| Kernel | Calls | Compile entries | Native function entries |
| --- | ---: | ---: | ---: |
| Each straight-line body | 1 / 99 | 0 | 0 |
| Each straight-line body | 100 | 1 | 1 |
| Each straight-line body | 101 | 1 | 2 |
| Each straight-line body | 1,000 | 1 | 901 |
| Each loop, including zero trips | 1 / 99 / 100 / 101 / 1,000 | 1 | Same as call count |

Whole-function admission depends on 100 calls or the existence of a loop on the
first call, not the amount of executed loop work or measured compilation cost.
Even the zero-trip loop compiles on its first call. At exactly 100 calls the
straight-line controls pay compilation cost for only one native invocation.
The 1,000-call controls establish sustained native participation, not a speedup.

In these single observations, the straight-line 100-call cells spend approximately
3.37 / 19.06 / 271.92 ms inside the compile instrumentation for 1 / 16 / 128
increments. These loaded-host inclusive wall measurements are illustrative,
not stable compiler scaling ratios or self-CPU attribution.

Function-JIT-disabled controls separate the paths:

- Straight-line and zero-trip kernels never compile.
- Eight-trip loops compile during the thirteenth invocation, with 87 / 88 / 89 /
  988 native boundaries for 99 / 100 / 101 / 1,000 calls.
- The 2,000-trip loop compiles on its first invocation and retains reuse for
  subsequent calls, including all 1,000 invocations.

The verifier asserts these exact structural results. VM controls compile nothing;
all measured modes start zero tasks. No startup time is moved out of official
timing and no benchmark port is modified.

## Implications for the candidate

The evidence rules out a single threshold adjustment as a complete remedy:
raising the call threshold does not affect eager first-call loop compilation,
and suppressing function JIT leaves per-task loop recompilation in the async
controls. Conversely, blanket suppression loses native straight-line coverage.

The next implementation design should separate two mechanisms:

1. Demand-based admission for cold work, with long-running native participation
   retained. Compare against unchanged tree and other cold workloads, not only
   these constructed controls. Reject policies that merely relocate compilation
   or make long hot workloads fall back.
2. Owned reusable code across isolated executions for repeated task work. The
   async evidence places the repeated compilation in the task VM, so outer
   mapper reuse alone is insufficient. Establish a code-owner/cache boundary
   before attempting cross-task function-pointer sharing.

Cancellation policy is immutable per JitCompiler, but the token address is read
from VMContext at execution time (`BytecodeTranslator::translate_instruction`
and `jit_runtime_failed`). Code is not specialized to one embedded token address.
A cache must distinguish cancellation-check policy and retain code plus embedded
metadata (dictionary keys, struct shapes, names, paths and literal values).
CompiledFnInfo is only pointer metadata and does not supply that ownership.
This source observation permits investigation; it does not prove thread safety,
complete cache keys, safe eviction or compatibility with the separate lifetime
candidate. Do not add unsafe Send/Sync declarations to bypass those obligations.

Before a candidate is accepted, require repeated uninstrumented target gains,
VM/interpreter/JIT semantics, actual native participation, snapshots, dynamic
binding, cancellation/capability/error isolation, code lifetime, resource limits
and all 150 unchanged test-size checksums. Production and clean-host gates remain.

## Reproduction and evidence

```text
ruby benchmarks/langarena/probes/collect-jit-admission.rb FROZEN_BINARY NEW_OUTPUT
ruby benchmarks/langarena/probes/verify-jit-admission.rb NEW_OUTPUT
```

The compressed adjacent archive contains generated programs, raw measurements
and outputs, verified summary, collectors/verifier and source snapshots, with
per-file SHA-256 hashes. One instrumented observation per cell cannot establish
elapsed-time improvements. No runtime fixes or competitive gains are claimed.
