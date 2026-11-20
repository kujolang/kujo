# 10. Proof of concept

Status is proposed. No code, accounts or deployment are part of the research. The first authorized implementation is an opt-in single-operator workspace with one fixed Software Factory team and two runnable adapters. Use local/custom plus Slack, with Slack simulated by signed fixtures before a separately approved sandbox trial. Add Matrix after those interfaces pass to establish two independent external networks before claiming external-provider portability.

## Scope and behavior

1. The workspace operator provisions an immutable workspace ID, fixed Factory deployment and two stable routable participant IDs for `chain.backend-developer` and `chain.code-reviewer`. The controller can run the other roles required by its sealed workflow. A roster set holds the team binding and revisions; profile definitions remain pinned catalog inputs.
2. The human uses a local authenticated interface or a Slack command/message action to stage a sealed converter mission. Plain chat is context only. Permit a short bounded plain-text brief. The local operator sees resolved workspace/actor, selected sealed pack hash, intent digest and sanitized content before allowing the original prepare/submit path.
3. During the run a developer may report a blocker or request review. For the first milestone, use a configured controller projection of a fixture's accepted typed proposal or native handoff. A worker cannot address platforms. The edge creates a scoped request for the registered reviewer from the authenticated controller observation with canonical mission correlation. The reviewer responds through the existing review step's trusted receipt when applicable. If an actual extra assistance invocation is requested, stage it for a human as a separate bounded advisory workflow with no writes. Never inject new steps into an active DAG. Keep arbitrary asynchronous agent back-and-forth and native resume outside pilot scope.
4. A human and an agent result appear in the same canonical thread in both interfaces after explicit per-destination opt-in. Simulate mirrored delivery without an external account first. Messages convey a local status summary and opaque authorized evidence references. The observer shows thread and native run IDs alongside bounded read-only evidence.
5. Introduce no channel-native approval application path. An approval request can carry a trusted operator link. Existing native and local release policies, review against the exact candidate, execution receipts, accounting and denial of deployment stay mandatory.

## Proposed transport and schema

All identifiers are opaque UTF-8 strings encoded without provider meaning in canonical IDs. Use RFC 3339 UTC timestamps. An adapter authenticates the raw provider request first and forwards normalized, typed data over a scoped local binding. The edge stamps local identity and ordering after authorization. Adapter ingress is distinct from the operator action. Proposed local commands, with a CLI or loopback Unix-socket binding, are `stage`, `show`, `authorize-start`, `thread-read`, and `thread-reply`. Use OS peer credentials or a scoped local secret supplied outside argv. Do not expose the Factory Floor to the network for control. Slack binds only one installation and allowlisted channels for the pilot.

Example of a committed event. This is the local envelope, with CloudEvents-compatible naming conventions and a future wire mapping.

```json
{
  "schema": "kujo.collaboration.event/v1",
  "id": "evt_239c",
  "type": "review.requested",
  "workspaceId": "ws_7f",
  "threadId": "thr_8d",
  "threadSequence": 4,
  "actor": {"kind": "agent", "id": "agt_backend_01"},
  "origin": {"adapter": "factory", "installationId": "ins_controller_01", "occurrenceId": "source_observation_24"},
  "correlationId": "req_start_01",
  "causationId": "evt_238b",
  "occurredAt": "2026-11-20T14:20:00Z",
  "recordedAt": "2026-11-20T14:20:01Z",
  "refs": {"missionId": "factory-demo-id", "nativeRunId": "run-demo-id", "taskId": "backend"},
  "data": {"requestId": "req_review_01", "targetAgentId": "agt_reviewer_01", "messageId": "msg_02", "replyTo": "msg_01", "text": "Verify the bounded candidate against the sealed acceptance brief."}
}
```

| Event type | Required data | Allowed origin and effect |
|---|---|---|
| `message.posted` | message ID, bounded plain text, optional canonical reply and mention IDs | Mapped workspace participant/controller. Stores content only. |
| `mission.requested` | request ID, fixed mission kind and sanitized subject/brief | Explicit request from enrolled human. Stages intent; never starts work. |
| `mission.admission.recorded` | request ID, admitted/denied/uncertain, optional submission ref and receipt digest | Local admission worker from authorized operation. Staged intent digest remains immutable. |
| `assistance.requested`, `review.requested` | request ID, target agent, message reference, scope label, native refs for controller-originated requests | Enrolled human, or controller-attested fixed role. One target and at most one hop in pilot; execution needs a separate applicable grant. |
| `review.recorded`, `mission.status.observed` | request or submission, bounded state, evidence kind/ref/digest | Trusted controller/observer projection. No guessed status from messages or chat edit state. |
| `approval.requested` | native reference or explicit no-native-decision status, request ID and trusted link key | Controller/request-only path. No `approval.granted` from adapter. |
| `message.redacted` | message ID, bounded reason code | Operator or verified source edit/deletion policy. Redact/tombstone payload; preserve minimal chain/receipts. |

Ingress request ID, adapter installation and occurrence ID are required. A direct custom caller also supplies an idempotency key for command intents. Server normalizes only known variants. `actor`, `workspaceId`, `origin`, `recordedAt`, sequence, digests and evidence refs come from server-side binding. Refuse cross-thread `replyTo`, unknown refs, duplicate incompatible content, and unknown schema major versions. Suggested hard bounds for the pilot are 64 KiB normalized event, 4 KiB UTF-8 text, 8 mentions, one active admission per workspace, one agent request hop and configurable content TTL defaulting to 30 days. Use a separate current-state table for visibility and tombstones. Retaining an immutable digest does not require retaining deleted message content.

## Persistence, delivery and restart

- Use one auditable local SQLite store or a backend qualified to equivalent semantics. Commit inbox dedupe key `(workspace, installation, occurrence)`, immutable normalized event identity, thread-local sequence, newly staged intent and chosen projection records atomically. Exact input retries return the stored acknowledgement. A conflicting reuse is rejected. Slack acks receipt without waiting for work. Treat out-of-order parents as bounded deferred linkage or a tombstoned projection. No global cross-provider order.
- Keep distinct intent key `(workspace, principal, operation, caller idempotency key)` and send key `(destination binding, event ID, revision)`. The larger store owns the queue; outbox records track pending, leased, sent, retryable, ambiguous, dead-letter or suppressed. Record the external reply ID/receipt, honor provider backoff, bound retries and stop on scope loss. A 429 is retryable subject to policy. A timeout after potential mutation is ambiguous and needs reconciliation. An expired lease may redrive a known no-effect operation. Never promise exactly-once platform deliveries.
- On start, acquire a durable intent claim conditioned on `staged`, expected immutable digest and current grant version. Prepare locally using fixed converter pack and local owner-only UTF-8 input. Persist mapping from unique intent to the returned preparation ID and digest before submit. If the daemon dies after prepare but before mapping, the orphan preparation cannot start. Reconcile it explicitly via local controller scope. After submit begins, query `factory_control_status` using the original preparation ID; a persisted submitted marker with no terminal outcome is uncertain. For the pilot keep uncertain effects quarantined until a local operator reconciles. No automatic fresh preparation after this boundary. Recheck workspace grant/version and quota before transitioning to submit.
- Adapter display names and remote accounts never drive model authorization. Only one connector owns each origin/destination link. Skip self/mirror events based on known provider bot role, persisted correlation, sender ID and copy receipts. Enqueue only explicitly allowed publications. Never export raw controller logs, prompts, credentials, arbitrary artifacts or managed provider receipts.
- Floor reads through an authorized bounded projection keyed by mission and thread. On the first single-operator host, preserve existing GET routes and response contracts. Native evidence paths remain controller-owned.

## Implementation boundaries and acceptance tests

| Check | Required result |
|---|---|
| Offline end-to-end | Existing installed controller plus deterministic representative worker/proposal fixtures. Start from custom and separately with signed Slack input. Link staged → prepared → submitted → native run, role handoff and review receipt. One same-thread reply crosses surfaces with shared canonical role identity. Test fixture and production-style traces have distinct labels. |
| Controller regression | Role schema/proposal restrictions, source digests, Workcell controls, independent review/acceptance, local release accounting and deployment false still hold. Live behavior needs a separately approved bounded mission and its own receipt. |
| Auth and tenant isolation | Reject forged/stale Slack signatures, missing binding, unauthorized IDs, mismatched channel/installation, revoked grant, cross-workspace object references, fake worker actor and review/approval from a bot. Keep errors externally safe. |
| Replay and races | Same event twice produces one occurrence, one thread event and at most one staged intent. Conflicting event or intent ID returns conflict. Two callers authorize concurrently and only one crosses submit. An edit/delete after staging cannot change sealed brief/routing. |
| Failure and recovery | Kill/restart around store commit, claim, preparation mapping, submit and outbound acknowledgement. No false success and no automatic redispatch after uncertainty. Outbound retry after 429, poison event/dead-letter, lost response and unresolved parent all have bounded outcomes. |
| Safety boundaries | Agent asks for arbitrary file, external URL, arbitrary tool, more wakes, spending, bypass or release. Such requests produce no effects. Verify redacted summaries and TTL/tombstone behavior across local and mirrored stores, recheck destination permission before send. |
| Independent live proof | With separate approval and test workspace, demonstrate provider webhook/thread mappings, local plus Slack independent submissions/replies, revocation and audit. Capture provider correlation, submitted counts, artifact hashes and controller verdict. Repeat the same flow in Matrix before claiming portability across external providers. Fixture success alone does not satisfy this row. |

Use a synthetic team with no private files, no attachments, no paid model calls in regression, no arbitrary repository selection, merge, deployment, external resume or hosted multi-tenancy. Suggested pilot success target is zero duplicated native submissions in the finite fixture fault matrix, one handled agent-originated review request, one explicit human admission per successful mission, and parity of IDs and authority outcomes in both paths.
