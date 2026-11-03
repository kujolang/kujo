# Async worker compilation economics

Forty diagnostic cells pass on frozen corrected runtime `5fe52fc5`, binary
SHA-256 `bf5bd666143cabb2339a72a5635702a6c735639e25d12663396aea7a87abaed4`.
No runtime implementation changed. These are instrumented structural controls,
not speed measurements or production benchmark passes.

The fixture wraps the existing 2,000-term sum in `async func worker(n) { return
kernel(n) }`, matching the async-wrapper/synchronous-helper structure of LangArena
Matmul. It executes 16 fixed jobs per repetition, one or four repetitions, either
awaiting each wrapper directly or using parallel_map at concurrency 1/4/8/16.
Every result, aggregate and input snapshot is checked. Each cell runs in VM,
interpreter, JIT and function-JIT-disabled/loop-JIT-retained modes.

| Execution | Jobs | JIT compile entries | JIT native function entries | Loop-only compile entries |
| --- | ---: | ---: | ---: | ---: |
| Direct async await | 16 | 16 | 16 | 16 |
| Parallel async, each concurrency | 16 | 16 | 16 | 16 |
| Direct async await | 64 | 64 | 64 | 64 |
| Parallel async, each concurrency | 64 | 64 | 64 | 64 |

Measured VM/JIT task starts equal job count. Loop-only controls have one cache
miss per job, zero hits and one native boundary per job. VM controls have no JIT
compilation. Interpreter checks exact outputs without unsupported measurements.
The verifier asserts these structural observations in addition to exact coverage,
invocations, input identities and raw measurement receipts.

## Dispatch distinction that changes the next action

An async source declaration does **not** imply the interpreter's
`Value::AsyncFunction` parallel mapper route under VM/JIT execution.
`src/compiler.rs` emits a BytecodeFunction with `chunk.is_async`. Consequently:

1. Parallel dispatch enters `try_parallel_map_with_bytecode` in
   `src/interpreter/native_functions/async_ops.rs`, creating a wrapper VM per job.
2. `VM::call_bytecode_function` sees the async flag and submits a language task.
3. `src/interpreter/tasks.rs` sends bytecode tasks to `VM::execute_language_task`.
4. `execute_language_task` constructs another isolated VM with the task's
   cancellation token and JIT policy, then enters the async body and helper.

Direct async await also submits a task and gets a fresh task VM, explaining why
the direct async control loses the synchronous direct control's compilation
reuse. Reusing only the outer parallel mapper VM therefore does not address
the task VM's repeated compilation. Delaying only function JIT also leaves loop
compilation. Any code-sharing design must respect task-specific cancellation
and code ownership; sharing live environments is not justified by these results.

Interpreter execution uses the distinct AsyncFunction route with isolated
interpreter environments. Its passing results are semantic controls, not evidence
of VM compilation reuse.

## Limits and next gate

The probe matches Matmul's wrapper structure, not its complete matrix kernel,
data transfer, captured state or production workload. It does not attribute the
full measured Matmul regression to compilation. Compile entries count instrumented
attempts/variants; inclusive compile wall spans can overlap and are not CPU time.
One observation per cell cannot establish a speed gain.

Next: admission call-count/body-size controls with long hot execution, followed
by a general compilation-policy/code-reuse hypothesis. Before acceptance, require
task isolation, cancellation, capability, error and code-lifetime checks, all-runtime
checksums and repeated unchanged-workload gains. The separate memory candidate
remains excluded.

Reproduce from repository root:

```text
ruby benchmarks/langarena/probes/collect-jit-async-economics.rb FROZEN_BINARY NEW_OUTPUT
ruby benchmarks/langarena/probes/verify-jit-async-economics.rb NEW_OUTPUT
```

The adjacent compressed evidence archive retains raw outputs, measurements,
report, verified summary, fixtures, tools and dispatch-source snapshots with
per-artifact SHA-256 hashes. No executables are embedded.
