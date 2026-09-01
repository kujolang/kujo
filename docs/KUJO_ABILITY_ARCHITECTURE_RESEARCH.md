# Kujo Ability Architecture Research

Status: architecture recommendation; no production implementation

Decision date: 2026-09-01

Decision: **BUILD WITH CHANGES**

## 1. Executive Summary

Kujo should adopt **Ability** as a shared ecosystem concept, but it should not extract the current CMS module wholesale, add language syntax, or make every tool and workflow an Ability.

The useful abstraction is narrow: an Ability is a versioned, portable semantic operation contract. It says **what an operation means**, what structured input it accepts, what structured output it returns, what durable effects it may cause, and what retry semantics callers may rely on. It does not say how to invoke it over MCP, where its handler runs, which model provider to use, who is currently authorized, or whether a particular request needs approval. Those concerns belong to bindings, adapters, and policy.

The CMS implementation proves the vertical slice: discovery, schemas, permission checks, execution, enablement, audit, plugin contribution, and MCP projection all work together in `cms/backend/routes/abilities.kujo`. It is not yet the reusable core. Its registry is coupled to CMS permissions, database-backed feature state, REST routes, plugin HTTP runtimes, and CMS-specific handlers. Its schemas and safety annotations are largely descriptive rather than uniformly enforced. In particular, plugin non-GET operations are marked `requires_confirmation`, but `execute_ability` forwards plugin calls before the confirmation checks used by the built-in SEO handlers. This must be corrected before generalizing the system.

The shared architecture should therefore be:

```text
AbilityDefinition (portable meaning)
       |
       +--> AbilityBinding (handler + execution substrate)
       +--> PolicyBinding  (authorization + approval + obligations)
       +--> ExposureBinding (REST/MCP/WebMCP/agent/CLI visibility)
                         |
                         v
                    Invocation
                         |
              validate -> authorize -> approve
                         |
                      execute
                         |
                validate -> receipt
```

Start as a strict, versioned JSON contract plus a small Kujo library. Keep it outside the language core. Prove it with CMS and one independent consumer before creating a permanent standalone package repository. Preserve the current CMS REST routes and names through adapters during migration.

## 2. Current State

This research inspected the following repositories at the listed commits. Paths cited below are relative to the `kujo-repos/` workspace root.

| Repository | Commit | Ability-relevant responsibility |
|---|---:|---|
| `kujo` | `ed51720892d8` | Language/runtime primitives, native host-effect capability gates, workflow-pack CLI extension mechanism |
| `cms` | `6ef9c7972221` | Existing Ability registry, execution, policy integration, REST discovery, MCP projection, plugin contributions, WebMCP |
| `cms-example` | `01c22cabfba4` | Studio UI and server-side proxy consuming CMS Ability, connector, MCP, and WebMCP APIs |
| `cms-contact-form` | `8f7e2aca428d` | Independent plugin that declares read and mutating CMS Abilities |
| `ssg` | `e7502322b75a` | Experimental static, same-origin, read-only WebMCP generation |
| `mcp` | `2ab8111f2c51` | MCP tool registry, schemas, dispatch, protocol exposure, safety tiers |
| `agents-sdk` | `d3904d348754` | Tool contracts, handlers, permissions, risk, approval hooks, external provider adapters |
| `ai-sdk` | `be9617a32344` | Provider-driver boundary for model and embedding transports |
| `spec` | `1211f37a0931` | Task goals, scope, acceptance criteria, risks, and human approval points |
| `eval` | `955713f487c0` | Deterministic outcome and evidence verification |
| `fence` | `fc7a000ba837` | Static architecture dependency rules and import-boundary enforcement |
| `leash` | `3e90f14b7abc` | Runtime policy, risk classification, approvals, and release gates |
| `runledger` | `eebd17bd89d0` | Durable run receipts, usage, cost, artifacts, and human verdicts |
| `casefile` | `6e8a6ded379a` | Failure evidence bundles and handoff artifacts |
| `workcell` | `0f8a806f5831` | Isolated execution substrate, resource/network/filesystem controls, verification, receipts |
| `dispatch` | `662417c264bd` | Durable multi-step workflow orchestration, tools, approvals, retries, and reports |
| `watchdog` | `060311289fb1` | Passive request/tool telemetry, redaction, tracing, replay-safe identities |
| `source` | `006c6e7db612` | Capability-security model and durable workflow/action vocabulary |

The `agents-sdk` and `source` working trees contained unrelated pre-existing changes. This research treated them as read-only and did not modify them.

## 3. Existing CMS Ability Analysis

### 3.1 Registry shape

`cms/backend/routes/abilities.kujo:17` defines `ability_registry()`. Each built-in descriptor currently includes:

- `name`, such as `cms/site-info`, `content/list`, or `seo/update-entry`;
- presentation fields `label`, `description`, and `category`;
- a CMS permission string;
- JSON-like `input_schema` and `output_schema`;
- annotations for `readonly`, `destructive`, `idempotent`, and `requires_confirmation`.

The six built-ins are implemented in the same route module. `execute_ability` at line 386 selects handlers by string comparison. Read operations call local CMS functions. The two SEO write operations validate `confirmed`, normalize changes, and update the CMS database.

`resolved_ability_registry(ctx)` at line 71 adds mutable enablement state from CMS settings and merges active plugin descriptors from `extension_ai_registry`. It changes plugin dotted names into the slash form used by the CMS API, derives read-only status from the HTTP method, assigns a default CMS permission, and attaches an `external_execution` record.

This is a sound product-local registry. It is not a portable definition layer because descriptor resolution mutates the contract with product state, authorization, source provenance, and transport bindings.

### 3.2 Discovery and execution surfaces

`register_ability_routes` at `cms/backend/routes/abilities.kujo:434` exposes:

- `GET /v1/abilities/categories` — authenticated category discovery;
- `GET /v1/abilities` — authenticated registry discovery with category filtering;
- `GET /v1/abilities/:namespace/:ability` — permission-scoped inspection;
- `POST /v1/abilities/:namespace/:ability/run` — permission-scoped invocation;
- `PATCH /v1/abilities/:namespace/:ability` — administrative enable/disable.

The run route rejects disabled Abilities, calls `guard_request` with the descriptor's permission, accepts an object under `input`, executes the handler, and emits an `ability.execute` audit event. `cms/backend/runtime/main.kujo:276-279` publishes those routes in the service description.

`mcp_tool_descriptors()` at `cms/backend/routes/abilities.kujo:174` and `resolved_mcp_tool_descriptors(ctx)` at line 191 project enabled Abilities into MCP-shaped tool records. The current projection changes `namespace/operation` into `namespace__operation`, passes through the input schema and annotations, and points execution back to the CMS REST route. `GET /v1/ai/mcp/tools` returns this catalog as “mcp-ready”; CMS itself is not pretending that a descriptor is an MCP transport.

### 3.3 Plugin execution

Plugin manifests may carry up to 50 bounded, secret-free Ability descriptors. Validation in `cms/backend/routes/extensions.kujo:190-218` limits descriptor count, key count, serialized size, sensitive keys, and name length, but it does not validate a full Ability schema. `extension_ai_registry` at line 277 returns contributions from active plugins.

Plugin execution in `cms/backend/routes/abilities.kujo:299-383` has valuable hardening that should remain in the CMS binding:

- only `GET`, `POST`, `PUT`, `PATCH`, and `DELETE` are allowed;
- paths must be absolute, cannot traverse, and require declared path parameters;
- outbound URLs pass the plugin hook URL policy;
- private-network access is opt-in and allowlistable;
- runtime bearer secrets come from normalized server environment variables;
- timeout is bounded;
- response bodies are capped at 1 MiB;
- non-2xx and malformed responses become stable CMS errors.

The independent `cms-contact-form/kujo-plugin.json:26-40` demonstrates that this extension path is real, not hypothetical. It contributes `contact.submissions.list` over `GET` and `contact.submissions.moderate` over `PATCH` with separate permissions.

### 3.4 Studio consumption model

`cms-example/app/api/cms/route.ts:100` fetches Abilities, connectors, MCP descriptors, WebMCP, and extension AI metadata together through a same-origin application route. The application authenticates the Studio user and keeps the CMS bearer on the server. Lines 137-149 proxy administrative Ability/connector state changes and connector health checks.

`cms-example/app/cms/AiWorkspace.tsx:34-70` treats the registry as an operations catalog. It counts write Abilities, renders source and permission metadata, toggles enablement, distinguishes REST/CLI/MCP/WebMCP access surfaces, and presents connectors separately. This is the correct conceptual split: a UI can manage several projections without claiming that they are the same protocol.

### 3.5 Generalization assessment

Inside CMS, “Ability” means an authenticated, schema-described, permission-scoped CMS operation that can be discovered, enabled/disabled, invoked through REST/CLI, projected as an MCP-ready tool, and audited. It was built to give agents a safer semantic surface than raw CRUD endpoints while letting core and plugin operations share discovery and governance.

Its current assumptions are product-specific:

| Concern | Current CMS assumption | Shared architecture treatment |
|---|---|---|
| HTTP | Execution is a CMS REST route; plugins are HTTP method/path bindings | Keep in CMS/API and plugin bindings |
| CMS content | Core handlers and categories know entries, SEO, integrations, and publication state | Keep in CMS definitions and handlers, not shared library |
| Users/roles | Bearer/session identity resolves CMS role permissions and admin capabilities | Keep in CMS policy binding |
| Authentication/authorization | `guard_request` evaluates a descriptor's CMS permission | Keep application-owned; shared gateway calls a policy interface |
| Persistence | Enablement uses CMS settings; handlers and audit use the CMS database | Keep in CMS registry/binding/observation layers |
| Confirmation | Core writes inspect `input.confirmed`; plugin writes are only annotated today | Replace with generic enforcement, then request-bound approval tokens |
| Auditing | Successful calls write CMS `ability.execute` events | Keep CMS audit; add a normalized optional receipt envelope |
| Tenancy | Tenant/workspace context is inherited from CMS request guards and data access | Make invocation context explicit; application remains source of truth |

Generalize the stable ID/version, description, bounded input/output schemas, semantic effects, idempotency, definition validation, compatibility rules, and projection fixtures. Keep CMS permissions, roles, feature-state rows, routes, plugin HTTP transport, handler dispatch, database work, categories, UI labels, rate limits, tenant resolution, and audit storage in CMS.

Extracting `backend/routes/abilities.kujo` would create unnecessary coupling because the reusable vocabulary and the product runtime are interleaved. Creating a second unrelated Ability concept would be worse: CMS and ecosystem tools would acquire competing semantic identities. The safe path is a compatibility converter into one shared definition, with CMS continuing to own binding, policy, exposure, and observation. Changing the representation now is easier than later because there are only six core descriptors, one demonstrated plugin producer, and one first-party UI consumer; preserving route/name aliases avoids forcing that timing onto clients.

## 4. Existing WebMCP Analysis

### 4.1 CMS WebMCP

CMS WebMCP is deliberately not an authenticated projection of all CMS Abilities. `cms/backend/routes/webmcp.kujo` defines four public tools: site information, search, content listing, and single-content retrieval. `webmcp_entry_allowed` filters unpublished or excluded records. Public records are bounded and sanitized. `public_guard` uses the public request path rather than a CMS bearer.

The manifest identifies `kujo-cms-webmcp/v1`, declares automatic same-origin registration, and reports security properties including `published_only`, `read_only`, `same_origin`, and `untrusted_content`. The generated browser runtime requires `document.modelContext`, rejects cross-origin configuration, fetches only the CMS public routes, and marks returned content untrusted.

`cms/docs/webmcp.md` makes the architectural boundary explicit: private, administrative, mutating, credential-bearing, and unbounded operations stay behind the authenticated Abilities API. That boundary should survive any shared Ability architecture.

### 4.2 SSG WebMCP

`ssg/docs/architecture/adr-0001-experimental-webmcp-v1.md` records an accepted but experimental design. It is opt-in, static-only, read-only, and same-origin. It provides the same four universal discovery operations through a generated public index. Cross-origin calls, mutations, authentication, server capabilities, and site-specific tools are explicitly deferred.

`ssg/docs/webmcp.md` documents privacy properties: draft and excluded pages do not enter the index, unknown front matter is omitted, arguments are strict and bounded, and output is untrusted. `ssg/scripts/test-webmcp-contract.sh` verifies disabled-build byte identity, privacy canaries, duplicate prevention, schema/runtime shape, and deterministic full versus sharded output. `ssg/build.kujo` owns index generation and runtime injection.

The SSG cannot execute general Abilities because it has no trusted server execution plane. It can eventually expose a small class of compile-time or pure static read Abilities, but static site tools should not be forced through the shared abstraction in the first release.

## 5. Concept Map

| Existing concept | Actual meaning | Relationship to Ability |
|---|---|---|
| Kujo native capability | Permission to use a host effect such as filesystem write, process execution, network, database, clock, or random | Lower-level execution authority required by a handler; never an Ability alias |
| Source `Capability` | Opaque, bounded, revocable grant of authority for allowed actions and scope | Security credential; using this name for semantic operations would be dangerous |
| CMS/admin capability | UI-facing permission derived from roles | Product authorization view; policy input, not operation definition |
| Agent/MCP tool | Model/protocol-callable name, schema, metadata, and invocation path | A projection or adapter surface for an Ability, or a protocol-native operation |
| Handler | Executable code implementing an operation | Bound implementation of an Ability |
| Adapter | Translation between a semantic contract and a protocol/provider/runtime | Projects or binds an Ability without changing its meaning |
| Provider | External model, API, transport, or service implementation | A handler dependency or binding choice |
| Workflow | Ordered/DAG composition with retries, routing, approvals, state, and outputs | May invoke Abilities; is not an Ability |
| Skill | Instructions and knowledge that help an agent act | May teach use of Abilities; is not executable authority |
| Effect | Declared semantic consequence on a resource or external system | Required, context-free part of an Ability definition |
| Policy | Contextual decision about authorization, approval, limits, and obligations | Evaluated around an invocation; not embedded as product roles in the definition |
| Spec | Goal, scope, acceptance criteria, risks, and approval points for a task | Can require outcomes produced by Ability calls |
| Eval | Evidence-backed verification of outcomes | Validates results; does not define or authorize operations |
| Receipt | Durable observation of one execution | Produced after an invocation; not the operation definition |

The strongest collision is **Capability**. `kujo/src/interpreter/capabilities.rs:2-50` already uses it for native host-effect permission gates. `source/src/capability.js` and `source/docs/source/08-identity-capability-model.md` use it for a bounded authority credential with allowed/denied actions, scope, expiration, and revocation. The new semantic abstraction must not be named Capability.

“Action” is also overloaded in Source workflow and authorization vocabularies (`source/src/auth.js`, `source/docs/source/27-workflows-actions.md`). “Tool” is already protocol- and agent-facing. “Operation” is accurate but too generic to establish a distinct contract. **Ability** has the least harmful collision, already has a deployed CMS meaning, and communicates user- or system-visible semantics rather than authority.

### Ability semantics

An **Ability** is a stable, versioned declaration of one bounded semantic operation that a system knows how to perform.

An Ability has five defining properties:

1. **Stable meaning.** Its identifier and major version preserve the same conceptual action across transports and implementations.
2. **Bounded contract.** Input and output are described by closed, size-bounded schemas.
3. **Declared effects.** The definition states the classes of effects the operation may cause, independent of who calls it.
4. **Explicit retry semantics.** Callers know whether repeat invocation is intrinsically safe, requires an idempotency key, or must not be retried automatically.
5. **Implementation independence.** The definition does not contain a URL, function pointer, provider key, bearer, tenant, or deployment location.

An Ability is not necessarily exposed. A system may define and bind an Ability without making it visible over REST, MCP, WebMCP, a CLI, or an agent registry. Conversely, a protocol may expose useful tools that are not Abilities, such as MCP server administration or SSG's generated generic site-index queries.

An Ability is not a Workflow. A Workflow has control flow, multiple steps, branching, retries, handoffs, approvals, budgets, and durable state. `dispatch/src/workflows/workflow.kujo` and `dispatch/src/workflows/loader.kujo` model exactly those concerns. A Workflow may call one or many Abilities; a convenience adapter may expose a whole Workflow as a tool, but that does not erase the semantic distinction.

## 6. Recommended Architecture

The proposed architecture separates durable meaning from every concern that changes by product, deployment, caller, or protocol:

```text
              portable, immutable
              AbilityDefinition
                     |
       +-------------+-------------+
       |             |             |
 AbilityBinding  PolicyBinding  ExposureBinding
       |             |             |
 handler/runtime  auth/approval  REST/MCP/WebMCP/CLI
       +-------------+-------------+
                     |
            invocation gateway
                     |
          validated AbilityReceipt
```

This is stronger than a single large Ability object. Definitions can travel without credentials or deployment assumptions; products can bind different implementations; policy can vary by tenant and caller; protocols can change without renaming domain semantics; and observation systems can consume one normalized result envelope.

### Naming recommendation

Keep the public name **Ability**.

Use a transport-neutral, lowercase dotted identifier:

```text
<organization>.<product-or-domain>.<resource>.<verb>
```

Examples:

```text
kujo.cms.site.inspect
kujo.cms.content.list
kujo.cms.seo.update-entry
kujo.contact-form.submissions.moderate
```

Identifiers should match:

```text
^[a-z][a-z0-9-]*(\.[a-z][a-z0-9-]*){2,}$
```

The identifier is semantic identity, not a route. Adapters derive constrained names:

- REST may use `/v1/abilities/{id}/invocations` or retain product-native aliases;
- MCP may map dots and hyphens to its accepted tool-name form;
- CLI may map segments to nested commands;
- WebMCP may use a short site-local name while retaining the canonical ID in metadata.

Do not bake `mcp`, `webmcp`, `http`, `plugin`, a provider name, a deployment environment, or a version into the identifier. Put compatibility versioning in the definition's `version` field. During CMS migration, keep `cms/site-info`, `content/list`, and the other slash names as explicit aliases so clients and routes do not break.

## 7. Recommended Ability Contract

The first durable contract should contain only:

```json
{
  "schema": "kujo.ability/v1",
  "id": "kujo.cms.seo.update-entry",
  "version": "1.0.0",
  "title": "Update entry SEO",
  "description": "Update the bounded SEO fields of one CMS entry.",
  "input_schema": {
    "type": "object",
    "properties": {
      "entry_id": {"type": "integer", "minimum": 1},
      "changes": {"type": "object"}
    },
    "required": ["entry_id", "changes"],
    "additionalProperties": false
  },
  "output_schema": {
    "type": "object",
    "properties": {
      "entry_id": {"type": "integer"},
      "revision": {"type": "integer"}
    },
    "required": ["entry_id", "revision"],
    "additionalProperties": false
  },
  "effects": [
    {"kind": "write", "resource": "kujo.cms.entry.seo"}
  ],
  "idempotency": {"mode": "keyed"}
}
```

Required fields and constraints:

| Field | Requirement |
|---|---|
| `schema` | Exact supported schema ID; reject unknown major versions |
| `id` | Stable dotted semantic ID; globally unique within the publisher's namespace |
| `version` | Semantic version of the operation contract |
| `description` | Bounded, plain description of the outcome, suitable for humans and agents |
| `input_schema` | Supported JSON Schema subset; object root; bounded depth/size; `additionalProperties: false` by default |
| `output_schema` | Supported JSON Schema subset; bounded output; secrets excluded by contract |
| `effects` | Non-empty list of `{kind, resource}` pairs |
| `idempotency.mode` | `intrinsic`, `keyed`, or `none` |

`title` is optional presentation metadata. The initial effect vocabulary should remain deliberately small: `read`, `write`, `delete`, and `external`. `resource` is a namespaced semantic target. A definition may declare multiple effects. Policy maps those facts into contextual risk; the definition must not hard-code “high risk” or “requires admin.” New effect kinds should be added only after real implementations require them.

Keep the following out of the portable definition:

- handler names, module paths, function references, URLs, HTTP methods, and timeouts;
- provider/model/runtime selection;
- permissions, roles, users, tenants, tokens, and secrets;
- enablement, rollout percentage, and environment state;
- approval mode, confirmation booleans, rate limits, and risk scores;
- MCP/WebMCP/REST/CLI names and exposure settings;
- audit destinations, telemetry sinks, receipt storage, and UI categories;
- retries, backoff, compensators, transactions, and workflow control flow;
- Workcell images, commands, filesystem/network rules, and native Kujo capabilities.

Those values change by deployment or invocation. Putting them in the semantic definition would make portability fictitious.

### Adjacent binding, policy, exposure, and receipt contracts

The minimal definition needs four adjacent contracts, each separately versioned.

### 9.1 Ability binding

An `AbilityBinding` connects an Ability ID/version range to an implementation. It may contain a handler reference, implementation version, timeout, required native Kujo capabilities, Workcell profile, provider/adapter configuration, and deployment health. It is local and trusted configuration, not portable catalog data.

CMS core handlers remain local function bindings. Plugin Abilities remain HTTP bindings with the URL policy and server-only bearer. Agents SDK handlers remain `handler` functions registered through `create_tool_impl`. Workcell may be selected as an isolated binding for untrusted or resource-sensitive work.

### 9.2 Policy binding

A `PolicyBinding` maps Ability effects and resources plus invocation context to a decision:

```text
allow | deny | require_approval
```

It may also return obligations such as a rate limit, redaction profile, receipt requirement, maximum result size, or allowed resource scope. CMS permissions and tenancy, Agents SDK approval hooks, and Leash policies plug in here. Policy is fail-closed when unavailable.

### 9.3 Exposure binding

An `ExposureBinding` explicitly allowlists an Ability for a channel and supplies channel-local naming and presentation metadata. It must never be inferred solely from the existence of a definition or handler.

Examples are `cms-rest`, `mcp`, `agent-tool`, `cli`, and `webmcp-public`. Each binding may further restrict inputs and outputs but must not broaden effects or authorization.

### 9.4 Invocation receipt

Every executed Ability should produce a normalized receipt envelope, whether or not it is persisted centrally:

- receipt schema and invocation ID;
- canonical Ability ID and version;
- implementation/binding version;
- caller, tenant/workspace, and channel identifiers in appropriately redacted form;
- policy decision and approval reference;
- idempotency key digest or intrinsic replay marker;
- start/end timestamps and status;
- declared effects and observed effect summary;
- output digest and artifact/evidence references;
- stable error code without secret-bearing raw payloads.

RunLedger can store or link these receipts at an integration boundary. Casefile can capture a failed invocation's evidence bundle. Watchdog can observe trace-safe telemetry. None of those products should become a mandatory in-process dependency of the Ability library.

### Invocation pipeline and invariants

The invocation pipeline should be fixed and testable:

1. Parse a size-bounded envelope and resolve an exact Ability ID/version.
2. Resolve an enabled binding and explicit exposure for the calling channel.
3. Authenticate the caller and derive tenant/workspace context.
4. Perform an inexpensive structural input check, then evaluate policy against the Ability's declared effects and requested resource scope.
5. Obtain an approval token when policy requires it. Approval is bound to caller, tenant, Ability version, normalized input digest, effects, and expiry.
6. Fully validate and normalize input against the supported schema subset.
7. Reserve the idempotency key atomically where `mode` is `keyed`; reject unsafe automatic retries where `mode` is `none`.
8. Execute the bound handler under its timeout, resource, native-capability, network, and filesystem limits.
9. Validate and bound the output. A mismatched output is an implementation failure, not a successful call.
10. Redact observations, finalize idempotency state, emit the receipt, and return the channel-specific response.

Security invariants:

- A caller cannot set `readonly`, `effects`, `permission`, `risk`, tenant, or handler identity in invocation input.
- Policy evaluates the server-resolved definition and binding, never client-supplied annotations.
- Approval is not a bare `confirmed: true`; it is a one-time or narrowly replayable authorization bound to the exact request.
- Output schemas are enforced, not just advertised.
- A handler cannot expand its declared effects by choosing a more privileged provider or Workcell profile.
- Cross-tenant identifiers are rejected before handler execution and again at the storage/service boundary.
- Disabled or unexposed Abilities are undiscoverable where non-disclosure is required and uncallable everywhere.
- Logs, receipts, and protocol errors are bounded and redact credentials and sensitive payload fields.

Kujo native capabilities remain the final host-effect gate. `kujo/docs/NATIVE_API_SECURITY_POSTURE.md:8` correctly states that Kujo is not a sandbox; `--allow-all` is trusted mode. An Ability declaration cannot turn trusted code execution into isolation. Workcell or an external sandbox is required for adversarial handlers.

### MCP and agent-tool projection model

MCP and agent tools should be adapters over Ability definitions and bindings, not synonyms.

`mcp/src/tools/registry.kujo` already owns protocol-facing tool metadata and handler dispatch. MCP scaffolds also use safety tiers such as `read_only`, `safe_command`, `write_scaffold`, `review_required`, and `blocked`. These are exposure/policy classifications, not portable semantic identity.

`agents-sdk/src/agents/tools/registry.kujo` already models tools with IDs, names, input/output schemas, permissions, risk, timeouts, handlers, and metadata. `create_tool_impl` binds execution directly. `agents-sdk/src/agents/security/approval.kujo` adds approval and guardrail policy. `agents-sdk/src/agents/integrations/adapters.kujo:343-375` maps external tools into registry entries and preserves provider/protocol metadata.

The shared adapter should therefore:

- derive a protocol-safe tool name from the canonical Ability ID;
- copy the bounded description and schemas;
- translate declared effects into conservative protocol annotations or local risk defaults;
- invoke the common Ability gateway rather than a second handler;
- preserve canonical ID/version and receipt correlation in metadata;
- refuse projection when schemas, effects, policy, or exposure are unsupported.

Tools may still exist without Abilities: MCP discovery/control tools, SDK-internal coordination tools, and one-off developer helpers are legitimate. Abilities may also exist without tools: internal service operations or operations exposed only through a product REST API.

### WebMCP projection model

WebMCP exposure must be an explicit, stricter projection.

An Ability is eligible for public WebMCP only when all of the following hold:

- every declared effect is `read`;
- the resource is explicitly public and published;
- the exposure binding is an allowlist entry, not an automatic export;
- input and output schemas fit the browser adapter's supported subset and limits;
- output contains no credentials, private metadata, drafts, tenant internals, or unbounded HTML;
- the handler is same-origin or compiled into the static site artifact;
- results are marked untrusted;
- deterministic privacy and disabled-build tests pass.

CMS's four existing public tools should remain protocol-native built-ins initially. They already have stronger hand-written public-record shaping than the current Ability registry. They can later become projections only if the shared contract can represent their privacy constraints without weakening them.

SSG's four generic site-index tools should also remain built-ins. A static site has no place to run a write Ability or protect a bearer. A future SSG custom Ability feature, if justified, should accept only pure static handlers evaluated at build time or read-only index queries compiled into the generated runtime.

Never export all `readonly` Abilities automatically. “Read-only” does not imply public, non-sensitive, same-origin, bounded, or safe for prompt consumption.

## 8. Repository Ownership

### 13.1 What Kujo core should own

Kujo core should own only reusable mechanisms needed by several ecosystem packages:

- existing JSON parsing and schema-validation primitives;
- native host-effect capability gates;
- stable error and serialization behavior;
- optionally, a generic CLI extension delegation point if one is already consistent with workflow-pack architecture.

It should not own a global Ability registry, provider routing, approval policy, catalog distribution, remote execution, or observability. `kujo/AGENTS.md:16` and `kujo/docs/AI_RUNTIME.md:6` explicitly place registries, provider policy, agents, eval, and observability in ecosystem packages.

### 13.2 Where the shared contract should live

Do not immediately create a permanent repository based on one product implementation. First place the draft schema, conformance fixtures, and normalization library in a bounded experimental package area consumed by CMS and one second prototype. The experiment must not require CMS to depend on Agents SDK or vice versa.

Once two independent consumers pass the same conformance suite, graduate the code into a small standalone ecosystem repository/package named `ability`. That is the only ownership shape that avoids all three bad dependencies:

- putting it in `cms` makes non-CMS consumers depend on product policy and storage;
- putting it in `agents-sdk` makes service and static consumers depend on an agent framework;
- putting it in `kujo` core violates the core's mechanism-before-policy boundary and creates pressure for native syntax.

The graduated package should contain schemas, validation/normalization, compatibility rules, projections, and fixtures—not a hosted registry, execution service, policy engine, or UI.

### 13.3 CLI ownership

A future `kujo ability` command should be a thin extension that validates manifests, lists local definitions/bindings, prints projections, and invokes a configured local gateway. It should use the existing workflow-pack/extension direction rather than hard-code product registries into the core binary. Current workflow packs run scripts with `--allow-all` (`kujo/docs/WORKFLOW_PACKS.md:338,425`), so they are not yet a suitable security boundary for untrusted Ability handlers.

## 9. Language/API Design

The recommended declaration mechanism is a hybrid: a strict manifest normalized to one JSON model, plus ordinary Kujo functions for validation, registration, binding, invocation, and projection. A conceptual API should stay this small:

```kujo
definition := ability_load("abilities/publish-article.json")
validated := ability_validate(definition)

registry := ability_registry()
registry = ability_bind(registry, validated["value"], {
	"handler": publish_article,
	"implementation_version": "1.0.0"
})
```

SDKs in other languages can register the same normalized definition with a native handler. Generated definitions are acceptable when they produce the same canonical form and pass the same conformance fixtures. SDK registration is a binding mechanism, not a competing definition format.

### Why native Kujo syntax is not justified

No inspected implementation requires a new AST node or language keyword. Ability definitions are data. Handlers are ordinary Kujo functions or external bindings. Policy, adapters, and registries change faster than language syntax.

A native construct such as `ability foo(...) -> ...` would create long-term costs:

- parser, formatter, type checker, interpreter, VM/JIT, LSP, docs, and compatibility work;
- pressure to encode policy and protocol metadata in language syntax;
- difficulty representing non-Kujo handlers;
- premature commitment before effect and version semantics are proven;
- conceptual collision with native capabilities and ordinary functions.

Use a **hybrid** instead: strict versioned JSON/TOML/YAML manifests normalized to one JSON data model, plus Kujo library functions for validation, registration, binding, invocation, and projection. Reconsider syntax only after multiple independent packages show repetitive boilerplate that cannot be solved by library APIs or generated code.

### Adjacent Kujo system boundaries

### AI SDK

`ai-sdk/docs/ARCHITECTURE_DATA_FLOW.md` and `ai-sdk/src/provider_driver.kujo` separate provider-native transport/auth/decoding from core validation and request flow. Ability handlers may call AI SDK, but provider selection belongs to the binding or application. Model features are provider capabilities, not Abilities.

### Dispatch

Dispatch owns multi-step execution, retries, approvals, routing, state, and reports. An Ability adapter can appear as a Dispatch tool step. A Dispatch workflow may be exposed as a tool for convenience, but its workflow identity and receipt remain distinct. `parallel_safe` and idempotency checks stay under Dispatch control; Ability idempotency metadata supplies evidence for that decision.

### Spec and Eval

`spec/docs/ARCHITECTURE.md` defines the task contract: goal, scope, acceptance criteria, risks, and human approval points. An Ability contract defines one executable semantic operation. Eval (`eval/docs/ARCHITECTURE.md`) checks outcome evidence. A Spec may permit certain Ability IDs/effects; an Eval may consume Ability receipts and artifacts. Neither should be folded into the Ability definition.

### Leash

Leash is the natural policy control-plane integration. Its low/medium/high/dangerous classifications, approval flows, and fail-closed behavior belong in policy bindings. Ability effects and resources are facts Leash can classify; portable definitions must not import Leash-specific risk labels.

### Workcell

`workcell/src/domain/definition.kujo` describes runtime/backend, workspace, commands, secrets, resources, network, filesystem, artifacts, verification, and receipts. That is an execution substrate. A binding may choose a Workcell profile; the Ability definition stays identical if execution moves from a local function to a Workcell.

### RunLedger, Casefile, and Watchdog

RunLedger records attempts, usage, cost, artifacts, notes, and verdicts; it is not an automatic judge. Casefile captures durable failure evidence. Watchdog observes telemetry while tool executors remain independent. All three should integrate through the receipt/trace boundary to avoid mandatory coupling and duplicate sources of truth.

### Fence

Fence can enforce static ownership boundaries after packages exist: definitions must not import handlers or policy; protocol adapters may depend on definitions; product bindings may depend inward on both; definitions may not depend outward on CMS, MCP, Agents SDK, or Leash. Fence does not authorize runtime invocation.

### Source

Source has a mature distinction between authorization actions, bounded capabilities, durable workflows, and operator-registered handlers. Preserve that distinction. An Ability ID may map to a Source authorization action in a policy binding, but a Source Capability must remain the credential that grants that action.

## 10. WebMCP Integration

WebMCP should consume only explicitly exposed, public-safe Ability definitions through a WebMCP adapter. The adapter derives the browser tool descriptor, narrows schemas where necessary, calls a same-origin public binding, marks results untrusted, and preserves the canonical Ability ID in metadata. It refuses any Ability with non-read effects, private resources, unsupported schema features, credentials, or missing public-output shaping.

The CMS and SSG four-tool catalogs should remain built-in during the first implementation because their privacy constraints are stronger and more specific than the proposed generic contract. Custom read-only Abilities may coexist beside those tools after conformance. Authenticated and mutating Abilities stay out of browser WebMCP until the protocol and application provide an independently reviewed identity, CSRF, approval, replay, and origin model. WebMCP remains an exposure mechanism, never the owner of domain semantics.

## 11. MCP Integration

The same Ability definition should be reusable through MCP, but only through an explicit MCP exposure binding. The adapter maps canonical IDs to collision-checked protocol names, copies supported input/output schemas, translates effects into conservative annotations or safety tiers, and invokes the common Ability gateway. It must preserve canonical ID/version and receipt correlation as metadata and reject unsupported contracts.

MCP-native tools remain valid when they do not represent stable domain semantics. Ability must not absorb server administration, protocol negotiation, catalog refresh, or other MCP control operations. Likewise, an Ability need not be exposed through MCP.

## 12. CMS Migration

### Phase 0 — close current contract gaps

Before extraction:

- enforce `requires_confirmation` generically before both core and plugin execution, or replace it with approval tokens;
- validate plugin descriptors against an explicit supported schema rather than “small secret-free object with a name”;
- validate all invocation input and handler output against the advertised schemas;
- make idempotency behavior real and test concurrent/replayed mutation calls;
- document the difference between descriptor annotations and enforced controls.

These fixes belong in CMS even if no shared Ability system is built.

### Phase 1 — introduce the normalized definition behind compatibility adapters

- Add `kujo.ability/v1` schema and conformance fixtures.
- Write explicit converters from the six CMS core descriptors and plugin manifest descriptors.
- Keep the existing REST route shapes, slash names, response fields, CLI commands, and Studio APIs.
- Store canonical ID/version as additive fields and keep old names as aliases.
- Run legacy and normalized descriptor generation in shadow mode and compare golden output.

### Phase 2 — split runtime concerns

- Move CMS-specific handlers into bindings while leaving route ownership in CMS.
- Move permissions, tenancy, enablement, confirmation/approval, rate limiting, and audit into CMS policy/exposure bindings.
- Keep plugin HTTP execution and its URL/secret/timeout/size controls in the CMS plugin binding.
- Introduce normalized receipts without requiring RunLedger.

### Phase 3 — replace projections one at a time

- Project MCP descriptors through the shared adapter and compare them byte/semantically with the current `resolved_mcp_tool_descriptors` output.
- Add an Agents SDK adapter that calls the CMS Ability gateway and retains canonical receipt IDs.
- Leave public CMS and SSG WebMCP unchanged until dedicated public-exposure tests demonstrate equivalence.

### Phase 4 — graduate shared ownership

After CMS and a genuinely independent consumer pass the same suite, graduate schemas/library/fixtures into the standalone `ability` package. Deprecate legacy descriptor fields only in a new CMS API major version. Do not remove aliases until telemetry and release notes show no supported client relies on them.

### Compatibility strategy

Compatibility is defined at three independent layers:

1. **Schema compatibility.** Consumers reject unknown schema major versions and unknown fields unless the schema explicitly permits extensions. Minor schema evolution is additive.
2. **Ability compatibility.** The Ability's semantic `version` changes major when required input, output meaning, effect set, or idempotency guarantees break. Adding optional input or output fields may be minor only when old consumers remain correct.
3. **Projection compatibility.** REST routes, MCP names, CLI forms, and WebMCP names have their own compatibility promises. Changing a projection does not silently rename the Ability.

CMS should publish a compatibility map:

| Legacy CMS name | Canonical ID |
|---|---|
| `cms/site-info` | `kujo.cms.site.inspect` |
| `content/list` | `kujo.cms.content.list` |
| `seo/audit-summary` | `kujo.cms.seo.audit` |
| `seo/update-entry` | `kujo.cms.seo.update-entry` |
| `seo/bulk-update` | `kujo.cms.seo.bulk-update` |
| `ai/integration-status` | `kujo.cms.integrations.inspect` |

Plugin names should receive publisher-aware canonical IDs at install time, with the original manifest name retained as a stable alias. The mapping must reject collisions instead of silently rewriting two names to one protocol name.

### Testing and evaluation strategy

### Contract conformance

- valid/invalid fixtures for every required field and bound;
- unknown schema version and unknown-field rejection;
- ID normalization and collision tests;
- semantic-version compatibility matrices;
- supported JSON Schema subset and depth/size limits;
- effect and idempotency vocabulary tests;
- canonical serialization and digest fixtures shared across repositories.

### Security and policy

- caller attempts to forge effects, read-only status, tenant, permission, approval, or handler;
- cross-tenant resource IDs and confused-deputy cases;
- disabled, unbound, unexposed, and version-mismatched calls;
- approval replay with changed input, caller, tenant, Ability version, or expiry;
- concurrent keyed idempotency and crash recovery;
- handler output schema mismatch, oversized output, secret redaction, and stable errors;
- plugin URL traversal/private-network/redirect/bearer/timeout/response-limit cases;
- mutation execution without confirmation/approval for every binding type.

### Differential migration tests

- legacy CMS descriptors versus normalized projections;
- existing CMS REST/CLI/Studio snapshots unchanged;
- current MCP tool names, schemas, enablement, and execution URLs preserved;
- plugin `GET` and `PATCH` paths exercised end-to-end;
- audit and receipt correlation verified;
- rollback to the legacy registry demonstrated.

### WebMCP

- preserve SSG disabled-build byte identity;
- preserve draft/private/excluded canaries;
- same-origin enforcement and no credentialed requests;
- only explicit read-effect allowlist entries exported;
- deterministic generated bytes across full and sharded builds;
- public output shaping equal to or stricter than current hand-written tools.

### Cross-product evaluation

- one Ability invoked through CMS REST, MCP, and Agents SDK produces the same normalized semantic output and correlated receipt;
- Dispatch invokes the same Ability in a tool step without taking ownership of authorization;
- Leash denial/approval is fail-closed and channel-independent;
- Workcell-bound implementation cannot exceed its declared native/network/filesystem limits;
- RunLedger, Casefile, and Watchdog integrations remain optional and receive redacted stable envelopes;
- Fence rules prevent definition-to-product dependency inversion.

## 13. Security Model

The trust boundary is the invocation gateway, not the manifest and not the protocol adapter. A definition is signed or trusted catalog data, but its effect claims do not authorize execution. The gateway resolves the server-owned definition and binding, authenticates the caller, fixes tenant/workspace context, asks the product or Leash policy layer for a decision, obtains request-bound approval where needed, validates input, reserves idempotency state, executes under binding limits, validates output, and emits a redacted receipt.

Protection ownership is explicit:

| Protection | Owner |
|---|---|
| Semantic effect and idempotency declaration | Ability definition |
| Input/output contract enforcement | Ability gateway/library |
| Identity, session, role, permission, and tenant checks | Application policy binding |
| Risk classification and human approval | Application or Leash policy |
| Native filesystem/network/process/database gates | Kujo runtime |
| Isolation, resource limits, filesystem/network sandbox | Workcell or external runtime |
| Same-origin, public-only shaping, untrusted output, browser registration | WebMCP adapter/application |
| CSRF defenses for cookie-bearing application routes | Application HTTP boundary; WebMCP must not bypass it |
| Provider credentials and endpoint policy | Handler/provider binding |
| Audit, telemetry, run evidence, and failure bundle | Application, receipt adapter, RunLedger, Watchdog, Casefile |

Public Abilities are an exposure class, not a weaker authorization path. Authenticated Abilities propagate a server-verified principal. Privileged and destructive calls require narrowly bound approvals and idempotency. Cross-origin browser execution is rejected unless a future protocol and application security model explicitly supports it. Prompt-injected content is always untrusted data; it cannot alter the resolved Ability, effects, caller, policy, approval, exposure, or handler.

The current CMS plugin confirmation mismatch is a concrete warning: `resolved_ability_registry` marks every non-GET plugin operation as requiring confirmation (`cms/backend/routes/abilities.kujo:80-94`), but `execute_ability` forwards plugin operations at lines 388-391 before the built-in confirmation checks at lines 402 and 416. Documentation in `cms/README.md:187-189` and `cms/docs/extensions.md:55` describes a common confirmation boundary. Generalization must not proceed until enforcement and documentation agree.

## 14. Architecture Stress Tests

The proposed split remains coherent across the required scenarios:

| Scenario | Semantic definition | Implementation owner | Authorization and effect/risk | Possible exposure | Evidence/audit |
|---|---|---|---|---|---|
| Public website: `request_quote` | `kujo.service.quote.request`, bounded contact/service input, receipt output, effects `write` on a quote request and `external` if notification is sent, `keyed` idempotency | Site application's trusted server; CRM/email are provider bindings | Anonymous anti-abuse policy plus consent; elevated fraud/spam risk, no blanket public write permission | Same-origin HTTP; WebMCP only with explicit public mutation design and request-bound confirmation; MCP/agent for authenticated staff | Request ID, consent/policy decision, provider status, redacted contact digest; application audit |
| CMS: `publish_article` | `kujo.cms.article.publish`, entry/version input, published revision output, `write`, `keyed` | CMS content handler and database transaction | Authenticated editor, tenant scope, publish permission, approval when editorial policy requires; high content effect | CMS REST/CLI, MCP, agent tool; not public WebMCP | CMS audit, prior/new revision IDs, approval reference, Ability receipt |
| Source control: `approve_change` | `kujo.source.change.approve`, change/revision input, decision output, `write`, normally `keyed` | Source service handler | Source Capability credential grants the canonical authorization action; protected-branch policy may require review; privileged effect | Source API/CLI, MCP, agent tool | Source event/audit plus receipt containing immutable change revision and policy decision |
| Workcell: `verify_patch` | `kujo.workcell.patch.verify`, patch/workspace reference and verification plan input, evidence output, predominantly `read` with controlled execution effects, `keyed` | Workcell backend selected by binding (Docker, Podman, or future provider) | Project policy chooses allowed commands/network/filesystem; medium execution risk despite no product mutation | SDK/CLI, Dispatch tool, MCP/agent tool; not public WebMCP | Workcell receipt, logs, artifacts, Eval result, optional Casefile on failure and RunLedger link |
| Commerce: `place_order` | `kujo.commerce.order.place`, cart/pricing/fulfillment input, order output, `write` and often `external`, `keyed` | Commerce service; payment and fulfillment are provider bindings | Authenticated buyer or scoped guest session; inventory, price, fraud, financial approval/SCA policy; very high irreversible-risk surface | First-party API/SDK; tightly controlled agent tool; no generic public WebMCP export | Idempotency record, price snapshot, payment/fulfillment references, audit and receipt; secrets excluded |
| Read-only knowledge: `find_documentation` | `kujo.docs.content.find`, bounded query/filter input, result references, `read`, `intrinsic` | SSG index, CMS delivery API, or search adapter | Public allowlist and published-only filter; low direct effect but prompt-injection/privacy risk | WebMCP, MCP, HTTP, CLI, SDK, agent tool | Query/result counts and content/version digest; usually telemetry rather than durable business audit |
| External integration: `create_issue` | `kujo.issues.issue.create`, normalized issue input/output, `external`, `keyed` | GitHub/GitLab/Source/Gitea adapter selected by binding | Authenticated principal mapped to provider scope; repository/project policy and approval for sensitive destinations; medium external write risk | API/CLI, MCP, agent tool, Dispatch step | Canonical receipt plus provider, remote issue ID/URL, idempotency state, sanitized provider response |

The provider-independent `create_issue` case validates the key boundary: provider-specific fields may appear in an explicitly namespaced binding extension or narrower product profile, but the base semantic contract must not become a union of every provider API. If two providers cannot uphold the same outcome and effects, they implement different Ability IDs rather than pretending compatibility.

### Operational failure scenarios

The design is not proven until it survives:

- 10,000 definitions with paginated discovery and no prompt-sized catalog dump;
- concurrent registry updates while invocations resolve exact immutable versions;
- duplicate publisher IDs and protocol-name collisions;
- policy latency or outage with authorization failing closed;
- approval replay after caller, tenant, input digest, definition, or binding changes;
- idempotency reservation followed by timeout, process death, or an ambiguous provider response;
- a schema-valid handler result paired with an undeclared external effect;
- malicious plugin schemas, sensitive keys, URL/path tricks, redirects, or misleading descriptions;
- MCP/WebMCP transport retries and large model-generated validation inputs;
- tenant migration/deletion during queued execution;
- unavailable telemetry, audit, or receipt sinks under their documented failure policy;
- prompt injection and unsafe links in WebMCP content;
- inconsistent SSG catalogs across shards;
- mixed old/new CMS nodes during rolling migration.

Definitions remain immutable by `(id, version)`, registry/policy snapshots are recorded per invocation, `idempotency.mode = none` forbids automatic transport retry, and ambiguous keyed calls are queried before retry.

## 15. What Kujo Ability Should NOT Become

Kujo Ability should not become:

- another generic function registry—ordinary functions and handlers already exist;
- another MCP or WebMCP wrapper—protocols are optional projections;
- another workflow engine—Dispatch owns composition, state, routing, retries, approvals, and reports;
- another provider framework—AI SDK and product adapters own provider translation;
- a replacement for Spec, Eval, Fence, Leash, authentication, Workcell, RunLedger, Casefile, Watchdog, or Source capabilities;
- an enormous object containing semantics, roles, credentials, deployment, policy, provider, protocol, UI, telemetry, and workflow state;
- an agent-only abstraction or a CMS-specific abstraction;
- an abstraction that requires WebMCP, MCP, HTTP, or any other transport;
- a hosted global registry, marketplace, remote executor, identity service, or policy language;
- a large industry ontology or semantic package registry in v1;
- automatic export of every endpoint, function, Tool, or read-only operation;
- native Kujo syntax, compiler/VM semantics, or a new type system before library evidence demands it;
- a reason to remove or rename current CMS routes and tool names during initial migration.

Reusable semantic packs may eventually group independently versioned Ability definitions, examples, and conformance profiles for domains such as content or service businesses. V1 should only keep IDs and schemas package-friendly; it should not design distribution, marketplaces, dependency resolution, or industry taxonomies.

## 16. Risks

| Rank | Risk | Why it is difficult to reverse | Mitigation |
|---|---|---|---|
| **VERY HIGH** | Security metadata remains advisory | Clients will trust schemas/effects/confirmation that handlers do not enforce | Close CMS gaps first; central gateway enforcement; adversarial conformance tests |
| **VERY HIGH** | Wrong semantic boundary | Collapsing Ability, Tool, Workflow, Capability, and Policy spreads incompatible contracts across repositories | Lock the narrow definition and dependency rules before implementation |
| **VERY HIGH** | Native syntax too early | Syntax commits parser, compiler, VM, tooling, and compatibility indefinitely | Use manifest plus library; require multi-package evidence before reconsideration |
| **HIGH** | Core/global registry ownership | Creates circular dependencies and central policy/provider pressure | Experimental package, then standalone `ability` only after two consumers |
| **HIGH** | Effect/idempotency semantics are vague | Autonomous retries and approvals become unsafe across protocols | Small closed vocabulary, normative behavior, failure and concurrency tests |
| **HIGH** | CMS compatibility break | Existing REST, CLI, Studio, plugins, and MCP descriptors already consume current names | Aliases, additive fields, shadow comparison, staged rollback |
| **HIGH** | Automatic public/tool exposure | Private reads or destructive writes become agent-callable through metadata inference | Explicit channel allowlists and stricter adapter eligibility |
| **MEDIUM** | Provider leakage into definitions | Portable IDs become unusable across GitHub/GitLab/Source or runtime backends | Keep provider config in bindings; split semantics when outcomes truly differ |
| **MEDIUM** | Catalog and prompt sprawl | Discoverability and model selection degrade as every endpoint becomes an Ability | Curated exposures, pagination, search, task-scoped catalogs |
| **MEDIUM** | Version layers are conflated | Schema, semantic, implementation, and protocol changes trigger needless breakage | Version each layer independently and test compatibility matrices |
| **MEDIUM** | Undeclared handler effects | Metadata cannot constrain trusted code by itself | Native capability limits, Workcell isolation, observation, security review |
| **LOW** | Name `Ability` is imperfect | Some developers may initially confuse it with capability or skill | Publish the concept map; never use `Capability` as an alias |

## 17. Recommended Implementation Sequence

| Phase | Scope | Exit condition | Estimated focused effort |
|---|---|---|---:|
| 0 — CMS safety baseline | Generic confirmation/approval, input/output enforcement, plugin descriptor schema, idempotency tests | CMS metadata and execution agree for core and plugin Abilities | 3-5 days |
| 1 — contract proof | Draft `kujo.ability/v1`, normalizer, fixtures, canonical digests, CMS converters; no new repo or syntax | Legacy/new golden comparison passes | 5-8 days |
| 2 — CMS adoption behind compatibility | Definition/binding/policy/exposure split, canonical IDs plus aliases, normalized receipts, existing APIs unchanged | REST/CLI/Studio/plugin rollback rehearsal passes | 7-12 days |
| 3 — protocol adapters | MCP and Agents SDK projections through the common gateway | Cross-channel semantic output and receipt correlation pass | 6-10 days |
| 4 — independent consumer | Use the same contract outside CMS, preferably Source- or Workcell-adjacent without product coupling | Two consumers pass one conformance suite | 5-10 days |
| 5 — graduation decision | Create standalone `ability` package only if gates pass; harden optional CLI validation/inspection | Dependency and release review approves permanent ownership | 4-7 days |
| 6 — WebMCP experiment | Explicit public-read Ability projection alongside existing built-ins | CMS/SSG privacy, origin, untrusted-output, and deterministic tests are equal or stronger | 5-8 days |

Security review and rolling-migration rehearsal are separate from these architecture-sized estimates. If the Phase 4 gate fails, retain Ability as a CMS-local feature and stop; do not manufacture a second consumer to justify a repository.

## 18. Decisions We Should Make Now

- Ability means a versioned semantic operation, not authority, transport, handler, provider, workflow, or skill.
- The public name is **Ability**; **Capability** is reserved for authority/runtime permission meanings.
- `Ability != Workflow` and `Ability != Tool` are foundational rules.
- Definitions, bindings, policy, exposure, and receipts are separate contracts.
- The durable definition requires ID, semantic version, bounded input/output schemas, effects, and idempotency.
- Effects use a small initial vocabulary; risk, permission, approval, and authentication remain policy.
- Protocol export is explicit and allowlisted; read-only never automatically means public.
- CMS aliases and routes remain compatible through the first migration.
- No native Kujo syntax and no core/global registry.
- CMS's existing enforcement gaps are Phase 0, not deferred cleanup.
- A standalone `ability` repository is conditional on two independent conforming consumers.

## 19. Decisions We Should Deliberately Defer

- semantic package distribution, registries, marketplaces, and industry taxonomies;
- native language syntax or compile-time Ability types;
- a permanent CLI surface beyond experimental validate/inspect commands;
- richer effect kinds such as financial, deployment, privileged, or communication-specific categories until policy integrations supply evidence;
- preconditions, postconditions, compensators, rollback declarations, and transaction semantics;
- negotiation among multiple handlers/providers for one Ability;
- remote discovery/federation, signing, and publisher trust infrastructure;
- protocol-specific support for future WebMCP or MCP revisions;
- standardized hosted receipt storage and retention;
- automatic generation from OpenAPI, MCP catalogs, functions, or workflows;
- public authenticated/mutating WebMCP exposure;
- whether semantic packs are manifests, packages, or documentation profiles.

Deferral is a compatibility feature: each item can be added beside the small contract without changing what an Ability means.

## 20. Final Recommendation

**Recommended architecture:** A portable `AbilityDefinition` with separate implementation, policy, exposure, and receipt contracts, enforced through a common invocation gateway.

**Recommended repository ownership:** Experimental shared package first; graduate to a standalone `ability` ecosystem repository only after CMS and one independent consumer pass the same conformance suite. Keep registries and policy out of Kujo core.

**Recommended name:** Ability, with transport-neutral dotted canonical IDs and explicit legacy aliases.

**Recommended declaration mechanism:** Strict versioned manifest plus small Kujo/library APIs and SDK registration; no native syntax.

**Recommended relationship with CMS:** Reuse the current vertical slice and compatibility surfaces, fix enforcement first, then separate its definitions from CMS handlers, permissions, storage, plugins, and audit.

**Recommended relationship with WebMCP:** Optional, explicit, public-read projection; retain current CMS/SSG built-ins until the adapter proves equal or stronger privacy and origin guarantees.

**Recommended relationship with MCP:** Optional tool projection through an explicit exposure binding; MCP control tools remain protocol-native.

**Recommended first implementation:** Phase 0 CMS safety enforcement followed by a `kujo.ability/v1` contract proof and legacy converter—no production-wide extraction yet.

**Biggest architectural risk:** Treating descriptive metadata as enforced security while collapsing semantic Ability, invocation Tool, policy, and execution into one registry.
