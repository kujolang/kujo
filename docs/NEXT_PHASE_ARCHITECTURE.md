# Next-phase architecture audit and implementation plan

Audit: 2026-09-26, clean `main` at `cd6d2ea` (equal to fetched origin/main).
This record distinguishes source capability, released capability, and proposals.
Historical completion records are evidence, not instructions to reopen work.

## Stage 1: current truth and ownership

| Capability | Classification before this work | Source and verification anchors | Owner |
| --- | --- | --- | --- |
| Lexical snapshots, returned closures, indexed captures | Implemented on main | `src/compiler.rs`, `src/vm.rs`, `tests/closure_capture_contracts.rs`, `tests/closure_capture_audit.rs` | Runtime |
| Owned generator continuations | Implemented on main | `GeneratorState`, `generator_next` in `src/vm.rs`; `tests/generator_continuation_contracts.rs` | Runtime |
| Eager async, reusable promises, detached spawn | Implemented on main | `src/interpreter/{tasks,promise,async_runtime}.rs`; `tests/concurrency_lifecycle_contracts.rs`; 16 admission slots, cooperative cancellation, snapshot transfer | Runtime |
| Optional typing / compatibility | Partially implemented, continuing maintenance | `docs/RUNTIME_COMPATIBILITY_FOLLOWUP.md`, `src/type_checker.rs`, parity tests; 149-fixture completion is historical verified evidence, not this session's new test result | Runtime |
| Production measurement | Partially implemented | `src/benchmarks/profiler.rs` has explicit unwired collectors; `profile` in `src/main.rs` times interpreter execution, not default VM execution; hashmap diagnostics and JIT's own stats cover narrower paths | Runtime mechanism |
| Sampling, total heap allocation/drop attribution | Missing | Profiler inventory has placeholders; `Value` is an enum with direct construction, Arc aliases and interior containers, not a universal `Value::new` allocation API | Design required; do not label logical events as allocator bytes or leaks |
| Failure/evaluation control | Available elsewhere, implemented | Dispatch `61a367e`: `src/core/{control,runner,intervention}.kujo`, `tests/failure_gate_{execution,safety}_tests.kujo`; Eval `6a5ab09`: `src/control_contracts.kujo` | Dispatch policy; Eval producer |
| Bounded durable journal and safe replay | Available elsewhere, partially implemented | Dispatch `src/core/control_journal.kujo`: immutable record, directory sync, then JSONL index; 1 MiB/event, 8 MiB/journal. Admission reconciles hashes/cursor/orphans; ambiguity rejects execution | Dispatch |
| Preservation / materialization | Available elsewhere, partially implemented | Workcell `dc2afd1`: `src/evidence/{preservation,execution_result}.kujo`, preservation/coordinator tests; pre-evaluation preservation and provider capability results | Workcell, adapters |
| Intervention across process lifetime | Available elsewhere | Dispatch persisted boundary and v2 decisions; Leash `56c59c9`, `daemon/src/{chatops,store,policy}.rs` receipt/outbox transport | Dispatch authorization policy; Leash transport |
| Generic durable Run/Step/Checkpoint | Partially implemented; composition/design required | Dispatch state/revisions/attempts, execution/evaluation/evidence/preservation/reexecution schemas and existing failure golden path | Ecosystem, not a second VM serializer |
| Receipts, usage and costs | Available elsewhere | RunLedger `e0187ea`, `src/record.kujo`: nullable usage/cost, correlation IDs, commands/tests/notes; receipt status is not workflow state | RunLedger aggregation; providers supply costs |
| Failure bundles | Available elsewhere | CaseFile `f26df30`, `casefile.kujo`: redacted repository evidence and reproduction | CaseFile, not continuation authority |
| Agent/tool lifecycles | Available elsewhere, partial interoperability | Agents SDK `bb2202d`: `src/agents/ai/adapter.kujo` usage extraction and MCP lifecycle tests; MCP `0784589` local framework | SDK/MCP adapters |
| Structured context/provenance | Available elsewhere, partial | RAG `85743f8` local retrieval; Scent `969f0fd` context/manifest/redactions; SDK context ratchet | Ecosystem; semantic interop needs design |
| Universal graph execution | Proposed, design required | Dispatch already has DAG scheduling/admission and descendant control; extend compatible nodes/contracts | Ecosystem; unscheduled |
| Provider pricing/runtime agent framework/new syntax | Unnecessary for this slice | No runtime primitive requires it | Outside core |

Canonical boundaries remain `docs/{ARCHITECTURE,V1_SCOPE,FAILURE_GATE_IMPLEMENTATION,RUNTIME_CONCURRENCY_COMPLETION}.md`.
Generated `docs/generated/V1_CODE_TODO_TRIAGE.md` lists 29 markers at baseline;
13 concern the legacy profiler. These are not 13 missing language features.
`benches/v1_perf_benchmarks.rs`, `src/benchmarks/{runner,timer,stats}.rs` and
`scripts/closure_snapshot_bench.rs` are existing measurement infrastructure.

### Release reconciliation

`v1.5.0` is annotated tag object `83fbdd4`; its peeled source commit is
`cc2d7dbb59a8dc05f00d629e100932f56f4062f6`.
GitHub published its five native archives on September 22. The directory-sync
primitive was added later in `f765152`; the tag's filesystem implementation
does not contain it. Dispatch's `release/dispatch-v1.3.0.refs` pins Kujo
`87fae36dd331b185d29256f74d2a92f1aefc5ea5`, not tag v1.5.0. Main includes
additional concurrency/compatibility work through PR #12 (`9d3c6ed`). All can
report crate version 1.5.0: version text alone does not identify source behavior.

The npm registry confirms resolver 1.5.0 and five exact native optional
dependencies at 1.5.0 with provenance metadata. Publication of packages does
not imply rebuilding them from current main. `publish-npm-runtime.yml` publishes
tarballs from an explicitly selected release workflow run. Binary comparison
evidence is recorded in the validation section when collected. Source-runtime
Workcell validation with `WORKCELL_TEST_KUJO_VERSION=1.5.0` does not certify its
older release pin. The roadmap's two entries were compatible facts with an
insufficiently explicit distribution/source boundary, not proof of a new release.

## Stages 2–3: Wave A implementation plan (before coding)

Extend the existing profiler module with an opt-in, fixed-cardinality runtime
measurement collector. Do not introduce another tracing/event bus or change
`kujo.control-event/v1`. Add a versioned **summary artifact**, referenced as
evidence by existing control events and RunLedger notes/correlations.

1. `src/benchmarks/profiler.rs` and its runtime submodule: bounded numeric
   counters, monotonic elapsed durations, fixed names, no values/names/paths or
   secret-bearing labels. Process-scoped single session includes language workers;
   a snapshot does not wait for detached work or claim global atomicity.
2. `src/main.rs`: additive `run --measurements PATH`, exclusive output creation,
   ordinary VM/scheduler/capability path, separate artifact preserving stdout and
   runtime failure exit codes. Parsing failure creates no measurement. Interpreter
   remains an explicit unsupported combination for this first VM surface.
3. `src/vm.rs`: dispatcher call/return, scheduler rounds, closure capture count,
   generator state creation/drop and shallow size; no recursive heap traversal.
   `src/interpreter/{tasks,promise}.rs`: admitted/rejected/started/completed tasks,
   queue duration and poll outcomes; detached observation count. A promise poll
   count is not a model request or unique promise completion count.
4. `src/jit.rs`: compile entries/time and cache lookups, executed type guards.
   Preserve JIT opt-in and specialization policy. Distinguish attempted compilation
   from success; do not infer internal generated calls from dispatcher counts.
5. Contract documentation/schema and `tests/runtime_measurements.rs`: actual CLI
   workload, failure, exclusive output, redaction, detached lifetime, task/promise,
   generator/capture and JIT/no-JIT coverage. Unknown heap/CPU/provider metrics
   stay unsupported rather than fabricated zero measurements.
6. Benchmark with the existing Rust runtime/compiler, repeated paired enabled/
   disabled runs, fixed inputs and raw timings; compare to clean pre-change
   executable where practical. Record host/build details and dispersion, with
   no universal speed/overhead promise. Run format/check, targeted regressions,
   full tests and canonical release gate; review before Wave B.

Performance risk: atomic increments and clock reads at selected boundaries,
especially high-frequency calls/polls. No opcode-wide timing, per-function map,
dynamic labels, export network calls, or per-event file writes. Compatibility
risk: CLI parsing and diagnostic paths; assert identical output/exit behavior.
Persistence risk: measurement artifact is diagnostic evidence, not a checkpoint;
termination can leave it incomplete. Consumers must validate before accepting it.

### Measurement ownership and consumption handoff

Runtime: elapsed time, VM events, logical lifecycle counts and shallow storage.
Dispatch: run/step/attempt, retries, blocked/review time, checkpoints, interventions,
effects and replay decisions. Workcell: lifetime, preservation availability/size,
environment and materialization. Eval: evaluation count/outcome/evidence. SDK and
provider adapters: requests, observed provider/model, estimated versus reported
tokens and external pricing/currency. RunLedger aggregates nullable measurements
and references; unknown does not become zero. Context tools own input/artifact
size/provenance. Do not copy payloads or private model reasoning into telemetry.

RunLedger ingestion handoff: retain the summary as an evidence artifact, hash it,
link it from a receipt note/command and existing Dispatch correlation. Validate
schema/version and bounded input; aggregate only matching scopes/units, never sum
nested wall time as CPU time. A richer importer remains RunLedger-owned; no
receipt status or cost schema changes are needed to preserve this evidence.

## Wave B entry criterion and Wave C boundary

Wave B starts only after Wave A verification and overhead review pass. Reuse
Dispatch's persisted run/steps, control attempts, locked v2 decision admission,
control journal and Workcell preservation refs. A checkpoint is a verified safe
boundary manifest, not a serialized live VM/promise or a new lifecycle enum.
Prove separate-controller stop/load/decision/resume against the actual offline
Workcell/Eval path before claiming completion.

Crash rules to preserve: before action admission, only verified pending work can
run; during execution or after a local/external mutation, classify uncertainty
and block replay; after result creation but before indexing, retain orphan evidence
and require reconciliation; before checkpoint publication, no resumable checkpoint;
after publication, verify journal/state/input/policy and preservation before use.
Evaluator crash is indeterminate and evaluator-only retry must not repeat the
action. Transport failure leaves the review boundary pending; repeated identical
decisions are idempotent and conflicting/stale decisions fail. Expired preservation
or missing adapter blocks replay, even with a well-formed checkpoint.

Wave C recommendation: extend existing execution-result effect entries, separating
operation class from replay properties and assurance. Keep existing conservative
`none/local_reversible/external_idempotent/unknown` admission meaning. Candidate
assurance states are claimed, observed, adapter-attested and independently verified;
strings alone are not trusted proof. Bind target/scope, attempt, completion,
transaction/idempotency enforcement evidence, compensation availability and
uncertainty to bounded evidence refs. Trust resolution belongs to authorized
adapters/Dispatch; unknown and started effects remain unsafe. Taxonomy and migration
need cross-adapter fixtures before any broad v2 effect contract. No exactly-once,
universal rollback, pricing tables, or provider orchestration enters Kujo core.

### Artifact comparison evidence

The downloaded macOS x64 archive matches its published SHA-256
`1aebcd482125031104b2df79abae6db57973f1874ceb196f95989b14e287d820`.
Its executable and the npm `@kujolang/kujo-darwin-x64@1.5.0` executable both hash
`3e1e475ea165c8b4a714495596fe8661ad970b27119ad44f1db4d9779f7f05d4`.
The npm package's `metadata.json` identifies source `cc2d7dbb59a8dc05f00d629e100932f56f4062f6`,
matching the peeled tag and signed release-build record (Actions run 35787043614).
This session compared macOS x64 bytes; the other four platform clean-install
claims remain the existing release CI evidence, not fresh local execution.

A direct CLI probe also verified the distinction: npm's executable exits 4 with
`Undefined variable: sync_directory_beneath`; the immutable main baseline built
from `cd6d2ea` executes the same confined-directory sync and prints `ok`.

Implementation audit refinement: `process_usage` already supplies Unix process CPU
and peak RSS through `web_data.rs`. Its host read is shared with the profiler at
session boundaries; no second resource-usage implementation or per-event syscall
is needed. Non-Unix/unavailable readings remain null. The original plan's CPU
unsupported assumption is superseded by this source finding.
