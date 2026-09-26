# Runtime measurement contract

Status: additive source capability, not part of published v1.5.0 binaries.

```bash
kujo run --measurements measurement.json examples/hello.kujo
```

The existing profiler now has a production collector at
`src/benchmarks/runtime_measurements.rs`, exposed through `profiler::runtime`.
The legacy `profile` command remains an interpreter timer with manually populated
function/heap collectors; do not interpret its zero counters as measured absence.
The new surface executes the ordinary VM, including its existing capability,
scheduler, JIT opt-in, failure and detached-task semantics.

## Scope and meaning

One session per process. Fixed metric names and saturating unsigned 64-bit totals
bound memory and cardinality. No source text, paths, function names, values,
provider labels, exception text, or secret payloads enter this collector. There
is no event queue or network exporter. Optional clocks are monotonic wall clocks,
not CPU clocks. Process CPU and RSS reuse the existing `process_usage` host
mechanism at session boundaries only. Disabled collection reads the uninitialized collector flag and
does not read clocks or allocate per event.

| Fields | Exact interpretation |
| --- | --- |
| `wall_ns` | Wall time from collector admission after parsing/output creation through snapshot; includes compile, VM setup and teardown, excludes export |
| `cpu_seconds`, `process_peak_rss_bytes` | CPU delta for this process (all threads, excluding children) and process-lifetime peak RSS from the existing Unix getrusage mechanism; null when unsupported or unavailable; RSS is not a session delta |
| `vm_entries`, `vm_inclusive_wall_ns` | Entries/time in VM execute, including resumes and nested/callback execution; inclusive totals can exceed wall time |
| `vm_call_opcodes`, `vm_return_opcodes`, `vm_native_call_opcodes` | Dispatched opcodes, not unique functions, successful calls or calls executed wholly in generated machine code |
| `scheduler_rounds` | Cooperative scheduler rounds; not Tokio's internal scheduling decisions |
| `vm_closures_created`, `vm_capture_cells_created` | Created closure values and cells at MakeClosure; cumulative, not live/peak retention |
| `vm_capture_value_shallow_bytes` | Count times Rust Value size, excluding Arc/Mutex, map/string storage and referenced graph; not retained heap bytes |
| `vm_generator_states_created`, `vm_generator_state_drops`, `vm_generator_state_shallow_bytes` | Creation and drop events for VM GeneratorState; shallow size excludes continuation buffers/graphs. Rust clones are not separately counted as creation; do not infer a heap leak from subtraction |
| `vm_generator_resume_attempts` | Calls to generator_next, including exhaustion and rejected attempts |
| `tasks_admitted`, `tasks_admission_rejected` | Language-task admission, excluding callable/arity errors before admission |
| `tasks_started`, `task_queue_wall_ns` | Entry into blocking worker and sum of submission-to-entry time |
| `task_bodies_exited` | Admission release (including cancellation before worker entry); not successful completion |
| `task_completions_published`, `task_cancellations_published` | Winners taking the shared completion sender; publication attempts even if all receivers disappeared |
| `detached_tasks_observed` | Admitted tasks passed to detached observer; no implicit join |
| `promise_polls`, `promise_pending_polls`, `promise_ready_polls` | Shared receiver polls and outcomes, including cached outcomes and cooperative polling; not unique promises or wait duration |
| `jit_compile_entries`, `jit_compile_inclusive_wall_ns` | Compiler implementation entries/time, including rejection/failure and nested compilation; not successful compiled function counts |
| `jit_cache_lookup_hits`, `jit_cache_lookup_misses` | JitCompiler get_compiled/get_fn_info lookups; excludes VM inline-call cache |
| `jit_type_guard_passes`, `jit_type_guard_failures` | Executed integer/float type guard helper outcomes; zero does not mean specialization ran |

`unsupported` explicitly names total heap bytes, total Value allocation
and drop accounting, retained capture graph bytes, provider usage and provider cost.
Total allocator accounting needs a separate design: Values can be inline, cloned,
constructed directly and shared through Arc. Walking captures would add unbounded
work and could expose sensitive content. Sampling remains deferred. These limits
do not reopen completed closure/concurrency contracts.

Snapshot counters are independently read while workers may finish or continue.
No cross-counter equality is guaranteed during concurrent execution. Collection
does not wait for detached tasks; the report describes a prefix, not a complete
worker history. Separate processes have separate summaries. The Rust API rejects
a second start to prevent attribution of old detached work to a new session.

## Artifact handling and consumers

The CLI exclusively creates a new destination before compilation (0600 on Unix),
writes bounded JSON after VM teardown and fsyncs the file. It does not atomically
publish a checkpoint or guarantee a directory entry survives machine failure.
An interrupted writer can leave empty/truncated output. Consumers must bound reads
to 8 KiB, validate the schema, and hash accepted bytes. Version 1 preserves names,
units and meanings; additive fields are allowed within declared bounds.

This is a summary artifact, not another workflow event model. Reference it through
existing evidence refs/control-event refs. RunLedger's existing `note` and
`correlate --dispatch-run` commands preserve a path/digest and workflow association.
Never convert unsupported usage/cost into zero, sum inclusive wall time as CPU
duration, infer safe replay from measurements, or trust producer metadata as proof.
See [the ownership/handoff plan](NEXT_PHASE_ARCHITECTURE.md#measurement-ownership-and-consumption-handoff).

## Validation and benchmarking

`cargo test --test runtime_measurements` compares measured and ordinary program
stdout/stderr/exit status and validates real reports. Existing closure, generator,
concurrency and security gates protect behavior. Raw benchmark results and the
fresh review are recorded in `NEXT_PHASE_ARCHITECTURE.md`; no percentage overhead
guarantee applies across hosts, builds or workloads.
