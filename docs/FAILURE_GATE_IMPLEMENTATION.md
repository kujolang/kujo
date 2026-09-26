# Failure-gate implementation record

Audit date: 2026-09-25. This record supersedes the current-state claims in the
September 24 review plan, while retaining that plan as historical rationale.

## Source-verified map (before changes)

| Component / base | Result shape and failure | Evidence / integrity | Preservation / lifecycle | Intervention / retry / history |
|---|---|---|---|---|
| Eval `6a5ab09` | `evaluation-result/v1`, pass/fail; evaluator execution errors separate | report, manifest, embedded SHA-256 evidence references | artifact files; no suspended runtime | no policy; result producer only |
| Dispatch `a01a365` | tool ok/data/error; normalized evaluations consumed only after successful execution | state, bounded trace, control JSONL hash chain | completed/failed/paused; admission barriers for DAG scopes | revision-bound v2 decisions; execution failures bypass policy; key presence incorrectly permits replay; journal cursor can lag disk after crash |
| Workcell `e57ea19` | execution-result/v1; execution/infrastructure/verification/artifact classes | receipt, manifest, logs, patch, execution/reexecution documents | local failed filesystem retention; remote export then destruction; snapshot/live pause unsupported | provider-neutral preservation; success cleaned before late evaluation; no human policy |
| Leash `56c59c9` | v2 intervention transport mapped to durable queue | SQLite decision receipts / outbox | durable request/claim lifecycle | single-use decision delivery; legacy Eval policy hook remains a stub and is not used as failure policy |
| ShipCheck `111bfc8` | JSON checks, severity and gate exit | report/check IDs; no portable envelope by default | files only | no intervention or action replay |
| RunLedger `e0187ea` | receipt status in_progress/pass/partial/fail/abandoned | repository and cross-system correlation, notes/test records | persistent receipt | not a controller or complete append-only control journal |
| CaseFile `f26df30` | capture metadata and classified failure | redacted case/log/reproduction/handoff; completion marker | repository-scoped evidence bundle | review handoff, not executable authority |
| Agents SDK `bb2202d` | run status, typed errors, tool lifecycle | artifacts, trace and observed identities | caller-owned persistence | approval adapters; not durable workflow control |

Source anchors: Eval `src/control_contracts.kujo`; Dispatch `src/core/{control,
intervention,control_journal,runner,state}.kujo` and `dispatch.kujo`; Workcell
`src/evidence/{execution_result,preservation}.kujo` and execution coordinators;
Leash `daemon/src/{chatops,store,policy}.rs`; ShipCheck `src/report.kujo`;
RunLedger `src/record.kujo`; CaseFile `casefile.kujo`; Agents SDK lifecycle tests.
Existing user maintenance-agent files in Agents SDK are untouched.

## Contract audit

Eight schemas already exist: execution/evaluation/evidence/policy/preservation/
reexecution v1 and intervention request/decision v2. Object extensions are
allowed; enums and required fields are stable. ISO timestamps and hash patterns
are specified, but JSON shape does not prove provenance, effect enforcement,
preservation availability, actor authentication or legal state transitions.
Evaluation v1 has pass/fail/warn/indeterminate: evaluator errors must use
indeterminate plus an additive evaluation_status, not reinterpret fail. Schema
owners remain Kujo; policy and state transitions remain Dispatch. No provider
or evaluator internals belong in the consumer.

## Validation evidence

Baseline commands run before behavior edits in isolated worktrees. Raw logs:
`/tmp/failure-gate-baseline-{fmt,check,test}.log`,
`/tmp/failure-gate-dispatch-baseline.log`,
`/tmp/failure-gate-workcell-baseline.log`. Final results and exact commands are
recorded below after verification. Baseline failures are not new regressions.

## Implemented

- Existing schema versions retained; additive evaluator status/rule/metric,
  execution summary/workspace/idempotency-enforcement evidence, and explicit
  re-execution mode/attempt fields. Added portable `control-event/v1`.
- Dispatch consumes execution or evaluation facts, including failed handlers,
  before choosing stop/review. A crashed evaluator is indeterminate/error.
  Unknown outcomes stop; explicit legacy defaults remain supported.
- Control-enabled execution stops after one attempt; retry policy enters review.
  Replay needs known reversible effects or evidenced sink-enforced idempotency.
  Mere key submission, unknown partial effects, wrong targets, missing/expired
  preservation and unsupported adapter modes reject replay.
- Evaluator-only replay retains its inputs and evidence IDs; it never resets the
  producing action. Clean and same-workspace replay are explicit adapter modes.
  Continue/override accepts a failed boundary without replaying it.
- V2 decision transactions hold the run lock through reload/claim/continuation;
  identical duplicates are idempotent, differing payloads conflict, stale or
  expired requests and terminal-state continuations reject.
- Atomic immutable control records precede the JSONL index. Recovery checks
  sequence/hash/cursor and durable record agreement once at admission. Orphans
  or torn writes block execution and retain evidence for operator reconciliation.
- Workcell accepts pre-finalization preservation intent with a future deadline.
  Successful local workspaces can be retained; portable providers export before
  destruction. Actual outcome never promises unavailable runtime suspension.
- The Dispatch offline failure example invokes real Workcell fixture lifecycle,
  Eval and manifest verification, records a typed decision, reruns only Eval,
  proves descendants remain blocked, and creates RunLedger/CaseFile handoffs.
  Four evaluator identities share the exact same contract/policy tests.

## Remaining operational boundaries

Authentication belongs to the local caller or authenticated intervention
transport; actor metadata is not proof. Producer effect attestations need trusted
adapters. Same-workspace and clean replay require an adapter that actually checks
preservation/materializes new state; arbitrary evidence commands are never run.
Live pause/snapshot remain unsupported unless a provider supplies that capability.
No general exactly-once side-effect claim, automatic compensation, hosted control
plane or rollback is introduced. Recovery intentionally requires operator review
when a crash leaves uncertainty about execution. Active journals are capped at
8 MiB rather than implementing automatic archival/rotation.

## Deferred by ownership

Leash's legacy Eval-specific policy stub remains outside the neutral path;
Dispatch owns the failure policy and Leash already transports v2 decisions.
ShipCheck/Fence/third parties can emit the normalized envelope without importing
Dispatch; their legacy CLI outputs are not silently reinterpreted. Agents SDK,
Watchdog and Muzzle remain result/telemetry providers. No runtime syntax, closure,
async, spawn or package-manager changes were made.

## Security and performance review

Bounded, no-follow reads protect evaluator files and amended inputs; decision
files are bounded to 1 MiB. URI references remain opaque. Control-enabled retry
is conservative even when an action crashed after committing an external effect.
No network fetch, capability escalation or command comes from evidence metadata.
Logs redact known secret fields and omit raw producer outputs; producers must
still avoid sensitive payloads in labels, summaries and extension fields.

Policy operates on bounded facts; artifacts stay referenced. Each control event
adds one small atomic write and one append. Journal validation is linear once per
admission, not once per event; existing DAG scheduling complexity is unchanged.
This is a control-boundary durability cost, not a runtime hot-path change.

## Verified validation results

Kujo: `cargo fmt --check`, `cargo check`, and
`cargo test --no-fail-fast -- --test-threads=1` passed. The full serial test log
contains 83 successful suite summaries (including a nested subprocess summary),
with no failures. The actual-producer contract command passed 11/11:

```bash
KUJO_FAILURE_GATE_HANDOFF=/absolute/path/to/golden/handoff.json \
KUJO_FAILURE_GATE_DISPATCH_ROOT=/absolute/path/to/dispatch \
cargo test --test workflow_control_contracts -- --include-ignored
```

Initial full runs encountered unrelated HTTP shutdown and LSP latency failures
under shared host load; both passed isolated reruns and the subsequent full serial
run. Runtime source was unchanged by this branch. Initial baseline command startup
preceded behavior edits; the long full baseline run overlapped additive schema
work, so it is not represented as a pristine post-checkout schema benchmark.

Dispatch focused validation: 14 boundary tests, 17 effect/recovery safety tests,
and four execution/evaluator/pass tests passed. Concurrent CLI decision claims
passed. The offline example passed and its emitted Workcell, Eval, policy,
intervention and journal artifacts passed the canonical Rust schema validators.

Workcell focused validation: preservation 10/10, execution-result 7/7 and portable
coordinator 18/18 passed. Official adapter conformance passed 6/6 after isolating
fixture run IDs; previously fixed IDs collided with retained fixture ownership.
No existing fixture state was deleted. Official adapter Node tests and manifest
integrity checks passed after `npm ci --prefix adapters/official`. The release
report fixture reported 249 passed, zero failed. No live-provider claim is made.

A 100-event journal debug-build benchmark under shared host load measured
9,790 ms append and 1,031 ms recovery for 48,819 journal bytes. This establishes
measured cost of durable writes; it is not an unloaded latency guarantee or a
before/after regression comparison. Full component release-gate results are
recorded with the final handoff.

Workcell's final release-version assertion intentionally defaults to Kujo 1.2.1.
Source-runtime validation uses the new explicit
`WORKCELL_TEST_KUJO_VERSION=1.5.0` override, which checks the supplied version
without changing the release commit or Docker pins. Default release validation
must leave this override unset. This distinction prevents source-build test
results from being presented as certification of the pinned release runtime.
