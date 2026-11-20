# 05. Architecture options and decision

## Alternatives

Sizes are relative across the first two interfaces and include authorization and durability work. They are planning estimates, not measured effort.

| Dimension | A. Extend current components directly | B. Dedicated service/package | C. External collaboration backend | D. Small Kujo contract, interchangeable backends |
|---|---|---|---|---|
| New code and maintenance | Smallest first hook; adapters, delivery and ACL logic can spread across Factory and Dispatch | Largest. Service APIs, storage, migration, policy, instrumentation and deployment | Small when external task/chat covers requirements; durable translation and operational monitoring still required | Small to medium. Local edge, contracts, one store binding and two transports; extract only when reused |
| Reuse | Maximum execution reuse, weak reuse of transport systems | Reuses native execution but creates a new coordination service | Board/Paperclip provide tasks, actors, messages, sync or wake. Reuses Factory only as execution handoff | Reuses Factory, Dispatch, Workcell and catalogs; qualified Board/Paperclip/Chat SDK can supply storage/transport capabilities |
| Security and reliability | Few deployment boundaries; control and observer code can accumulate public ingress risks | Clear isolation and independent rollout; larger attack surface and more failure modes | Vendor policy helps at platform boundary; mixed principals, split-brain tasks, external availability and data retention are harder to control | Staged ingress, durable queues and single authority; adapter confinement and external effects still require testing |
| Portability and extension | Strong local portability; adding factories risks copy/paste or Dispatch surface bloat | Strong if protocol succeeds; higher adoption friction | Lowest for mandatory Board/Paperclip; optional adapter preserves portability | Strong. Same identities and native execution behind custom/local, Slack, Teams or task-oriented adapters |
| Self-hosting and multi-user | Local-first, eventual access/scoping retrofit if public endpoints land too early | Independent self-hosting, viable multi-user model after full investment | Paperclip self-hosting available, Board local engine available for dev/offline, managed Board control differs | Single-operator proof; versioned workspace scoping supports later tenant isolation and deployment choice |
| Performance and costs | Few hops, local I/O; coupling to engine can stall runs during remote failures | Separate scaling; network/db/ops costs from day one | Quotas and network tail latency; SaaS or self-hosted operations and vendor pricing exposure | SQLite transaction/worker overhead in local deployment; async delivery protects workflow latency; hosted cost deferred |
| Compatibility and factory utility | Risk of reinterpreting Dispatch events and forcing one factory's flow into another | Generic if semantics settle; premature policy migration | External states have unfamiliar semantics for existing controllers | Existing records remain native. Small projection/adapter port can be independently adopted by all factories |

**Recommend D, delivered in Software Factory first.** Build a bounded interoperability edge and prove the same two roles, request semantics and receipts through a local/custom interface and Slack. Establish independence with live dual-interface use during a later authorized pilot and require Matrix as the second external network before claiming portability across external providers. Treat Board as an adapter and a candidate storage implementation. Benchmark it against the minimal local implementation after reviewing the required durability and authorization semantics. Integrate Paperclip through its existing Kujo plugin when demand is demonstrated. Keep Teams as a separately qualified transport. No new repository is needed for the first two interfaces. Extract a package within the ecosystem after an independent factory or plugin consumes the contract. Service/repository extraction needs an independent deployment, ownership or scaling requirement.

## State and authority

| Record | Owner | Cross-boundary treatment |
|---|---|---|
| Profile, role instructions, static capabilities | Kujo Agents catalog and pinned Factory role contracts | Publish stable definition reference, profile revision and bounded public description |
| Workspace, human principals, agent slot/deployment IDs, bindings and admission grants | Kujo collaboration edge/operator configuration | External identifiers map to a principal and versioned installation. Platform profile/role strings supply display only |
| External platform message, actor, installation and permission state | Respective provider | Store scoped provider references and admission-time evidence; revalidate current grant for commands |
| Admitted communication, thread links, peer requests, delivery attempts | Collaboration edge, or one qualified backend per workspace | Authoritative canonical IDs; origin policy, destination copies and redaction classification. External backlog and edits cannot rewrite a staged intent |
| External backlog and claims | Board/Paperclip/issue tracker per configured workspace | Allowlisted intent only. Do not propagate external claim/close into workflow state |
| Mission admission, work orders, artifact verification and local release | Software Factory controller | Edge stores submission identity and allowlisted status/evidence summaries |
| Dependency graph, native run revisions, attempts, interventions and decisions | Dispatch | Correlate native run/step/revision plus decision or event ID. Signed/request link refers back to native authority |
| Effective execution controls and receipts | Workcell/backend | Provide summary and digest via existing controller, never forward raw output by default |
| Accountability, verification, telemetry | RunLedger, Casefile, Eval, Lens, Watchdog and Factory bindings | Immutable reference and scope, retention independent of collaboration content |

## Protocol fit and versioning

| Capability | Existing shape | Treatment |
|---|---|---|
| Identity / resolution | Factory role IDs, SDK target IDs, City imported catalog/instance identities, upstream Paperclip company and agents, Board actors | Introduce workspace and stable agent deployment IDs. Map each `provider/installation/subject` with version and revocation; never infer identity from handles. Team registration is a versioned set of agent bindings. |
| Mission/task references | Factory IDs and task IDs, Dispatch workflow/run/step/attempt/revision, Paperclip issue IDs | Explicit typed references, including provider namespace. Link after admission and retain correlation rather than copying native state. |
| Message/thread/reply/mention/context | SDK session messages, Board and Paperclip transcripts, City event replay | Independent collaboration schema for canonical message ID, thread, reply, text and structured mention IDs; use explicit mapping or tombstone for provider edits/deletes. Context ingestion must be bounded and marked untrusted. |
| Delegation / review / approval | Factory `kujo.handoff/v1`, Agents SDK `HandoffRequest`, Dispatch native decisions, plugin observation receipts | Keep native versions. Translate collaboration request payloads into a reviewed local intent. Any future approval response preserves exact controller/effect/decision/revision and actor bindings. |
| Correlation and replay | Dispatch control journal, hooks, Factory JSONL, City event IDs, Relay event exports | Introduce event ID plus source scope, causation/correlation, delivery occurrence, command idempotency and audit pointers. Thread-local order is separate from native revision. Managed backends require conformance for recovery and export. |
| Acknowledgement / dispatch | Dispatch hook receipts, Factory prepared/submitted/terminal receipts, Board sync receipts | Ack means durable receipt. Delivery means provider-specific write. Admission/execution needs separate authenticated status and immutable binding. Retain uncertainty for ambiguous writes. |

Independent versioning is required for the envelope, connector interface and capability negotiation. Existing native payloads should pass through read-only projections by reference and digest. Add optional fields to existing native contracts only under their compatibility rules and contract tests, such as a future caller correlation key. Do not add chat types to Dispatch's scheduler or broaden Factory event schemas to fit provider state. See [the proposed pilot contract](10-proof-of-concept-spec.md).
