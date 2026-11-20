# 11. Registry and factory integration

**Decision for review.** Put the shared protocol, connector runtime, local storage and integration tests in a separately versioned collaboration repository, tentatively `kujo-collaboration`. Software Factory supplies the first execution adapter. Other factories and standalone agents can use the same connector installation and workspace without installing Software Factory. Each host keeps ownership of its execution, verification and approvals. This updates the pilot ownership recommendation in the original audit.

## What the catalog supports today

- `kujo-agents/agent-registry.json` lists definition IDs, names, package and relative profile path. The per-agent `schemas/agent-manifest.schema.json` enumerates allowed top-level fields and rejects unknown properties. Its capability, permission, handoff and adapter fields describe a role's supported behavior and ceilings. `scripts/generate_runtime_adapters.py#manifests` resolves the registry and renders credential-free Hermes and Paperclip descriptors. Those descriptors are a pattern for distribution. They do not provision workspace connections. [Canonical registry and adapter source](https://github.com/kujolang/kujo-agents/tree/a7e9d0236d6e64b5db30273b32f4ad8c124f5173)
- `agents.kujolang.ai/scripts/sync-agent-content.py#sync_set` derives static agent pages from `AGENT.md`, set ordering and namespaced public slugs. The current front matter has no collaboration connector fields. `templates/page-home.html` links the Kujo agent installer and owned-project setup. Existing site tests cover separate set ownership and cross-set slug collisions. This is an appropriate discovery and setup surface, with authorization performed by a local or self-hosted operator interface. [Catalog site at audited HEAD](https://github.com/kujolang/agents.kujolang.ai/tree/97b1affa6fc133f561d55ac6a753d2193e09e1ef)

## Ownership and connection flow

| Owner | Proposed responsibility |
|---|---|
| Shared collaboration repository | Workspace bindings, transport contracts and adapters, thread store, receipt/retry, visibility and scoped request staging. Publish a versioned host adapter interface and capability negotiation. Support local/embedded operation first and optional external stores. |
| Kujo Agents registry | Keep stable profile and team definitions. Publish a credential-free, independently versioned integration index alongside the registry. Link profiles and teams to supported host capabilities, current qualification and connector setup references. |
| agents.kujolang.ai | Project registry integration metadata onto existing agent and team pages. Show compatible choices, minimum supported host version, supported actions and maturity. Provide setup links or copyable install/configuration instructions. |
| Factory or standalone agent host | Register its available actions and stable deployment references with the local operator. Translate typed intents into its existing admission path. Produce scoped work, verification, review and observation receipts. |
| Workspace operator | Choose an agent or team, install its host and selected connector, complete scoped platform authorization, map users and channels, and grant individual actions. Review and revoke bindings locally. |

An agent page should offer choices such as **Connect Slack** or **Connect Microsoft Teams** only when the registry publishes the matching host and connector combination. A standalone user chooses a supported agent host. A team user chooses the relevant factory. The setup page hands off public, validated package and profile references to local documentation or the operator UI. The local setup verifies package/version, probes host capabilities, authorizes the provider, displays proposed workspace and channel scopes, and creates a binding only after operator consent. Provider tokens, user IDs, private channel IDs, grants and installation status stay local to that deployment. One bot may represent several named agents. The canonical identities remain stable across transports, while each connection receives its own provider account mapping.

## Proposed public compatibility index

Publish a versioned sidecar such as `collaboration-integrations.json` to avoid breaking strict manifest readers or existing factory pins. Join on canonical profile/team IDs, never slugs. Each row declares host kind, connector, supported protocol majors, tested host and connector versions or ranges, supported actions, qualification status, setup URL, pinned artifact reference and scope documentation. A row may cover a team when its roles share the same host requirements. Default missing or incompatible rows to unavailable, and distinguish `experimental` from `qualified`. Generate the compatibility facts from host and connector releases and qualification receipts. Existing registry entries and manifest permissions remain valid. Treat the index as discovery metadata and validate it again against local installation capabilities before displaying authorization options.

## Host adoption

| Host | First integration | Authority retained locally |
|---|---|---|
| Software Factory | Sealed mission request, role review and Floor status through local control MCP and controller receipts. | Fixed pack, Dispatch admission and native decisions, Workcell enforcement, verification and release. |
| WebOps Factory | Bounded site QA review request and read-only evidence projection over the installed WebOps host. Start with an enrolled fixture report and public-fixture site scope. | Approved site scope, Lens/SiteProbe evidence and controller verification. No connector-provided crawl targets or deployment rights. |
| VideoOps Factory | Request clarification, reviewer feedback and bounded render status. | Media, provider spend, render and review decisions. Upload and publication require distinct native authority. |
| Publishing House Factory | Editorial review and production handoff with scoped evidence references. The proposal interprets “production house factory” as the audited Publishing House Factory. | Editorial stage, source rights, publication review, VersionSeal and Presswire boundaries where used. |
| Product Launch Factory | Connect one launch conversation to parent and child receipts, with an explicit routing grant per child. | Parent/child orchestration, package sealing and approval. Parent thread membership grants no automatic child authority. |
| Standalone Agents SDK or owned agent project | Register a named deployment, receive scoped message context and publish bounded review replies through a host adapter. | Agent tool, session, model and budget policies. Initial fixture offers no effectful commands. |
| Future host | Run the same host conformance suite and declare only implemented operations. | Its own admission and evidence rules. |

Keep the shared port small. A host advertises supported operations, resolves authorized scope, stages a typed intent, lets its controller admit it, and projects bounded observations. No common native workflow state machine is required. A collaboration grant can narrow host permissions. The host controller determines whether it can perform the operation.

## Acceptance before publishing setup options

1. Use the same connector build, envelope and installation bindings for the Software Factory pilot, a WebOps evidence/review fixture and a standalone no-effect agent. Verify that neither independent host loads Software Factory. Confirm that changing hosts requires no connector code changes.
2. Generate site options for two independent profile families and a team. Prove stable identity across namespaced slugs, reject bad versions and references, hide unsupported combinations, and keep pages buildable with older catalog pins.
3. Install a connector independently of agent installation. A blank workspace remains unconnected. Credential flow stays out of the public website and registry. Revocation disables delivery and future requests without deleting the agent definition.
4. Verify custom and Slack transport behavior with fixtures, then qualify Slack in an authorized test workspace. Verify Matrix as a second external provider. Add Teams after its authentication, installation scope and proactive delivery contract are qualified. Add Board and Paperclip through their own connector surfaces and declare exact capabilities.
