# Kujo failure-gate, evidence preservation, and review architecture

Review date: 2026-09-24

This report is an implementation plan, not an implementation. It is based on
source, tests, and documentation in the Kujo ecosystem repositories listed in
the verification record. Repository behavior takes precedence over prose.

## 1. Executive summary

Kujo already has most of the mechanisms needed for reviewable failure
boundaries, but they do not share a control contract. Dispatch owns durable
workflow state, dependency scheduling, approvals, and resume. Eval produces
deterministic gate artifacts and a meaningful exit code. Workcell produces
receipts, logs, changes, artifacts, and integrity manifests. Leash provides a
durable, single-use human-decision queue and resume outbox. RunLedger,
Watchdog, CaseFile, ChangeBucket, ShipCheck, Fence, Muzzle, the Agents SDK,
and the AI SDK each contribute useful evidence or result shapes.

The original concern is valid. Today an Eval failure executed as an ordinary
required Dispatch step becomes a terminal Dispatch failure. It does not enter
the existing resumable approval path. Dispatch deliberately distinguishes
`failed` from `paused`, and that distinction should remain: failure is a
terminal outcome; pause is a nonterminal control decision. A failed
evaluation should become a pause only when an explicit policy maps that
evaluation to intervention.

The smallest sound change is additive:

1. Publish small, versioned, producer-neutral contracts for evidence
   references, execution results, evaluation results, policy decisions, and
   intervention decisions.
2. Teach Dispatch to normalize those results, apply declarative or external
   policy, checkpoint a review boundary, quiesce a declared DAG scope, and
   enter its existing paused lifecycle.
3. Extend Leash's existing intervention transport and Dispatch resume bridge
   with explicit continuation actions rather than inventing a second review
   system.
4. Give Workcell a provider-neutral preservation outcome and bounded handoff
   bundle. A late evaluation cannot preserve a remote workspace that was
   already destroyed, so preservation intent must be declared before cleanup.
5. Make Eval emit a formal evaluation result while preserving its current CLI,
   reports, manifests, and exit codes.

Kujo should not merge these products, make every failure resumable, promise
rollback of external effects, retain failed environments indefinitely, or
call stored logs deterministic replay. State snapshots should remain the
operational source of truth. A small append-only control journal is justified
for review-boundary transitions because Dispatch traces are bounded and its
existing mutation log is not a complete lifecycle history; this is an audit
journal, not an event-sourced rewrite.

## 2. Current architecture

### Ownership map

| Concern | Current owner | Actual boundary |
|---|---|---|
| Language execution and capabilities | Kujo runtime | Runs Kujo code and gates filesystem, process, network, AI, database, clock, and random effects. It does not own workflow policy. |
| Workflow execution and DAG scheduling | Dispatch | Persists run/step state, resolves dependencies, schedules bounded parallel tool batches, and stops on required-step failure. |
| Evaluation | Eval; also ShipCheck, Fence, ChangeBucket, custom tools | Each tool has its own result shape and exit semantics. There is no shared evaluation-result contract. |
| Workflow policy | Dispatch workflow definitions and runner logic | Approval, route-review, optional/required failure, budget, and cancellation behavior are built into step handling. There is no general outcome-to-control policy mapper. |
| Current workflow state | Dispatch `state.json` | Mutable, checkpointed run snapshot with step state and bounded traces. |
| Human approval and intervention | Dispatch plus Leash | Dispatch creates intervention events and consumes decisions. Leash durably stores and claims decisions and delivers resume callbacks. |
| Isolated workspace execution | Workcell | Local Docker/Podman v1 and provider-neutral remote v2alpha1 execution, collection, verification, export, receipt, manifest, and cleanup. |
| Workspace preservation | Workcell | Local `keep_failed` can retain filesystem state; the container/process is still destroyed. Remote finalization destroys owned resources and relies on exported evidence unless a backend-specific capability is used. |
| Evidence and integrity | Each producer | Eval manifests, Workcell receipts/manifests, Dispatch traces/state, RunLedger receipts, CaseFile bundles, Watchdog events, and tool-specific JSON remain independent. |
| Run receipts | RunLedger | Human/automation-authored run status and correlation references. It does not judge or orchestrate execution. |
| Failure bundles | CaseFile | Redacted, repository-scoped failure evidence and reproduction handoff. It is not a workflow state machine. |
| Telemetry | Watchdog | Fail-open observability events and references. It is not authoritative workflow state or an audit store. |
| Quiet command execution | Muzzle | Structured command status, logs, timeout, and cancellation. It is not an orchestrator. |
| Change/release/architecture gates | ChangeBucket, ShipCheck, Fence | Useful machine results and exit codes with different schemas and severity rules. |
| Agent runs and approvals | Agents SDK | Agent status, errors, artifacts, lifecycle events, and approval-provider adapters. Persistence and workflow resume remain external. |
| Model/provider retry and fixture replay | AI SDK and Kujo AI runtime | Provider normalization, retry/backoff, redaction, and cassette/fixture replay. They do not own workflow progression. |
| Cleanup and retention | Workcell/operator; individual tools | Cleanup is product-specific. Cross-tool retention ownership and expiry are not normalized. |
| External side effects | Tool and remote system | Dispatch supplies stable effect keys and guidance; exactly-once behavior requires sink enforcement or a transactional outbox. |

### Present control flow

```text
Dispatch step
  -> tool/agent/approval/route executes
  -> Dispatch interprets success or failure
  -> step and run snapshot are persisted
  -> next dependency-ready step is scheduled, or run ends/pauses

Eval CLI
  -> evaluates local inputs/state
  -> writes reports, summaries, history, and manifest
  -> exits 0 or 1

Workcell
  -> prepare -> launch -> execute -> collect -> verify -> export
  -> write receipt and integrity manifest
  -> destroy runtime resource
  -> clean or preserve eligible local workspace files
```

These flows can be invoked together, but their control meanings are currently
connected through exit status and workflow-specific interpretation rather
than a shared contract.

## 3. Verified current behavior

The following labels are deliberate:

- **Documented** means prose promises it.
- **Implemented** means source contains the behavior.
- **Tested** means repository tests assert the behavior.
- **Assumed** means a plausible integration expectation lacks a contract or
  end-to-end proof.
- **Missing** means the reviewed source does not provide it.

| Behavior | Documented | Implemented | Tested | Classification and evidence |
|---|---:|---:|---:|---|
| Eval writes machine artifacts and exits nonzero when checks fail | Yes | Yes | Yes | Verified in `eval/main.kujo`, `src/eval_core.kujo`, and CLI/artifact tests. The suite result itself may be structurally `ok: true` while failed counts drive process exit 1. |
| Eval directly places Dispatch into review | No | No | No | Missing. Eval has no Dispatch dependency or orchestration callback. |
| Dispatch required-step failure fails closed | Yes | Yes | Yes | Runner marks the step and run `failed`, persists, finalizes trace, and stops scheduling. |
| Dispatch `failed` equals `paused` | No | No | Yes | Explicitly false. `failed` is terminal and not accepted by `can_resume_status`; `paused`, `interrupted`, and `needs_changes` are resumable. |
| Dispatch approval/review can pause and resume | Yes | Yes | Yes | Approval and route-human-review paths create an intervention, persist paused state, and consume single-use decisions. |
| Every abnormal result can use that intervention path | Implied in some integration prose | No | No | Missing. Ordinary tool, Eval, verification, and policy failures follow failure logic unless authored as an approval/review step. |
| Dispatch can guarantee exactly-once external effects | No | No | No | Correctly documented as at-least-once across the crash-before-checkpoint window. Stable effect keys only help when the sink enforces them. |
| Dispatch has a complete append-only lifecycle record | No | No | No | Missing. Trace is bounded and may truncate. `dispatch-mutations.jsonl` records selected repair/import/cleanup mutations, not every transition. |
| Parallel failure prevents all sibling effects | No | No | No | Not guaranteed. A prepared parallel batch launches eligible idempotent, parallel-safe tool steps together; already-started siblings may complete before a failure is processed. |
| Local Workcell `keep_failed` preserves filesystem state | Yes | Yes | Yes | Verified. It changes cleanup of the workspace directory. |
| Local Workcell `keep_failed` freezes the container/process | No | No | No | Explicitly false. The coordinator destroys the container after execution. Preservation is filesystem state, not live runtime state. |
| Remote Workcell preserves failed live resources | Not generally promised | No, by default | No | Missing as a portable guarantee. Remote finalization destroys an owned handle; optional backend pause/snapshot capabilities are not canonical lifecycle guarantees. |
| Workcell produces tamper-detectable evidence | Yes | Yes | Yes | Receipt and manifest cover run artifacts; manifest verification catches missing or modified files. |
| Workcell v2 supports provider-neutral lifecycle operations | Yes | Yes | Yes | Required operations include resolve/provision/prepare/execute/cancel/collect/export/destroy/inventory. Pause/resume/snapshot are optional. |
| Leash decisions are durable and duplicate-safe | Yes | Yes | Yes | SQLite claims, decision receipts, and resume outbox provide compare-and-swap, single-use delivery semantics. |
| Leash enforces Eval failures as documented | Yes, in `docs/eval-integration.md` | No | No | Contradiction. The current policy labels the Eval gate a stub, always allows it, and passes `eval_context: null`. The documentation overstates integration. |
| RunLedger is an authoritative workflow controller | No | No | No | Correctly absent. It records run receipts and correlations. |
| Watchdog is an authoritative audit/control plane | No | No | No | Correctly absent. It is optional, fail-open telemetry. |
| The Agents SDK has useful generic result primitives | Yes | Yes | Yes | Run status, retryable errors, artifacts, lifecycle events, and approval provider boundaries exist, but no persisted generic review boundary. |
| Kujo can deterministically replay arbitrary AI/API behavior | No | No | No | Correctly absent. Strict recorded cassettes can make a bounded AI interaction hermetic; live providers and arbitrary external effects cannot be promised deterministic. |

### Verified distinction: preservation versus suspension

The current Workcell meaning is:

```text
preserved workspace = files retained or exported after execution
live suspended runtime = process/container/provider resource remains resumable
```

Only the first is a normal local guarantee. Optional remote `pause`, `resume`,
or `snapshot` operations must advertise their exact semantics. A disk snapshot
does not imply open-process, memory, socket, or clock continuity.

### Verification limits

The review inspected implementation and tests across the named repositories.
A direct Dispatch test invocation from its repository root did not complete
within a bounded interactive review window and was interrupted; this report
therefore does not claim a fresh full Dispatch suite pass. The relevant tests
and runner/state implementation were inspected directly. No claim here relies
on the earlier, invalid invocation from the Kujo repository, where Dispatch
module resolution predictably failed because the working directory was wrong.

## 4. Gap analysis

### Gap 1: no shared result and evidence vocabulary

**Gap:** Eval, ShipCheck, Fence, ChangeBucket, Muzzle, Workcell, Dispatch, and
the Agents SDK expose different status and artifact shapes.

**Why it matters:** An orchestrator must understand each producer or reduce it
to an exit code, losing failure class, evidence pointers, integrity, and safe
next actions.

**Current workaround:** Product-specific parsers, shell exit handling, and
workflow-specific glue.

**Severity:** High for ecosystem composability; low for standalone tools.

**Affected components:** Shared contracts package, Dispatch, Eval, Workcell,
ShipCheck, Fence, ChangeBucket, Muzzle, Agents SDK, third-party producers.

### Gap 2: terminal failure and reviewable boundary are not policy-connected

**Gap:** Dispatch has both failure and intervention mechanisms, but ordinary
execution/evaluation failure cannot be mapped to the latter through a generic
policy contract.

**Why it matters:** A failed postcondition cannot stop future work, preserve a
review handoff, and wait for a typed decision without modeling the evaluator
as a bespoke approval step.

**Current workaround:** Author explicit approval steps, wrap tools, or terminate
and manually start a new run.

**Severity:** High.

**Affected components:** Dispatch and Leash; producers need only emit standard
results.

### Gap 3: resume and retry actions are too approval-specific

**Gap:** Current decisions cover approval/rejection/request-changes/reply, but
do not distinguish continuing past a failed gate, rerunning only an evaluator,
rerunning an action in retained state, or rebuilding from clean state.

**Why it matters:** An ambiguous `resume` can duplicate an external effect or
rerun the wrong attempt.

**Current workaround:** Workflow-specific follow-up messages or a new run.

**Severity:** High where tools have effects; medium for pure computations.

**Affected components:** Dispatch, Leash, workflow authors, Agents SDK adapters.

### Gap 4: audit history is incomplete for control decisions

**Gap:** `state.json` is mutable, traces are bounded, and the mutation log does
not record all lifecycle transitions.

**Why it matters:** A reviewer may not be able to reconstruct the exact policy
input, decision, evidence set, state revision, and intervention that formed a
boundary.

**Current workaround:** Correlate snapshots, trace fragments, webhook JSONL,
and external logs.

**Severity:** Medium. It becomes high only for regulated or security-sensitive
workflows.

**Affected components:** Dispatch; Watchdog may mirror events but must not be
the authoritative source.

### Gap 5: late evaluation and remote cleanup conflict

**Gap:** A remote Workcell may be destroyed before a later evaluator asks that
its state be preserved. Current remote finalization does not make
`keep_failed` a provider-neutral retained-workspace guarantee.

**Why it matters:** A workflow cannot resurrect a deleted remote filesystem.
Preservation must be anticipated or reconstructed from exported evidence.

**Current workaround:** Declare and export artifacts/changes before destroy,
use backend-specific snapshots, or keep a resource outside the normal
lifecycle.

**Severity:** High for same-workspace resume; low for evidence-only review.

**Affected components:** Workcell, Dispatch workflow policy, remote adapters.

### Gap 6: effect safety is guidance, not a normalized retry input

**Gap:** Dispatch supports idempotency keys and documents crash semantics, but
there is no portable effect classification or completion state that policy can
use to permit or forbid retry.

**Why it matters:** Failure does not imply that nothing happened. Automatic
retry of a payment, deployment, message, or destructive mutation can be
unsafe.

**Current workaround:** Tool-specific idempotency, workflow conventions, and
manual review.

**Severity:** High for external non-idempotent effects; low for pure/local
steps.

**Affected components:** Shared contracts, Dispatch, tool adapters, external
systems.

### Gap 7: retention and preservation ownership are not cross-tool concepts

**Gap:** Evidence and retained workspaces do not share expiry, owner,
sensitivity, or deletion-status metadata.

**Why it matters:** Failed state can retain secrets, customer data, malicious
artifacts, and expensive remote resources indefinitely.

**Current workaround:** Per-tool cleanup and operator procedures.

**Severity:** Medium.

**Affected components:** Shared evidence contract, Workcell, Dispatch, remote
providers, evidence stores.

### Gap 8: documented Leash Eval enforcement is not implemented

**Gap:** Leash documentation presents an Eval gate that current policy source
explicitly marks as a permissive stub.

**Why it matters:** Operators may believe approval is blocked by evaluation
when no such enforcement exists.

**Current workaround:** External policy or manual inspection.

**Severity:** Medium because it is a claim/expectation mismatch, not a hidden
runtime regression.

**Affected components:** Leash documentation immediately; Leash policy and
Eval adapter in later phases.

## 5. Recommended architecture

The architecture should separate observation, judgment, policy, and control:

```text
Action producer
  -> ExecutionResult --------------------.
  -> EvidenceRef(s)                       |
                                         v
Evaluator(s) -> EvaluationResult(s) -> Policy resolver
                                         |
                                         v
                                  PolicyDecision
                                         |
                                         v
                              Orchestrator admission gate
                           /       |        |       \
                    continue     retry    pause     fail/cancel
                                           |
                                           v
                              PreservationOutcome
                                           |
                                           v
                              InterventionRequest
                                           |
                                           v
                              InterventionDecision
                                           |
                                           v
                               explicit continuation plan
```

No arrow requires Eval to import Dispatch or Workcell. Results and references
are data contracts. Policy can be embedded in a Dispatch workflow, supplied by
an external engine, or implemented by another orchestrator. Workcell accepts a
preservation request and reports what it actually preserved; it does not
decide whether the workflow should pause. Leash transports and authorizes a
decision; it does not evaluate the action.

### Design rules

1. **Results describe; policy decides.** An evaluator reports `fail`; it does
   not command `pause`. A policy can map the same failure to pause, terminal
   failure, retry, or continuation with warning.
2. **The scheduler owns quiescence.** Only the orchestrator knows which DAG
   nodes are pending or running and can enforce a scheduling barrier.
3. **Preservation reports actuality.** A request for a snapshot is not proof
   that a snapshot exists. The provider returns a typed outcome and evidence
   reference.
4. **Interventions are revision-bound.** A decision applies to one boundary
   and one state revision. Replays and duplicate callbacks are idempotently
   rejected or return the prior result.
5. **Snapshots and journals have different jobs.** `state.json` remains the
   current operational state. A narrow append-only journal records important
   control transitions and their hashes.
6. **Backward compatibility is adapter-based.** An exit code remains valid.
   Dispatch can wrap legacy tool results into a minimal `ExecutionResult`.
7. **No control service is required.** The contracts belong in a small shared
   Kujo package with JSON Schemas, conformance fixtures, and helper functions,
   not a mandatory hosted control plane.

### Where policy belongs

Policy is a shared contract plus an orchestrator integration point:

- Dispatch should include a small deterministic mapper for common rules in a
  workflow definition.
- An external policy engine may produce the same `PolicyDecision` contract.
- A future Kujo Control package may provide reusable policies, but it should
  not become required infrastructure.
- Eval, Workcell, Watchdog, and the Kujo runtime should not own workflow
  policy.

## 6. Proposed contracts

Use a small package tentatively named `kujo-control-contracts`. Names below
are contract identifiers rather than Rust or Kujo types. Each JSON document
must include `schema`, and unknown additive fields must be ignored within a
major version. Breaking semantic changes require a new major version.

### 6.1 `kujo.evidence-ref/v1`

**Owner:** shared contracts package.

**Purpose:** Refer to an independently stored artifact without embedding its
contents into every result.

**Required fields:** `schema`, `id`, `producer`, `subject`, `kind`, `uri`,
`media_type`, `created_at`, and `integrity`.

**Optional fields:** byte count, artifact schema, manifest reference,
correlation/causation IDs, sensitivity, redaction status, retention metadata,
and human label.

**Producers:** every evidence-producing tool.

**Consumers:** evaluators, orchestrators, policy engines, review systems,
telemetry, and humans.

**Compatibility impact:** additive. Existing paths and manifests remain
unchanged and receive reference adapters.

```json
{
  "schema": "kujo.evidence-ref/v1",
  "id": "ev_01K5Y4M8YQ6P",
  "producer": {"name": "workcell", "version": "2.0.0-alpha.1"},
  "subject": {
    "run_id": "run_203",
    "step_id": "edit",
    "attempt_id": "attempt_1"
  },
  "kind": "changes.patch",
  "uri": "artifact://run_203/edit/attempt_1/changes.patch",
  "media_type": "text/x-diff",
  "created_at": "2026-09-24T14:32:05Z",
  "integrity": {
    "status": "sha256",
    "sha256": "d8f9a8b7d59e8c03a3f1d9c7c19f4d2e75a0db5a2e3bb7b87dfd1f0a68f07219"
  },
  "bytes": 4821,
  "artifact_schema": null,
  "sensitivity": "internal",
  "redaction": {"status": "applied", "profile": "workcell-default/v1"},
  "retention": {
    "owner": "team-build",
    "policy": "failed-run-30d",
    "retain_until": "2026-10-24T14:32:05Z"
  }
}
```

`uri` is opaque to generic consumers. Resolvers must allowlist schemes and
apply path/host authorization. Integrity may be `sha256`, `manifest`, or
`unverified`; it must never imply verification that did not occur.

### 6.2 `kujo.execution-result/v1`

**Owner:** shared contracts package.

**Purpose:** Describe what an action attempt did, independently from the
workflow transition chosen afterward.

**Required fields:** schema, result/subject IDs, producer, status, timestamps,
attempt, failure classification when non-successful, effects, and evidence.

**Optional fields:** normalized output, retry hint, metrics, warnings,
correlations, and a re-execution descriptor reference.

**Producers:** Dispatch adapters, Workcell, Muzzle, Agents SDK, tool wrappers.

**Consumers:** evaluators, policy resolvers, orchestrators, RunLedger,
Watchdog.

**Compatibility impact:** additive. Legacy exit codes are wrapped, not removed.

```json
{
  "schema": "kujo.execution-result/v1",
  "result_id": "xr_01K5Y4P2QJ3T",
  "subject": {"run_id": "run_203", "step_id": "deploy", "attempt_id": "a1"},
  "producer": {"name": "custom-deployer", "version": "4.2.0"},
  "status": "failure",
  "classification": "verification_failure",
  "started_at": "2026-09-24T14:31:00Z",
  "finished_at": "2026-09-24T14:31:14Z",
  "attempt": 1,
  "retry": {"disposition": "operator_required", "reason": "remote effect may have committed"},
  "effects": [{
    "effect_id": "effect_run203_deploy_a1",
    "class": "external_idempotent",
    "state": "unknown",
    "idempotency_key": "deploy:service-a:revision-551",
    "enforced_by": "remote_service"
  }],
  "evidence": [
    {"$ref": "ev_01K5Y4M8YQ6P"}
  ]
}
```

Recommended `status` values are `success`, `failure`, `timeout`, `cancelled`,
and `indeterminate`. `classification` is orthogonal and includes at least
`execution_failure`, `infrastructure_failure`, `verification_failure`,
`artifact_failure`, `policy_denial`, `unsafe_output`, and `unknown`. Producers
must not label a result retryable merely because a process returned nonzero.

### 6.3 `kujo.evaluation-result/v1`

**Owner:** shared contracts package; Eval is the reference producer.

**Purpose:** Report judgment about a subject without selecting workflow
control.

**Required fields:** schema, result ID, evaluator identity, subject, verdict,
severity, checks summary, timestamps, and evidence.

**Optional fields:** bounded check details, configuration/policy digest,
confidence, annotations, and input evidence IDs.

**Producers:** Eval, ShipCheck, Fence, ChangeBucket, security scanners, custom
evaluators.

**Consumers:** policy engines, CI, Dispatch, custom orchestrators, humans.

**Compatibility impact:** additive. Eval's existing summary and exit code stay
stable.

```json
{
  "schema": "kujo.evaluation-result/v1",
  "result_id": "er_01K5Y4R0B5SN",
  "evaluator": {
    "name": "eval",
    "version": "1.5.0",
    "configuration_sha256": "2dbf69e99af54159f50d2f61526bd4d4f9f45e10a585529799b2df36ab3bb4a2"
  },
  "subject": {"run_id": "run_203", "step_id": "edit", "attempt_id": "a1"},
  "verdict": "fail",
  "severity": "error",
  "checks": {"passed": 18, "failed": 2, "warned": 0, "skipped": 1},
  "started_at": "2026-09-24T14:32:08Z",
  "finished_at": "2026-09-24T14:32:15Z",
  "input_evidence_ids": ["ev_01K5Y4M8YQ6P"],
  "evidence": [
    {"$ref": "ev_eval_report_203"},
    {"$ref": "ev_eval_manifest_203"}
  ],
  "annotations": [{"code": "tests.failed", "message": "2 contract tests failed"}]
}
```

Verdicts are `pass`, `fail`, `warn`, and `indeterminate`. Severity is a
separate ordered label (`info`, `warning`, `error`, `critical`). There is no
`pause`, `retry`, or `continue` field: those are policy decisions.

### 6.4 `kujo.policy-decision/v1`

**Owner:** shared contracts package. The policy implementation belongs to the
workflow deployment or an external engine.

**Purpose:** Map normalized facts to an enforceable control decision.

**Required fields:** schema, decision ID, policy identity/version/digest,
input result IDs/digests, disposition, scope, in-flight behavior, reason,
timestamp, and permitted continuation actions when intervention is required.

**Optional fields:** preservation requirement, retention policy, effect
constraints, deadline, and evidence.

**Producers:** Dispatch's deterministic mapper or an external policy engine.

**Consumers:** any orchestrator implementing the admission/control contract.

**Compatibility impact:** new and optional. Existing Dispatch required/optional
failure rules are expressed as an implicit legacy policy.

```json
{
  "schema": "kujo.policy-decision/v1",
  "decision_id": "pd_01K5Y4S91DNP",
  "policy": {
    "id": "code-change-gates",
    "version": "3",
    "sha256": "1d35ce6d7a83dfa3c3fe2a6351860961fb93f36dc69e46d797cc93fd4ac76b5a"
  },
  "inputs": ["er_01K5Y4R0B5SN", "xr_01K5Y4P2QJ3T"],
  "disposition": "require_intervention",
  "reason": {"code": "evaluation_failed", "message": "Required checks failed"},
  "scope": {"kind": "descendants", "root_step_id": "edit"},
  "in_flight": "allow_finish",
  "preservation": {
    "required": true,
    "acceptable_modes": ["handoff_bundle", "snapshot"],
    "retention_policy": "failed-run-30d"
  },
  "allowed_actions": [
    "retry_evaluation",
    "retry_step",
    "retry_clean",
    "amend_inputs",
    "approve_override",
    "abort"
  ],
  "decided_at": "2026-09-24T14:32:16Z"
}
```

Dispositions are `continue`, `retry`, `require_intervention`, `fail`, and
`cancel`. `scope.kind` is `step`, `descendants`, `branch`, or `workflow`.
`in_flight` is `allow_finish`, `cancel_cooperative`, or `terminate`. A policy
must state these choices; the runtime must not infer that “stop” means kill.

### 6.5 `kujo.intervention-request/v2` and
`kujo.intervention-decision/v2`

**Owner:** shared contract with Leash as reference transport and Dispatch as a
reference consumer.

**Purpose:** Present a reviewable boundary and return a single explicit,
authorized continuation decision.

**Required request fields:** request/boundary IDs, run and state revision,
reason, summary, policy decision reference, evidence references, preservation
outcome, allowed actions, creation/expiry, and callback target.

**Required decision fields:** decision ID, request/boundary IDs, expected state
revision, action, actor/authentication metadata, reason, timestamp, and an
action-specific continuation plan.

**Optional fields:** amended input reference, comment, authorization claims,
and review evidence.

**Producers:** orchestrators and authorized review clients.

**Consumers:** Leash-like transports and orchestrators.

**Compatibility impact:** additive v2. Leash v1 approval actions map to v2
actions during migration.

```json
{
  "schema": "kujo.intervention-decision/v2",
  "decision_id": "id_01K5Y5A19HWW",
  "request_id": "ir_01K5Y4V1JAZ5",
  "boundary_id": "boundary_run203_7",
  "expected_state_revision": 42,
  "action": "retry_clean",
  "continuation": {
    "target_step_id": "edit",
    "source": "original_source",
    "reuse_inputs": true,
    "reuse_effects": false,
    "new_attempt": true
  },
  "actor": {
    "id": "user_416",
    "type": "human",
    "authenticated_by": "oidc",
    "authorization": "release-reviewer"
  },
  "reason": "Retry from the recorded source revision after fixing the toolchain image",
  "decided_at": "2026-09-24T15:02:00Z"
}
```

Allowed action names are `resume_from_boundary`, `retry_evaluation`,
`retry_step`, `retry_clean`, `amend_inputs`, `approve_override`, `abort`, and
`cancel`. A workflow may expose only a subset. `approve_override` requires a
reason and policy authorization. `resume_from_boundary` never reruns the
action that produced the boundary.

### 6.6 `kujo.preservation-outcome/v1`

**Owner:** shared contract; Workcell is the reference producer.

**Purpose:** State what execution state was actually retained, for how long,
and how it can be accessed or cleaned.

**Required fields:** schema, request/result IDs, subject, requested mode,
actual mode, status, provider, created/expiry timestamps, evidence refs, and
cleanup ownership.

**Optional fields:** opaque snapshot handle, reconstructability, limitations,
and deletion receipt.

```json
{
  "schema": "kujo.preservation-outcome/v1",
  "result_id": "po_01K5Y4TMN71K",
  "subject": {"run_id": "run_203", "step_id": "edit", "attempt_id": "a1"},
  "requested_mode": "snapshot",
  "actual_mode": "handoff_bundle",
  "status": "degraded",
  "provider": {"name": "remote-provider-x", "workspace_id": "opaque:9c41"},
  "reconstructability": "clean_source_plus_patch",
  "evidence": [{"$ref": "ev_workcell_handoff_203"}],
  "limitations": ["process memory and open sockets were not preserved"],
  "created_at": "2026-09-24T14:32:17Z",
  "retain_until": "2026-10-24T14:32:17Z",
  "cleanup": {"owner": "workcell-gc", "handle": "cleanup:run203:a1"}
}
```

`actual_mode` is `none`, `filesystem`, `handoff_bundle`, `snapshot`, or
`live_pause`. `status` is `satisfied`, `degraded`, `unsupported`, or `failed`.
A policy that requires a snapshot may fail closed when the result is merely a
handoff bundle.

### 6.7 `kujo.reexecution-descriptor/v1`

**Owner:** shared contracts package; action producers populate it when safe.

**Purpose:** Describe whether and how an attempt can be run again. It is not a
promise that output will match.

Required fields include source/workflow/tool/configuration identities and
digests, normalized input references, dependency lock/image digest where
applicable, declared secret names or secret-store references, effect
constraints, and replay capability. Raw credentials, tokens, and sensitive
ambient environment values are forbidden.

Capability is explicit:

```json
{
  "schema": "kujo.reexecution-descriptor/v1",
  "capability": "re_executable",
  "source": {"repository": "kujo", "commit": "abc123", "dirty_patch_ref": "ev_patch"},
  "workflow": {"id": "agent-change", "version": "7", "sha256": "..."},
  "tool": {"name": "coding-agent", "version": "2.4.1", "image_digest": "sha256:..."},
  "inputs": [{"name": "task", "evidence_ref": "ev_task_input"}],
  "secret_refs": ["secret://provider/api-key"],
  "external_dependencies": [{"name": "model", "mode": "live", "replayable": false}],
  "effects": {"automatic_retry_allowed": false}
}
```

Capability values are `inspectable`, `re_executable`, and
`deterministic_replay`. Producers must select the strongest proven guarantee,
not the desired one.

## 7. State machine

Keep the durable top-level workflow state small:

```text
pending -> running -> completed
              |  \
              |   -> failed
              |   -> cancelled
              |   -> paused <----.
              |         |         |
              |         +-> running  (accepted continuation plan)
              |         +-> failed   (abort/reject)
              |         +-> cancelled
              '----------------------' (new boundary may occur later)
```

Top-level states:

- `pending`: admitted but not started.
- `running`: the scheduler may admit work not blocked by dependencies or a
  control barrier.
- `paused`: no new work in the declared scope may start; a reason, boundary,
  state revision, allowed actions, and evidence set are required.
- `completed`: all required work completed under policy.
- `failed`: terminal failure with no accepted continuation path.
- `cancelled`: terminal operator/policy cancellation.

`failed` must remain distinct from `paused`. `retryable`, `needs_changes`,
`awaiting_review`, `timeout`, and `blocked` should not become additional
top-level states:

- retryability is a result/policy attribute;
- `needs_changes` becomes a legacy projection of `paused` with reason
  `changes_requested`;
- awaiting review is `paused` plus an open intervention;
- timeout is an execution result classification and may map to pause or fail;
- blocked is a scheduler condition on a step/branch, not a workflow lifecycle
  state.

### Transitions

| From | Event/decision | To | Required behavior |
|---|---|---|---|
| pending | run admitted | running | Persist initial revision and run-started journal entry. |
| running | all required steps satisfy policy | completed | Persist final evidence refs and terminal entry. |
| running | terminal failure policy | failed | Stop new work in scope, record result/policy, and finalize. |
| running | cancellation | cancelled | Stop admission, apply declared in-flight behavior, record outstanding effect uncertainty. |
| running | intervention policy | paused | Execute the boundary checkpoint protocol below. |
| paused | valid `resume_from_boundary` or approved override | running | CAS expected revision, consume decision once, leave producing attempt intact, and schedule only newly eligible work. |
| paused | valid retry action | running | Create a new attempt with explicit source/workspace/evaluator semantics. Never overwrite the prior attempt. |
| paused | amend inputs | running | Persist new input revision/reference, create new attempt, and retain prior evidence. |
| paused | abort/reject | failed | Persist actor, reason, decision, and terminal evidence. |
| paused | cancel | cancelled | Persist cancellation and cleanup policy. |
| any nonterminal | infrastructure timeout/cancel | policy-dependent | Normalize the result first; policy chooses pause, fail, retry, or cancel. |

### Boundary checkpoint protocol

For a reviewable abnormal condition, Dispatch or another orchestrator must:

1. Record the producing action result and evidence references.
2. Record every evaluation result consumed by policy.
3. Resolve and validate the policy decision.
4. Close the scheduler admission gate for the declared scope.
5. Atomically persist a new state revision containing the boundary, decision,
   and allowed actions.
6. Apply the declared behavior to work already in flight.
7. Request preservation and record the actual preservation outcome.
8. Append control-journal entries in sequence.
9. Emit an intervention request only after the review state is durable.

If preservation is required and cannot be satisfied, the policy defines
whether to fail closed, remain paused with degraded evidence, or abort. The
runtime must not silently claim successful preservation.

### DAG and concurrency semantics

Quiescence has two independent dimensions:

- **Scope:** `step`, `descendants`, `branch`, or `workflow`.
- **Already running work:** `allow_finish`, `cancel_cooperative`, or
  `terminate`.

For `A -> {B, C}` with an evaluator downstream of B, a descendant-scoped gate
blocks only descendants of B; C may continue. A workflow-scoped gate blocks
all new scheduling. In either case, C may already be running because Dispatch
launches eligible parallel batches together. Policy must explicitly choose
whether to allow it to finish or attempt cancellation. Completed effects are
never undone merely because another branch later fails.

## 8. Evidence architecture

Evidence remains a collection of independently useful artifacts. Do not
replace `receipt.json`, `state.json`, `trace.json`, `changes.patch`, Eval
reports, manifests, CaseFiles, model responses, or tool-specific results with
one giant workflow document.

### Production and correlation

- Every attempt receives stable run, step, attempt, correlation, and causation
  identifiers before execution.
- Producers write their native artifacts and return `EvidenceRef` records.
- Manifests refer to artifacts; results refer to evidence; policy decisions
  refer to the result IDs and hashes they actually consumed.
- Dispatch stores compact references and summaries in `state.json`, not large
  payloads.
- RunLedger and Watchdog may record the same IDs for discovery, but neither is
  the authoritative resolver.

### Integrity

- Reuse Workcell's complete run-directory manifest and Eval's optional SHA-256
  artifact manifest.
- Hash immutable artifacts at production or export. Use a manifest reference
  for a bundle rather than copying every digest into workflow state.
- Before a sensitive resume/retry, verify required evidence and manifest
  integrity. A missing or tampered required artifact produces a new
  `indeterminate`/`artifact_failure` result and returns to policy.
- Do not treat a path, `exists: true`, or transport TLS as content integrity.

### Retention and inspection

- Each retained reference carries an owner, policy name, and expiry when
  applicable.
- Evidence resolvers enforce authorization, sensitivity, redaction status,
  allowed URI schemes, path confinement, and audit access.
- Expiry produces a deletion receipt where the provider supports proof. An
  unavailable provider is not proof of deletion.
- A paused boundary whose required evidence expires must transition only via
  policy; it must not silently resume.

### Control journal

Dispatch should add `control-events.jsonl`, containing only durable
control-relevant facts:

```json
{
  "schema": "kujo.control-event/v1",
  "sequence": 17,
  "event_id": "ce_01K5Y4V3A2QM",
  "previous_sha256": "...",
  "event_sha256": "...",
  "run_id": "run_203",
  "state_revision": 42,
  "kind": "workflow_paused",
  "subject": {"step_id": "edit", "attempt_id": "a1"},
  "refs": ["pd_01K5Y4S91DNP", "po_01K5Y4TMN71K", "ir_01K5Y4V1JAZ5"],
  "occurred_at": "2026-09-24T14:32:18Z"
}
```

Events include action/evaluation completion, policy decision, admission gate,
preservation outcome, pause, intervention request/decision, resume/retry,
cancellation, and terminal outcome. Sequence and hash linkage expose deletion
or reordering within an exported journal. `state.json` remains the recovery
source of truth; this is not event sourcing, does not replay business logic,
and does not replace bounded diagnostic traces.

## 9. Replay model

Kujo should use three precise guarantees.

### Inspectable

A reviewer can resolve authorized inputs, outputs, logs, diffs, receipts,
policy/evaluation results, manifests, and decisions. This is the minimum
guarantee for a reviewable boundary and should be required whenever policy
pauses for review.

### Re-executable

The system has enough nonsecret configuration and immutable references to
create a new attempt: source revision and patch, workflow/policy/tool versions
or digests, normalized inputs, dependency locks or image digest, environment
and secret names/references, correlation IDs, and effect constraints.
Re-execution may produce a different result.

### Deterministically replayable

Re-executing the captured operation produces the same relevant behavior under
a declared deterministic boundary. This is available only for hermetic,
pinned tools and recorded external interactions. Kujo AI strict replay
cassettes can qualify for their bounded calls. Live LLM calls, time, random,
mutable package sources, network APIs, and external side effects do not.

Kujo should guarantee inspectability for review boundaries, support
re-executability when a valid descriptor exists, and claim deterministic
replay only after a conformance test proves it. Logs alone satisfy none of the
latter two guarantees.

### Safe snapshot contents

Capture or reference:

- source commit, dirty patch, and workspace digest;
- workflow, tool, evaluator, policy, and schema versions/digests;
- normalized step inputs and declared outputs;
- dependency lockfiles and container image digest;
- model/provider identifiers and captured response/cassette references;
- evidence, manifest, correlation, causation, and effect IDs;
- nonsecret environment variable names and secret-store references;
- execution capabilities and provider/backend metadata.

Do not capture raw credentials, bearer tokens, private keys, secret values,
ambient host environment dumps, or unreviewed sensitive prompts. A secret
reference demonstrates dependency identity, not permission to fetch or reuse
the secret.

## 10. Side-effect model

Execution control and effect rollback are separate. Stopping Dispatch cannot
unsend an email, reverse a payment, delete a posted message, undo a database
mutation, retract a deployment, restore a remote file, or remove a created
issue unless a separate compensating operation is defined and succeeds.

### Effect declaration

Add effect metadata only because it changes runtime retry behavior:

```json
{
  "class": "external_non_idempotent",
  "effect_id": "effect_run203_charge_a1",
  "state": "unknown",
  "idempotency": {
    "key": null,
    "scope": null,
    "enforced_by": null
  },
  "compensation": {
    "supported": true,
    "operation": "refund",
    "automatic": false
  }
}
```

Classes are:

- `none`: computation or observation with no declared mutation;
- `local_reversible`: local effect with a tested restore/cleanup operation;
- `external_idempotent`: remote sink enforces the key within a documented
  scope/window;
- `external_non_idempotent`: remote mutation may duplicate;
- `destructive`: irreversible or high-impact operation requiring explicit
  authorization.

The declaration must affect scheduling:

- automatic retries are allowed for `none` and may be allowed for proven
  `local_reversible` or `external_idempotent` effects;
- `external_non_idempotent` and `destructive` attempts require policy and
  operator authorization before retry when completion is uncertain;
- parallel automatic execution continues to require explicit safety and a
  stable idempotency key where effects exist;
- retry policy must distinguish `not_started`, `started`, `committed`,
  `compensated`, `failed_before_commit`, and `unknown` when the tool can prove
  them.

### Crash-before-checkpoint

For:

```text
remote effect commits -> process crashes -> result checkpoint is absent
```

Dispatch records an `indeterminate` result with effect state `unknown`. It may
query the sink by effect/idempotency key if the tool supports reconciliation.
Otherwise it must not retry automatically. Dispatch's run lock and local
idempotency cache cannot provide exactly-once semantics across this window.
Exactly-once requires the remote sink to enforce the key or a transactional
outbox/inbox spanning the business write.

### Compensation

Compensation is a new, auditable effect with its own authorization, result,
evidence, idempotency key, and possible failure. Kujo should orchestrate a
declared compensating action but must not label it rollback or assume it
restores the full prior world state. Prepare/commit protocols and dry runs
remain tool-specific mechanisms exposed through the common result/effect
metadata.

## 11. Workcell changes

Workcell should remain a general execution environment. It should not acquire
workflow policy, evaluator knowledge, or a human-review state machine.

### Required

1. Emit `ExecutionResult`, `EvidenceRef`, and `PreservationOutcome` adapters for
   existing receipt, logs, changes, artifacts, verification, and manifest.
2. Accept preservation intent before cleanup, including required/acceptable
   modes, expiry, owner, and whether failure to preserve is fatal.
3. Persist actual preservation mode and limitations in the receipt. Keep the
   existing `cleanup_status`, but do not use `preserved` to imply a live
   process.
4. Expose provider capabilities before execution so policy can reject a
   workflow that requires unsupported snapshot or pause semantics.
5. Ensure remote finalization resolves preservation before destroying an
   owned resource. It must still destroy on normal completion unless policy
   explicitly requested and authorized bounded retention.

Likely modules: `src/execution/coordinator.kujo`, portable coordinator and
receipt code, domain definitions, backend capability types/adapters, manifest
helpers, schemas, and lifecycle/remote-provider tests.

### Recommended

- Add a bounded `handoff_bundle` export containing source identity, patch,
  change metadata, declared artifacts, stdout/stderr, receipt, manifest,
  provider metadata, and re-execution descriptor. Prefer reconstruction from
  clean source plus patch over a raw entire-filesystem archive.
- Add retention/cleanup handles, expiry scanning, and deletion receipts.
- Add capability conformance tests for `snapshot` and `live_pause`; document
  whether each captures disk, memory, processes, sockets, or none of those.
- Permit a provider-neutral workspace bundle only after path, size,
  sensitivity, symlink, device-file, and secret scans are enforced.

### Optional

- Native provider snapshots where the backend exposes durable snapshot IDs.
- Live pause/resume for backends that can prove lifecycle and expiry behavior.
- A reviewer mount/view that is read-only and non-executable by default.

### Not needed

- Dispatch status transitions inside Workcell.
- Eval-specific fields in Workcell receipts.
- A policy engine or approval queue.
- A promise to keep a container/process alive under `keep_failed`.
- Automatic indefinite retention.

### Important timing constraint

An evaluation may run after the Workcell command has completed. If the normal
Workcell lifecycle already destroyed the remote resource, no policy can later
request a same-filesystem resume. Workflow authors must choose one of these
before execution:

1. always produce a reconstructable handoff bundle;
2. retain/snapshot on a declared boundary candidate for bounded time; or
3. accept evidence-only review and use `retry_clean`.

This constraint must be visible in policy validation, not discovered after a
failed evaluation.

## 12. Dispatch changes

Dispatch is the reference orchestrator because it already owns dependency
scheduling, persisted state, pausing, decisions, and resume. The changes are
extensions to those mechanisms, not a runner rewrite.

### Normalize outcomes

- Add a result adapter layer for `ExecutionResult` and `EvaluationResult`.
- A generic tool step may declare an output file/stream as a versioned result.
- Legacy steps receive an inferred execution result from exit/status and
  captured artifacts. Inferred results must be marked as such and default to
  conservative effect/retry semantics.
- Add an evaluator/gate step mode or a post-step evaluation binding that reads
  any conforming evaluation result, not only Eval output.

### Resolve policy

Add a `control` block to workflow definitions. A representative shape is:

```yaml
control:
  policies:
    - when:
        result: evaluation
        verdict: fail
        minimum_severity: error
      decide:
        disposition: require_intervention
        scope: descendants
        in_flight: allow_finish
        preservation:
          required: true
          acceptable_modes: [handoff_bundle, snapshot]
        allowed_actions:
          - retry_evaluation
          - retry_step
          - retry_clean
          - amend_inputs
          - approve_override
          - abort
    - when:
        result: evaluation
        verdict: warn
      decide:
        disposition: continue
```

The built-in mapper must be deterministic, schema-validated, and ordered with
an explicit default. An external policy step may instead emit
`PolicyDecision`; Dispatch validates that its referenced inputs and policy
digest match the current state. Unknown/malformed required decisions fail
closed.

### Checkpoint and pause

- Add a boundary record to `state.json`: boundary ID, trigger result IDs,
  policy decision, state revision, scope, admission status, evidence refs,
  preservation status, allowed actions, and intervention ID.
- Stop new scheduling in scope before requesting review.
- Append the narrow `control-events.jsonl` journal.
- Generalize `human_intervention_required` to v2 while emitting the v1 event
  for configured legacy sinks during migration.
- Keep `failed` terminal. A policy that wants review must choose
  `require_intervention` before the run is terminalized.

### DAG and parallel behavior

- Maintain a scheduler admission bitmap/barrier for affected nodes.
- Compute scope from the persisted DAG revision, not from mutable workflow
  source at resume time.
- Record every in-flight step at the boundary and its chosen action.
- For `allow_finish`, record late results under the same boundary and rerun
  policy if they materially change its inputs.
- For cooperative cancellation, retain `cancel_requested` separately from
  proven `cancelled`.
- For hard termination, record effect state `unknown` unless the tool proves
  no commit.

### Explicit continuation semantics

- `resume_from_boundary`: continue with the existing producing attempts marked
  accepted; do not rerun them.
- `retry_evaluation`: create a new evaluator attempt against the same subject
  evidence unless amended references are supplied.
- `retry_step`: create a new action attempt using the declared retained or
  reconstructed workspace. Refuse if effect policy disallows it.
- `retry_clean`: provision a new Workcell/source state and create a new action
  attempt.
- `amend_inputs`: persist an immutable input revision, then execute the plan
  specified by policy.
- `approve_override`: continue past the failed gate with actor, reason,
  authorization, and original failure retained.
- `abort`: transition to terminal failed; `cancel` transitions to cancelled.

Every decision uses compare-and-swap on `boundary_id` and
`expected_state_revision`. A duplicate decision returns the prior applied
receipt. A stale or different decision is rejected without side effects.

### Idempotency

- Preserve existing stable effect keys and persisted result cache.
- Extend attempt identity so retries never overwrite prior results.
- Refuse automatic retry when effect completion is `unknown` unless the sink
  can reconcile/idempotently enforce the key.
- Treat an input amendment, clean retry, or policy override as a new attempt
  while retaining causation to the original boundary.

Likely modules: `src/core/runner.kujo`, `state.kujo`, `step.kujo`,
`hooks.kujo`, workflow loader/schema, new outcome/policy/intervention/journal
modules, CLI resume commands, and `tests/dispatch_tests.kujo` plus focused
contract fixtures.

## 13. Eval changes

Eval should remain an evaluator usable from a shell, CI job, Workcell,
Dispatch, an agent, or a third-party orchestrator.

### Required

1. Emit `evaluation-result.json` with schema
   `kujo.evaluation-result/v1` after each suite run.
2. Add that artifact to `artifact-manifest.json` and hash it when checksum mode
   is enabled.
3. Preserve existing `summary.json`, `cli-summary.json`, reports, history, and
   process exit behavior.
4. Accept an optional bounded subject-context input containing run/step/attempt
   and input evidence IDs. It must remain optional for CLI/CI independence.
5. Define deterministic default severity mapping for existing suites, while
   allowing future suite/check severity fields.

### Recommended

- Include evaluator configuration digest, bounded check annotations, and
  referenced input evidence.
- Publish JSON Schema and fixtures for pass, fail, warn, and indeterminate.
- Add a `validate-result` or shared conformance helper rather than relying on
  ad hoc JSON parsing.

### Not appropriate

- `pause`, `resume`, `retry`, Dispatch step IDs as mandatory fields, Workcell
  cleanup commands, or Leash callback URLs.
- Direct Dispatch API calls.
- A claim that `fail` always means terminal workflow failure.

Likely modules: `main.kujo`, `src/eval_core.kujo`, artifact manifest helpers,
schemas, CLI tests, manifest-tamper tests, and documentation.

## 14. Leash and human review changes

The existing Leash architecture is the right transport foundation: durable
requests, compare-and-swap decision claims, decision receipts, and a retryable
resume outbox already solve the hardest delivery and duplicate-decision
problems. Do not build a parallel “evaluation review” queue.

### Generalize the intervention contract

- Extend `human_intervention_required` to v2 with typed reason,
  `PolicyDecision`, evidence refs, preservation outcome, boundary/state
  revision, expiry, and allowed actions.
- Extend decisions with the explicit action and action-specific continuation
  plan from section 6.
- Keep actor, authentication, authorization, channel, and audit metadata.
- Require user interfaces to display effect uncertainty and preservation
  limitations before enabling retry/override.
- Make unsupported or disallowed action buttons impossible to submit, while
  still validating server-side.

### Compatibility mapping

| Leash v1 action | v2 mapping |
|---|---|
| approve | `resume_from_boundary` for a plain approval, or `approve_override` only when request reason is a failed gate and policy allows it |
| reject | `abort` |
| request_changes | `amend_inputs` with a follow-up input reference; until supplied, remain paused |
| reply | informational response; no state transition unless workflow policy explicitly consumes it |
| cancel | `cancel` |
| acknowledge | audit-only; no workflow transition |

The older Leash decision schema and the HITL schema have overlapping action
vocabularies. Phase 0 must designate one canonical v2 model and document
adapters so `defer`, `interrupt`, `kill`, and similar transport actions cannot
be mistaken for workflow continuation semantics.

### Fix the Eval integration claim

Until enforcement is implemented, `docs/eval-integration.md` must say the
policy gate is a stub and that `eval_context` is not populated. Once the shared
evaluation adapter is live, tests must prove that a failed required result
cannot be approved unless policy explicitly permits `approve_override`.

## 15. Backward compatibility plan

1. **Schemas are additive.** New result/evidence files and fields do not rename
   current artifacts.
2. **Legacy exit behavior stays.** Eval, ShipCheck, Fence, ChangeBucket, and
   other CLIs keep their documented exit codes. Adapters create normalized
   results alongside them.
3. **Dispatch defaults stay.** A workflow without `control` uses existing
   required/optional/approval semantics. Required failure remains terminal.
4. **Current approvals keep working.** Dispatch and Leash accept v1 requests
   and decisions while emitting/accepting v2 when enabled. Plain approval
   maps to `resume_from_boundary` only for the same legacy approval boundary.
5. **`needs_changes` remains readable.** Persisted v1 state can resume under
   current behavior. New state stores `paused` with reason
   `changes_requested`; CLI output may project `needs_changes` during a
   deprecation window.
6. **Workcell defaults stay ephemeral.** No retention occurs unless requested
   by existing `keep_failed` or new preservation policy. Local v1 receipt
   consumers continue to see current fields.
7. **Unknown providers remain valid.** They report unsupported preservation
   modes; policy decides whether that blocks admission or permits degraded
   evidence.
8. **No runtime migration first.** Kujo language/runtime semantics are
   unchanged. Contracts are ordinary versioned JSON and Kujo helpers.

Migration is required only for workflows that opt into reviewable failure
boundaries. They must declare policy, preservation expectations, allowed
actions, and effect semantics. Existing persisted runs must not be rewritten
in place.

## 16. Security review

### New attack surface

- Artifact URIs can become path traversal, SSRF, confused-deputy, or
  unauthorized object-store access vectors.
- Retained workspaces may contain credentials, customer data, malicious code,
  poisoned build artifacts, device nodes, symlinks, or oversized files.
- Replay/retry can repeat destructive or financially significant operations.
- Human override endpoints invite spoofed decisions, stale approvals, and
  privilege escalation.
- External policy/result documents can lie about identity, integrity,
  idempotency, or effect completion.

### Required controls

- Allowlist evidence URI schemes and resolver destinations; confine filesystem
  paths and validate archive extraction.
- Authenticate producers where trust crosses a process/host boundary; bind
  result/policy hashes to the state revision that consumed them.
- Verify manifests before sensitive continuation and record verification
  evidence.
- Encrypt retained sensitive evidence at rest, use least-privilege access,
  and audit reads and exports.
- Store secret names or secret-manager references only. Redact command output,
  prompts, provider metadata, and environment material before persistence.
  Redaction reduces exposure; it does not prove absence of secrets.
- Treat preserved generated code and artifacts as untrusted. Reviewer views
  should be read-only and non-executing. Re-execution requires a newly
  authorized sandbox/capability set.
- Require authorization, reason, and policy support for `approve_override`,
  non-idempotent retry, hard termination, destructive action, and retention
  extension.
- Enforce boundary/state revision and single-use decision IDs server-side.
- Bound evidence size, count, retention, snapshot cost, and live-pause
  duration. Garbage collection needs its own receipts and alerts.
- Treat provider handles and signed evidence URLs as credentials; store opaque
  identifiers or encrypted tokens, and rotate/expire access.
- For remote providers, distinguish request accepted, resource deleted, and
  deletion independently verified. Network loss is not deletion proof.

### Trust boundaries

An evaluator may be trusted to report its own checks but not to authorize a
workflow override. A policy engine may authorize a transition but cannot
assert that a provider snapshot exists. Workcell can attest what it collected
but cannot guarantee a remote API's business transaction semantics. Leash can
attest who decided but cannot make an unsafe retry idempotent. The orchestrator
must validate each contract against the authority of its producer.

## 17. Implementation phases

### Phase 0 — clarify contracts and correct claims

**Goal:** Agree on terminology and remove documentation that promises
unimplemented enforcement, without changing runtime behavior.

**Repositories:** Kujo, Dispatch, Eval, Workcell, Leash.

**Likely files/modules:** this architecture document; Dispatch lifecycle,
resume, idempotency, and enterprise deployment docs; Workcell lifecycle,
provider, security, and retention docs; Leash `docs/eval-integration.md` and
decision schema notes; Eval artifact-contract docs.

**New contracts:** ADRs for result versus policy versus state, preservation
versus live suspension, effect uncertainty, and replay terminology. Draft JSON
Schemas and compatibility rules.

**Tests:** Documentation examples parse; schema examples validate; a source
assertion verifies the Leash Eval policy remains labeled stub until replaced.

**Migration impact:** None.

**Dependencies:** Maintainer agreement on contract ownership and package name.

**Risk:** Low. The main risk is allowing vocabulary to expand before
implementations need it.

**Definition of done:** Canonical terms and schema drafts are approved;
incorrect Eval/Leash enforcement claims are removed; no runtime behavior is
claimed to have changed.

### Phase 1 — shared result and evidence contracts

**Goal:** Publish the smallest standalone package that any Kujo or third-party
tool can produce or consume.

**Repositories:** New `kujo-control-contracts` package or an equivalently
small ecosystem repository; Kujo only for discoverability/documentation, not
runtime semantics.

**Likely files/modules:** `schemas/evidence-ref-v1.json`,
`execution-result-v1.json`, `evaluation-result-v1.json`,
`policy-decision-v1.json`, `preservation-outcome-v1.json`, intervention v2,
re-execution descriptor; Kujo validation helpers; fixtures and compatibility
tests.

**New contracts:** All section 6 contracts, with `EvidenceRef`, execution
result, and evaluation result graduating first. Policy/intervention may remain
experimental until Dispatch integration.

**Tests:** JSON Schema positive/negative fixtures; additive-field tolerance;
major-version rejection; required hash/URI/secret constraints; round-trip
Kujo helpers; deterministic serialization where signatures/hashes require it.

**Migration impact:** None; consumers opt in.

**Dependencies:** Phase 0 decisions.

**Risk:** Medium. A shared schema can fossilize accidental product fields.
Keep orchestration commands out of evaluator results.

**Definition of done:** Two independent producer fixtures and two independent
consumer validators interoperate without importing each other's repositories.

### Phase 2 — evaluator and evidence producer adoption

**Goal:** Make Eval the reference evaluation producer and provide adapters for
existing evidence contracts.

**Repositories:** Eval first; Workcell adapter-only work; ShipCheck, Fence,
ChangeBucket, Muzzle, Agents SDK, and CaseFile as bounded follow-ups.

**Likely files/modules:** Eval `main.kujo`, `src/eval_core.kujo`, manifest and
schema fixtures; Workcell receipt/manifest adapters; tool-specific JSON
renderers; Agents SDK artifact/result adapters.

**New contracts:** `evaluation-result/v1`, `execution-result/v1`, and
`evidence-ref/v1` production.

**Tests:** Eval pass/fail/warn/indeterminate fixtures; unchanged exit codes;
artifact manifest/tamper checks; missing subject context; custom evaluator
fixture consumed by a generic validator; adapters preserve native artifacts.

**Migration impact:** New files/fields only. Existing automation remains valid.

**Dependencies:** Phase 1 stable schemas.

**Risk:** Low to medium. Severity defaults can unintentionally change perceived
policy; they must not change CLI exit behavior.

**Definition of done:** Eval and at least one non-Eval tool emit conforming
results, and CI can validate them without Dispatch.

### Phase 3 — Dispatch policy boundaries and audit journal

**Goal:** Turn normalized abnormal outcomes into explicit, enforceable workflow
decisions with deterministic DAG quiescence.

**Repositories:** Dispatch and shared contracts.

**Likely files/modules:** `src/core/runner.kujo`, `step.kujo`, `state.kujo`,
`hooks.kujo`, workflow loader/schema, new result adapter, policy resolver,
boundary, scheduler barrier, and control-journal modules; CLI output; tests.

**New contracts:** Dispatch consumption of execution/evaluation/policy results;
`control-event/v1`; boundary state; workflow `control` configuration.

**Tests:** Policy matrix; malformed/unknown contract; snapshot/journal atomicity;
scope and in-flight behavior; crash recovery between boundary stages; journal
hash/sequence verification; legacy workflows unchanged.

**Migration impact:** Opt-in workflow fields. Current failed/paused semantics
remain default.

**Dependencies:** Phases 1 and 2.

**Risk:** High. Scheduler barriers, crash windows, and concurrent results are
state-machine changes. Implement behind a feature/version gate and test with
fault injection.

**Definition of done:** A conforming custom evaluator can cause continue,
terminal failure, or durable pause solely through workflow policy; no
downstream step in the selected scope starts after the persisted barrier.

### Phase 4 — generalized intervention and explicit continuation

**Goal:** Use the existing Leash/Dispatch durable decision path for typed
review, retry, amendment, override, and abort actions.

**Repositories:** Leash, Dispatch, shared contracts; Agents SDK approval
adapter optionally.

**Likely files/modules:** Leash HITL schemas, types, store/claim validation,
chat/CLI renderers, resume worker/outbox, policy; Dispatch hook emission,
decision claim, resume CLI, retry planner, state transitions; tests and docs.

**New contracts:** intervention request/decision v2 and action-specific
continuation plans.

**Tests:** Every allowed action; unauthorized/invalid/stale decision; duplicate
callbacks; crash after claim/before acknowledgment; v1 mapping; override
authorization; input revision persistence.

**Migration impact:** Dual v1/v2 emission and acceptance during a documented
window. No automatic reinterpretation of an old approval as a failed-gate
override.

**Dependencies:** Phase 3 boundary IDs and state revisions.

**Risk:** High because an incorrect mapping can duplicate work or bypass a
gate. Server-side policy must remain authoritative over UI labels.

**Definition of done:** A paused boundary survives process restart, accepts
exactly one authorized decision, executes exactly the requested continuation
plan, and records an auditable receipt.

### Phase 5 — provider-neutral Workcell preservation

**Goal:** Produce a bounded, honest preservation outcome across local and
remote backends.

**Repositories:** Workcell, Dispatch adapters, remote backend packages where
separate.

**Likely files/modules:** local and portable coordinators, receipts, backend
capabilities/types, collect/export/finalize/cleanup code, manifest, handoff
bundle builder, retention/GC commands, provider tests.

**New contracts:** preservation request/outcome, handoff bundle manifest,
cleanup/deletion receipt, re-execution descriptor population.

**Tests:** Local filesystem retention, container destruction, export bundle,
remote snapshot supported/unsupported/degraded, failure before collection,
retention expiry, GC idempotency, sensitive path exclusion, provider crash
recovery.

**Migration impact:** Ephemeral cleanup remains default. Existing
`keep_failed` maps to requested `filesystem` on supported local backends.

**Dependencies:** Phase 1 contracts; Phase 3 can initially pause with existing
evidence while this phase adds stronger workspace guarantees.

**Risk:** High for cost and secret retention. Start with reconstructable
handoff bundles; treat live pause as optional and time-bounded.

**Definition of done:** Every backend returns a truthful preservation outcome;
required unsupported modes fail before unsafe execution or at the declared
policy boundary; expired resources are cleaned with verifiable receipts.

### Phase 6 — safe re-execution and ecosystem adoption

**Goal:** Adopt the contracts broadly and enable retries only where evidence
and effect semantics support them.

**Repositories:** Dispatch, Workcell, Eval, Agents SDK, AI SDK, RunLedger,
Watchdog, ShipCheck, Fence, ChangeBucket, Muzzle, CaseFile, and selected
third-party integrations.

**Likely files/modules:** Tool result adapters, effect declarations,
reconciliation/compensation hooks, RunLedger correlations, Watchdog reference
events, CaseFile export adapter, docs and example workflows.

**New contracts:** Stable re-execution descriptor and effect metadata;
tool-specific extensions only when required.

**Tests:** Hermetic deterministic replay with AI cassette; live-provider result
correctly marked only re-executable or inspectable; effect reconciliation;
compensation receipts; cross-version contract matrix; ecosystem golden path.

**Migration impact:** Automatic retry may become more conservative for tools
that declare effects but cannot prove idempotency. This must be opt-in until
policies are migrated.

**Dependencies:** Phases 2–5.

**Risk:** Medium to high. “Replay” marketing can exceed technical guarantees;
capability labels and conformance tests are release gates.

**Definition of done:** The same contracts work in Dispatch, CI without
Dispatch, a custom orchestrator fixture, local Workcell, and one remote
Workcell provider, with no direct Eval/Dispatch/Workcell dependency.

## 18. Test plan

### Contract conformance

- Positive and negative fixtures for every schema and version.
- Unknown additive fields are accepted; unknown major versions fail closed
  where control depends on them.
- IDs, timestamps, hashes, references, sizes, and enums are bounded.
- Raw secret fixtures are rejected by producer-side safety tests where
  feasible; redaction behavior is verified independently.
- Evidence resolvers reject traversal, unsafe schemes, unauthorized objects,
  symlink escapes, oversized bundles, and manifest mismatches.

### Required workflow scenarios

| Scenario | Expected assertion |
|---|---|
| Eval fail -> workflow fail | Policy emits `fail`; run becomes terminal failed; no intervention is created; downstream scoped nodes never start. |
| Eval fail -> workflow pause | Policy emits `require_intervention`; boundary/state/evidence are durable before request emission. |
| Eval warn -> continue | Warning remains in evidence/journal; dependencies continue according to policy. |
| Workspace preservation | Requested supported mode produces resolvable evidence and truthful limitation metadata. |
| Cleanup after successful run | No unrequested retained workspace/resource remains; cleanup receipt is recorded. |
| Cleanup after failed run | Default cleanup still occurs; requested bounded preservation follows policy; container/process semantics are asserted separately. |
| Review decision -> resume | `resume_from_boundary` continues downstream without rerunning the producing action. |
| Review decision -> abort | Run becomes failed with actor, reason, and original failure retained. |
| Retry evaluator only | New evaluation attempt consumes the same subject evidence; action attempt count is unchanged. |
| Retry execution | New action attempt is created; prior attempt/effects/evidence remain immutable; retry policy is enforced. |
| Retry clean | New workspace/source reconstruction is used; preserved dirty filesystem is not reused. |
| Amend inputs | New immutable input revision and attempt are correlated to the original boundary. |
| Approve despite failure | Requires allowed action, authorization, reason, and records override; failed verdict remains visible. |
| DAG branch failure | Descendant/branch scope blocks only computed nodes; unrelated branch behavior matches policy. |
| Parallel running steps | Already-running siblings follow `allow_finish`, cooperative cancel, or terminate exactly; late outcomes are journaled. |
| Non-idempotent side effect | Automatic retry is rejected after started/unknown effect; operator sees uncertainty. |
| Process crash before checkpoint | Recovery records/derives indeterminate effect, does not execute duplicate effect without reconciliation. |
| Remote Workcell preservation | Supported snapshot succeeds; unsupported request fails admission or returns degraded handoff as policy declares. |
| Retention expiry | Resource/artifact becomes unavailable only through GC; deletion receipt and paused-boundary policy are exercised. |
| Tampered evidence | Integrity verification fails; resume is rejected or remapped by policy. |
| Missing evidence | Required reference produces indeterminate/artifact failure; optional evidence is labeled missing without false verification. |
| Invalid intervention decision | Unknown/disallowed action or malformed plan has no state/effect change. |
| Duplicate resume | Same decision returns prior receipt; no second scheduling transition occurs. |
| Duplicate retry | Same decision creates one new attempt only; a different stale decision is rejected. |
| Timeout | Execution result is timeout; policy maps it independently to retry, pause, fail, or cancel. |
| Cancellation | Admission closes; running work state/effect uncertainty and cleanup outcome are explicit. |

### Fault injection and crash recovery

Inject termination:

1. after action completion but before result checkpoint;
2. after evaluation artifact write but before manifest write;
3. after policy resolution but before admission barrier persistence;
4. after barrier persistence but before preservation request;
5. after preservation succeeds but before outcome checkpoint;
6. after intervention decision claim but before Dispatch callback;
7. after retry attempt creation but before scheduling;
8. during remote destroy and during retention GC.

Each recovery test must assert current state, journal sequence, allowed next
actions, artifact/effect uncertainty, and lack of duplicate scheduling.

### Compatibility tests

- Existing Dispatch required/optional failures and approval/resume fixtures are
  byte/behavior compatible where contracts are not enabled.
- Persisted legacy `paused` and `needs_changes` states resume under their old
  rules.
- Eval's existing output files and exit codes remain stable for pass/fail.
- Workcell v1 local `keep_failed` still preserves files and destroys the
  container.
- Leash v1 decisions map only to safe v2 actions.
- Custom evaluator fixture controls Dispatch without importing Eval.
- Eval fixture remains useful in a plain CI process with no Dispatch, Leash,
  or Workcell available.

### Security and scale tests

- Unauthorized reviewer, stale role, expired request, forged producer,
  modified policy hash, and cross-run evidence substitution.
- Malicious artifact, path traversal, symlink/device entry, decompression bomb,
  secret-bearing logs, and hostile HTML/Markdown rendering.
- Bounded event journal and evidence counts under long/high-concurrency runs.
- Remote resource quota, retention cost limit, cleanup backlog, and provider
  outage.
- Policy denial of destructive retry and required two-person authorization
  where configured.

## 19. Acceptance criteria

### Scenario A — code change, failed evaluation, review, deterministic control

1. The agent action runs in Workcell and produces an execution result,
   `changes.patch`, receipt, manifest, and evidence refs.
2. Eval emits a conforming failed evaluation result.
3. Policy resolves to `require_intervention`; Dispatch persists a descendant
   or workflow barrier before any downstream step starts.
4. Workcell returns the predeclared preservation outcome. A local filesystem,
   provider snapshot, or reconstructable handoff bundle is identified exactly;
   no live-process claim is inferred.
5. The reviewer receives compact reason, evidence pointers, effect state,
   preservation limits, and allowed actions.
6. The reviewer chooses an explicit amend/retry/resume plan. Dispatch consumes
   it once against the expected state revision.
7. Continuation creates the correct new attempt or continues downstream; it
   does not ambiguously rerun prior effects.

“Deterministically” here describes the control transition and attempt
selection. It does not promise identical model/tool output unless the
re-execution descriptor proves deterministic replay.

### Scenario B — external request followed by failed evaluation

The execution result records the external effect ID, idempotency enforcement,
and completion state. When evaluation fails, Dispatch stops new work according
to policy but does not claim rollback. If the crash window leaves completion
unknown, automatic retry is forbidden without reconciliation or an authorized
policy. Any compensation is a separate evidenced action.

### Scenario C — Eval in CI without Dispatch

Eval writes its existing reports, summary, manifest, and the new formal
evaluation result; exits according to current CLI rules; and requires no
Dispatch, Workcell, Leash, or policy service.

### Scenario D — custom evaluator

A third-party evaluator emits `kujo.evaluation-result/v1`. Dispatch validates
it, applies the same workflow policy used for Eval, and records its evidence.
Neither repository imports the other.

### Scenario E — remote Workcell

Before execution, provider capabilities are compared with preservation policy.
If a snapshot is unavailable, policy either rejects admission or permits a
bounded handoff bundle. On failure, the provider returns a preservation
outcome and retention/cleanup handle. The system never equates a local mount
with provider-neutral preservation and never claims process continuity unless
the provider proves `live_pause` semantics.

### Global acceptance criteria

- Machine consumers answer “what happened, why did execution stop, where is
  the evidence, what effects may have occurred, and what actions are allowed”
  from bounded contracts rather than scanning raw logs.
- Evaluation verdict and workflow decision remain distinct.
- `failed` remains terminal; `paused` is resumable only with an explicit
  boundary and valid continuation decision.
- Scheduling barriers are durable before intervention is emitted.
- Evidence integrity and retention status are explicit.
- Retry cannot silently duplicate a non-idempotent or uncertain effect.
- Local and remote preservation use the same outcome vocabulary without
  promising identical capabilities.
- Existing workflows retain current behavior until they opt in.

## Final recommendation: the smallest general-purpose change set

The smallest set of changes is not a new control-plane product. It is a shared
contract package plus narrow adapters in the components that already own each
responsibility.

```text
Recommended architecture
  Result/evidence contracts -> policy mapping -> scheduler boundary
  -> provider preservation outcome -> existing intervention transport
  -> explicit revision-bound continuation

Contracts required
  EvidenceRef/v1
  ExecutionResult/v1
  EvaluationResult/v1
  PolicyDecision/v1
  PreservationOutcome/v1
  InterventionRequest/Decision v2
  ReexecutionDescriptor/v1 (after the boundary path works)

Repositories affected
  New shared contracts package
  Dispatch (consumer, policy, quiescence, journal, resume/retry)
  Eval (formal evaluation producer)
  Workcell (preservation/handoff producer)
  Leash (durable generalized intervention transport)
  Later adapters: Agents SDK, ShipCheck, Fence, ChangeBucket, Muzzle,
  RunLedger, Watchdog, CaseFile, AI SDK

Implementation order
  0. Clarify semantics and correct documentation
  1. Publish schemas and conformance fixtures
  2. Emit results/evidence from Eval and one non-Eval producer
  3. Add Dispatch policy boundaries and audit journal
  4. Extend Leash/Dispatch explicit intervention decisions
  5. Add Workcell provider-neutral preservation and retention
  6. Add safe re-execution/effect-aware retry and ecosystem adapters

Tests required
  Contract conformance and backward compatibility
  Outcome-to-policy matrices
  DAG/concurrency quiescence
  crash-window fault injection
  evidence integrity and retention
  explicit intervention/retry idempotency
  external-effect uncertainty
  local and remote Workcell preservation
  standalone Eval and custom-evaluator interoperability
```

This makes evaluation failures, policy denials, unsafe outputs, verification
failures, timeouts, and other abnormal conditions enforceable, inspectable,
and optionally resumable without coupling Eval, Dispatch, and Workcell. The
policy remains an operator choice; the contracts and lifecycle mechanics make
that choice explicit and auditable.

## Review provenance

The review used the following repository revisions:

| Repository | Revision |
|---|---|
| Kujo | `3a82da03a199` |
| Dispatch | `3472219cd80d` |
| Eval | `7ad5caef1b71` |
| Workcell | `e2915a3fbf5b` |
| Leash | `3e90f14b7abc` |
| RunLedger | `e0187ea353a9` |
| ChangeBucket | `030eea63c604` |
| ShipCheck | `111bfc83c832` |
| Fence | `07006f3194c3` on `fix/confined-output-runtime` |
| Watchdog | `52f6d61a07bf` |
| Muzzle | `e5e031dae4d5` |
| Agents SDK | `bb2202d8b54f` |
| AI SDK | `71bad1468fbc` |
| CaseFile | `f26df300df0e` |

Primary implementation surfaces inspected include Dispatch runner/state/hooks,
resume commands, workflow tests, and idempotency documentation; Eval runner,
core, artifact manifests, and CLI tests; Workcell local and portable
coordinators, backend types, receipts, manifests, provider docs, and tests;
Leash HITL schemas, policy, store, resume outbox, daemon, and integration docs;
and the corresponding result/evidence paths in the other repositories listed
above.
