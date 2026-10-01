# Next-phase architecture audit and implementation plan

Release update (2026-09-28): Kujo 1.6.0 now publishes the reviewed source
`44af277848173664f72ca85f2a1b3b98d634ecdd` on all five native/npm targets.
The 1.5/source comparisons below are historical audit evidence, not current
installation instructions. See [release receipt](KUJO_1_6_RELEASE.md).

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
| Lifecycle telemetry | Available elsewhere, implemented | Watchdog `4e07223`: `schemas/watchdog-native-event-v1.schema.json`, `telemetry_native_adapter.kujo`; MCP `src/telemetry/watchdog.kujo` metadata-only producer | Watchdog/producer adapters |
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
Recent ancestry also includes merged roadmap PR #13 (`4c1d7c3`), npm publication
record `ea7ca60`, and npm README/version guard `cd6d2ea`. Those documentation/CI
commits do not retag or rebuild the older native release.

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
class enum (`none`, `local_reversible`, `external_idempotent`,
`external_non_idempotent`, `destructive`) and effect states. `unknown` is an
effect state, not a replay-safe class; missing/invalid classes remain unsafe. Candidate
assurance states are claimed, observed, adapter-attested and independently verified;
strings alone are not trusted proof. Bind target/scope, attempt, completion,
transaction/idempotency enforcement evidence, compensation availability and
uncertainty to bounded evidence refs. Trust resolution belongs to authorized
adapters/Dispatch; unknown effects without validated replay safeguards remain unsafe. Taxonomy and migration
need cross-adapter fixtures before any broad v2 effect contract. No exactly-once,
universal rollback, pricing tables, or provider orchestration enters Kujo core.

The later source review refined this statement: existing Dispatch deliberately
permits an unknown external-idempotent completion when enforcement references are
present, backed by an actual local deduplicating sink fixture. Reference presence
is not independent trust verification. See the [concrete Wave C recommendation](EFFECT_CONTRACT_DIRECTION.md)
for the exact current boundary, proposed assurance fields and migration tests.

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

## Durable-state crosswalk (audit, not a new lifecycle)

| Requirement | Existing representation | Remaining design boundary |
| --- | --- | --- |
| Run / step / attempt identity | Dispatch `run_id`, step `id`, action `attempts` and `control_attempt`; portable result subject has run/step/attempt IDs | Do not conflate transport decision ID, evaluator attempt and action retry |
| Parent / child | Dispatch `depends_on` DAG edges and scoped descendant barrier | Cross-run ancestry is not a universal contract yet |
| Inputs / environment | Persisted input and workflow definition; reexecution descriptor source commit, workflow SHA, tool version/image digest, input evidence refs and secret refs | A portable safe-boundary manifest must bind these to verified bytes and adapter identity |
| Capabilities | Runtime capability gates; workflow/tool admission and preservation capability outcomes | Persist the relevant policy identity/digest; a producer's claimed policy is not runtime enforcement evidence |
| Results / effects / evidence | Existing execution/evaluation-result and evidence-ref contracts plus control-event refs | Preserve effect uncertainty separately from quality verdict |
| Checkpoint / preservation | Persisted state revision and journal cursor; Workcell preservation outcome/deadline and reexecution descriptor | Generic checkpoint binding/publication needs composition, not serialization of VM stacks |
| Evaluation / intervention | Existing boundary, policy result, v2 request/decision, evaluator-only retry | Long pauses need durable evidence and revalidation, not a living controller |
| Eligibility / lifecycle | `status_is_terminal`: completed/failed/rejected/cancelled; `can_resume_status`: running/paused/interrupted/needs_changes; control barrier independently gates descendants | `indeterminate` belongs to a result; do not silently introduce a conflicting run enum |
| Failure / uncertainty | Classified execution result; unknown/started effects, corrupt journal and stale decision errors | No automatic recovery converts a crash into a known-safe action |

Source anchors: Dispatch `src/core/state.kujo::{create_run_state,persist_run_state,
status_is_terminal,can_resume_status}`, `src/core/runner.kujo` locked admission;
Kujo `schemas/workflow-control/{execution-result,reexecution-descriptor,preservation-outcome}-v1.schema.json`.
The existing failure-gate example rebuilds a registry and retries evaluation within
one controller. Its success alone does not prove the requested separate-process
checkpoint/load/continue golden path.

## Wave A measured evidence (initial dev-profile campaign)

Raw observations and host/build provenance are in
`benchmarks/results/next-phase-2026-09-26/`. Baseline is immutable `cd6d2ea`;
candidate runtime is `bdf634f`. Eleven samples/mode with a discarded warmup,
rotating order, and exact stdout/stderr/success checks:

| Workload | Baseline median ms | Disabled median ms | Enabled median ms | Median paired disabled/baseline delta | Median paired enabled/disabled delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| 10,000 calls | 372.648 | 377.330 | 435.945 | +1.26% | +6.31% |
| 1,000 captures/generators | 1536.788 | 1567.494 | 1571.184 | -0.11% | +1.82% |
| 100 async calls | 450.873 | 446.208 | 478.894 | -0.16% | +6.18% |

These are unoptimized dev executables, process-inclusive measurements with report
file sync in enabled samples, and substantial host timing dispersion (the JSON
contains min/p90/max and every sample). Negative deltas are noise, not speedups.
They establish a nonzero opt-in cost and no observed large disabled regression
on these workloads; they do not bound production/release overhead or JIT costs.

Focused CLI/schema/behavior tests: 8 passed. The initial test draft used a
nonexistent language-level generator_next function and an unsupported JIT closure
fixture; both were corrected to actual language iteration and a supported JIT
surface. These were harness failures, not suppressed runtime checks. A review
caught measurement options leaking through legacy args() fallback and added an
argument-parity regression.

Final Wave A validation: `cargo check`, eight focused measurement tests, and
`CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 bash scripts/release_gate.sh --full` passed.
The full gate includes formatting, strict Clippy across all targets/features,
all Rust suites, 90 native security tests, nine package/module tests, 117 parity
tests, and dual/interpreter sweeps (149/149 each, eleven explicit fixture skips).
An additional explicit VM sweep passed 149/149 with the same skips. Cargo audit
passed against 1,271 advisories and 652 dependencies; its existing documented
build-only unmaintained-dependency exclusion was unchanged. Optional cargo-deny
was unavailable, socket serving opt-in and the gate's benchmark smoke remained
at their defaults; the separate measured campaign above ran in full.

Fresh review checked fixed cardinality, no telemetry content, off-path clocks,
argument forwarding, detached-work prefix semantics, inclusive timing labels,
exclusive private report creation, outcome handling and schema bounds. No
outstanding implementation defect was found. Production optimized overhead is
not inferred from dev measurements; adoption can measure its own release build.

### Existing observability contract decision

Watchdog already owns `watchdog.native-event.v1`, normalized to
`watchdog.telemetry.v2`: session/turn/model/agent/tool/handoff/retrieval/workflow/
approval/execution/error/artifact/evaluation/internal observations, source-owned
references, usage, costs and bounded scalar attributes. Its adapter explicitly
excludes prompts, tool inputs/results and document content. MCP already produces
this contract. Therefore **no new neutral agent lifecycle event contract is
needed** for Wave A. Keep that event model and Kujo's control-event model distinct:
observations are not authoritative control transitions.

The runtime-measurements contract is a summary artifact because standalone VM
execution has no caller-owned trace/event/step identity. It is not a competing
trace envelope. A Watchdog adapter can attach the report digest as an artifact
reference to an existing execution observation and project numeric counters into
bounded scalar attributes, supplying actual caller-observed timestamps and trace
identity. Adapter conformance and delivery belong to Watchdog/SDK repositories.
RunLedger keeps receipt/correlation references rather than becoming another event
store. This handoff needs no provider-specific core fields.

An ingestion caveat from source: Agents SDK `extract_usage_metrics_impl` defaults
missing usage/cost values to zero for its budget interface; RunLedger starts those
fields as null. A future aggregator must consult original usage availability and
estimate provenance rather than interpret every SDK zero as observed zero spend.
No pricing or billing assumption is imported into the runtime. Existing untracked
Agents SDK maintenance-agent work was observed and left untouched.

## Wave B implemented slice and evidence

After the full Wave A gate passed, Dispatch implemented a composed **review
checkpoint**, not a general serialized execution engine. Its pre-coding plan is
`dispatch/docs/audits/durable-review-checkpoint-plan.md`; contract/crash semantics
are in `dispatch/docs/review-checkpoints.md`; detailed fixture evidence is in
`dispatch/docs/audits/durable-review-checkpoint-evidence.md`.

Dispatch `86a5f2c` closes a reproduced direct-runner bug: rejected terminal runs
could execute pending descendants. `682f0e4` adds a content-addressed immutable
state snapshot and `dispatch.review-checkpoint/v1` manifest, published last after
directory synchronization. A new `checkpoint` command loads authoritative state
under the run lock, requires a quiescent paused boundary and reconciled journal.
Optional `resume-decision --checkpoint` validates live authority and retained
bytes before the existing v2 claim/decision/continuation path. It does not replace
policy, introduce a new lifecycle or serialize live closures/tasks.

Real offline proof: `run-1790470206138-3990`, recorded under Dispatch
`tests/tmp/durable-review-63009`. The first process ran Workcell fixture
preservation and real Eval, retained the failing judgment, blocked its descendant,
published a 932-byte manifest plus 51,892-byte snapshot, and exited. Separate
processes rejected missing checkpoint and stale decision without changing state.
Another CLI controller accepted an explicit review override and continued.
Action/evaluator/descendant attempts were [1,1,1]; duplicate delivery caused no
additional decision or execution. A final process reconciled the journal, verified
unchanged Workcell/Eval hashes and wrote/read an actual RunLedger receipt with the
Dispatch correlation and proof digest. Unknown cost stayed null. Existing
single-controller failure/evaluator-only retry example also still passes.

Four focused checkpoint tests pass under filesystem and SQLite authority, including
a stale SQLite secondary state export. Schema/tampering/stale-input/in-flight-work/
orphan/torn-journal cases reject. Existing terminal/execution tests pass (six tests,
including all five terminal spellings). The fixture had initial harness failures
for overwrite flags and output-root policy; final runs use the supported interfaces
without weakening gates. A fresh review corrected character versus UTF-8 byte
limits and rejected malformed boundary metadata before manifest publication.
The first full Dispatch gate passed focused suites and 23 core shards, then
caught the missed help snapshot update (18 versus 19 commands). After inspecting
actual output, only the intended checkpoint command/flag expectations changed;
the affected four-test shard passed. The final full gate rerun passed with exit 0:
all smoke/concurrency/bridge checks, focused suites, all 101 core tests across 24
shards, quiet VM/interpreter command surfaces, version consistency, and 3/3 bounded
release workloads in 24s. The successful log is
`/tmp/dispatch-next-release-gate-final.log`; individual logs and workload JSON
remain under Dispatch's ignored `tests/tmp`. The final-head restart proof and
SQLite checkpoint suite also passed. No gate was suppressed. Final Kujo
formatting and eight architecture/README/Markdown/hygiene contract tests passed.

### Deferred scope and practical risks

| Priority / owner | Exact boundary |
| --- | --- |
| Completed, Watchdog/RunLedger | Bounded native-observation adapter, verified receipt notes and real HTTP/restart proof; see [Wave A completion](WAVE_A_ECOSYSTEM_COMPLETION.md) |
| Completed campaign; maintenance remains | Identical optimized baseline/disabled/enabled samples and export-cost probes are recorded. Platform/JIT-specific and long-run production tuning remain open; no universal budget |
| Later, Dispatch/Workcell | Recover lost authoritative state from verified backups, migrate machines/environments, archive journals, and support additional safe stop boundaries |
| Later, ecosystem trust adapters | Authenticate remote intervention and independently verify effect attestations; current producer references alone are not trust proof |
| Provider-dependent | Certify actual preservation/materialization/transaction semantics against authorized live systems; offline fixtures do not certify them |
| Speculative, unscheduled | Universal typed graph nodes, semantic context interoperability, provider SDK adapters, async generators and other new language syntax |

Security: metrics contain no payloads, but checkpoints contain persisted application
data and require private output storage/umask. Hashes do not authenticate a hostile
writer. Performance: counters add measured opt-in overhead; CPU is process-wide,
wall timings can overlap, and detached work gives a prefix observation. Persistence:
review checkpoints require the surviving run store, evidence and journal; partial
publication conveys no permission. Compatibility: both additions are opt-in and
unreleased; npm/native 1.5.0 does not acquire new behavior from source commits.
Replay: a checkpoint does not prove effect safety, and an override accepts an
existing outcome without replay. Exactly-once/rollback guarantees remain absent.

### Recommended next agent task

The original Watchdog/RunLedger and optimized-build handoff is now completed;
see [Wave A end-to-end evidence](WAVE_A_ECOSYSTEM_COMPLETION.md). Wave A is complete
as an unreleased foundation. Wave B remains the bounded review-checkpoint slice.
No Wave C assurance implementation was introduced by the integration task.

A next agent can begin a **scoped Wave C assurance slice** from
[EFFECT_CONTRACT_DIRECTION.md](EFFECT_CONTRACT_DIRECTION.md), starting with
Dispatch `src/core/intervention.kujo::retry_is_effect_safe` and the existing v1
execution-result/evidence contracts. Specify how two real offline sink adapters
bind run/step/attempt/effect/result digest to sink-enforced idempotency scope and
expiry, and how Dispatch resolves trusted evidence independently of producer
claims. Demonstrate crash-after-effect and expired/mismatched enforcement
failures before proposing admission changes. Preserve completion-state versus
replay-class distinctions and the current documented v1 exception; negotiate any
additive assurance contract before broad implementation. No general machine-loss
recovery, exactly-once effects, or universal rollback follows from Wave A.

Separately, measurement maintenance should evaluate bounded buffered export while
retaining exclusive creation, permissions and file sync. The optimized campaign
identifies a fixed export cost; do not silently weaken durability for latency.

### Commit map

| Repository | Commit | Purpose |
| --- | --- | --- |
| Kujo | `d9a8202` | Source/ecosystem audit, release-artifact distinction, architectural roadmap and Wave A plan |
| Kujo | `bdf634f` | Bounded production VM/JIT/task/generator measurements, CLI/schema/tests |
| Kujo | `a81da42` | Raw overhead evidence, full Wave A validation, consumption handoff |
| Kujo | `6b84f61` | Concrete future effect-assurance design and existing trust exception |
| Dispatch | `86a5f2c` | Terminal admission fix and pre-coding durable review plan |
| Dispatch | `682f0e4` | Immutable review checkpoints, fresh-controller resume and failure fixtures |
| Dispatch | `562f56d` | Reject malformed boundary metadata, align help snapshot and verify CLI publication |
| Dispatch | `73e7a87` | Final-head restart evidence, full release-gate pass and persistence/privacy boundaries |

Closing documentation commits retain the final gate evidence, Wave C design and
this actionable handoff. No Workcell/Eval/RunLedger/SDK implementation was changed.

### Wave C follow-up

The scoped next task above is now implemented as an opt-in Dispatch assurance
prototype validated against SQLite transactions and Workcell Git ref transactions.
See [the updated effect direction](EFFECT_CONTRACT_DIRECTION.md).
The default v1 replay policy and runtime are unchanged; broad migration remains
unscheduled. The next slice is the application-owned Ability gateway with
authenticated tenant/principal scope and replay-time enforcement, not result/v2.

### Ability application assurance follow-up

The next application-owned slice is validated with a real SQLite publication
gateway, separately durable business/receipt commits, external local session
authentication, owner fencing, revocation, expiry and concurrent replay tests.
Ability documents the source audit and evidence in
`docs/audits/application-assurance.md`; Dispatch provides an opt-in profile mapping
and a host admission callback that keeps the authoritative run lock through
execution. This supersedes the previous next-task pointer. The next task is the
Wave C compatibility/migration specification, not more adapters or result/v2.
Wave C remains experimental and unreleased; no general recovery or exactly-once
claim follows. Kujo runtime, Workcell and Wave A contracts are unchanged.

### Compatibility specification follow-up (2026-09-27)

The Wave C compatibility/migration specification is complete in Dispatch
`docs/effect-assurance-compatibility.md`, with a bounded advisory capability
catalog and opt-in reference evaluator tested through SQLite, Git and Ability
verifiers. This supersedes the prior compatibility-document next task. Next is
persisted admission negotiation and mixed-version restart/rollback tests. Alpha
remains unreleased; general ecosystem migration, remote trust and multi-effect
admission remain incomplete. Runtime and v1 result contracts are unchanged.

## Wave D non-Ability validation — 2026-09-27

The controlled process → Workcell Git CAS → Dispatch slice removes Ability from
the execution path. Existing Git marker/ref verification resolves real pre-commit
absence and post-commit completion loss before checkpoint-bound replay. A private
one-use ticket prevents duplicate process admission; the participant transports
references and never owns replay policy. Kujo runtime and execution-result/v1 are
unchanged.

The four-way comparison in Dispatch `docs/audits/wave-d-git.md` supports extracting
a small controller-subject/evidence-reference core next, while keeping participant
identity details and effect-family evidence in separate versioned contracts.
Ability receipts are not universal; a Git intent commitment is not an application
transaction. No generic handoff schema, remote trust or multi-effect behavior was
introduced. Wave D remains experimental and unreleased.

## Wave D external TypeScript adoption — 2026-09-27

Dispatch `interop/typescript-participant` independently implements the published
alpha correlation core using TypeScript and pinned generic Node dependencies. It
produces native generic handoffs rather than adapting a historical participant.
An operator-installed closed extension validator plugs into the existing generic
reader; no new family-specific core reader or replay policy is introduced.

Real pre/post-CAS SIGKILL paths retain unknown participant knowledge, use Workcell
live beta verification and resume through persisted Dispatch required/deny review.
Four concurrent contenders admit one execution, retaining one logical Git effect.
The four existing participants retain historical bytes and their regression paths.
This is local trusted-host interoperability, not remote authentication, machine-loss
recovery, arbitrary effect support or stable packaging. See Dispatch
`docs/audits/wave-d-typescript.md` for exact validation and remaining boundaries.
The next bounded test is an independently implemented external Python participant;
Kujo runtime and execution-result/v1 remain unchanged.

## Wave D external Python adoption — 2026-09-27/28

Dispatch `interop/python-participant` independently implements the published alpha
core with Python stdlib and hash-pinned jsonschema dependencies. It emits native
generic handoffs, owns a closed two-field participant extension and reuses the
existing Workcell Git correlation extension. No prior participant codec is imported
or translated, and no new production Dispatch correlation reader is needed.

Twenty published commitment vectors and28 runtime-neutral TypeScript/Python parity
cases agree. Canonical UTF-8, duplicate-key rejection, integer/Unicode limits and
exact content references do not depend on Node behavior. A copied standalone package
runs offline outside ecosystem checkouts after wheel installation.

Real Python SIGKILL before/after CAS preserves unknown participant knowledge while
Workcell separately observes not_started/committed. Fresh controllers preserve
required/deny beta policy through review and complete a descendant; four contenders
admit one and deny three, with one logical Git effect. The host remains trusted local
infrastructure. No remote authentication or machine-loss recovery is claimed.

Canonical detailed evidence: Dispatch `docs/audits/wave-d-python.md`. The next task
is generic participant SDK API design, comparing both independent external surfaces
without publishing, freezing the API or moving replay authority out of Dispatch.

## Wave D participant SDK API prototype — 2026-09-28

Dispatch `docs/contracts/participant-sdk/design.md` defines an experimental,
unreleased API/conformance version separate from `kujo.interop-handoff/v1alpha1`.
Independent TypeScript/Python codecs remain behind idiomatic exact-byte APIs,
closed operator-installed extension registration and host-snapshot correlation.
Pure recording helpers support provisional unknown, usable terminal report and
fresh recording-only finalization after host readback. They cannot execute effects,
consume admission, fetch evidence, authenticate a principal or authorize replay.

The shared 45-case SDK corpus, existing portable/parser corpus and actual Git
pre/post-commit loss/restart/contention fixtures validate the two prototypes.
Legacy participants and Dispatch production replay code remain unchanged.
Packaging is the next bounded task if full gates remain green; no publication,
API stabilization, remote trust or universal execution lifecycle is implied.

## Wave D experimental participant SDK packaging — 2026-09-28

Dispatch `packages/participant-sdk-ts` and `packages/participant-sdk-python`
prepare `@kujolang/participant-sdk@0.1.0-alpha.1` and
`kujo-participant-sdk==0.1.0a1` as local, unpublished artifacts. Only pure
codec/correlation and recording helpers ship, alongside pinned local assets
and conformance metadata. Admission, effects, trust configuration, reference
lookup, assurance and replay remain host/Dispatch responsibilities.

Clean external installs exercise the same 45-case corpus and real Workcell Git
pre/post-commit process loss, four-process contention and fresh-controller
review/replay. The fixture pins installed package inventories using the existing
configuration commitment; production algorithms and runtime are unchanged.
Npm and wheel builds are reproducibility-checked; sdist timestamps are documented.
See Dispatch `docs/audits/participant-sdk-packaging.md` for final gate results and
artifact provenance. Next: private distribution rehearsal, not automatic publication,
API stabilization, remote trust or an effect/Dispatch client SDK.

## Coordinated Wave C/D follow-up (2026-09-29)

See [the bounded effect-set proof](WAVE_CD_EFFECT_SET_PROOF.md) for the current
source audit, additive ownership decision, minimum participant protocol, Go
CLI/MCP proof, freshness/crash matrix and remote threat model. Historical wave
completion records above retain their original scope.

A subsequent [one-effect admission proof](WAVE_CD_ONE_EFFECT_ADMISSION.md) connects
one explicit candidate to Dispatch authority and final SQLite mutation checks.
The assessor remains informational, and no runtime policy moved into Kujo core.

The completed tranche is [bounded sequential continuation](WAVE_CD_SEQUENTIAL_CONTINUATION.md):
append-only unconsumed evidence rebinding and abandonment, independent next-effect
selection, and the same Dispatch authority over SQLite and Workcell Git. It retains
one negotiated family per run and does not add a scheduler or participant authority.
The linked receipt records the complete verified gate. Retained-host durable
recovery and operator reconciliation subsequently completed at their bounded local
scope; neither tranche implies remote trust.

## Retained-host reconciliation follow-up

[Retained-host recovery](RETAINED_HOST_RECOVERY.md) adds a bounded Dispatch
operator inventory/plan/locked-apply path over surviving trusted local authority.
It reconstructs supported lifecycle publication gaps while retaining consumed
claims and unresolved uncertainty. It does not restore arbitrary missing state or
resume work. The subsequent [parent finalization slice](PARENT_FINALIZATION.md)
binds completed effects, exact outputs, evaluation, preservation and explicit review
to a separate locked terminal decision. Its narrow scope is the last unresolved
protected parent, not general graph scheduling. The subsequent bounded Wave F
composition below preserves distinct scheduling dependencies and parent completion
obligations while adding explicit artifact provenance.

## Wave F bounded static composition

[Producer/consumer composition](WAVE_F_NODE_COMPOSITION.md) extends Dispatch's
existing DAG runner with exact run-backed nodes. Scheduling dependencies and typed
data inputs are distinct. Nodes retain independent effects, evaluation and terminal
authority; graph progression records successful node decisions without invoking a
consumer from producer finalization. The original parent-finalization contract and
all historical participant/result bytes remain unchanged.

The subsequent [heterogeneous terminal slice](WAVE_F_HETEROGENEOUS_TERMINALS.md)
adds graph-local Eval and authorized human review terminals alongside existing
program parent receipts. Kind remains descriptive; closed configured contracts own
completion requirements. Required/optional membership remains anchored independently
of scheduling/data edges. A separate locked graph decision binds exact node outcomes;
all facts without that event stay nonterminal, including after retained recovery.

The subsequent [static graph policy slice](WAVE_F_STATIC_GRAPH_POLICY.md) adds
closed failure strategies, predeclared branches, conditional joins, one-level subgraph
terminal decisions and a dispatch-reservation ceiling. The subsequent
[attempt-accounting slice](WAVE_F_GRAPH_ATTEMPT_ACCOUNTING.md) derives held, consumed
and released units from durable history. A sealed refusal before admission can support
an explicitly authorized second child run for the same node with exact original inputs.
Consumed or uncertain work cannot use that path. Nested/conditional groups and general
retries remain deferred. The subsequent [resource-accounting slice](WAVE_F_RESOURCE_ACCOUNTING.md) adds bounded Eval dispatch budgets, active-path reservation plans and exact runtime/SDK measurement references. Observed, provider-reported, estimated and unknown values remain distinct. Hard token/time/currency ceilings remain deferred; dynamic topology is not implied.
Remote trust and machine migration remain separate boundaries.
