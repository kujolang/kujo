# Effect contracts after the review-checkpoint slice

Design recommendation, unscheduled; no v2 effect contract ships with this work.
Core remains responsible for capability enforcement and confined host operations.
Dispatch owns admission; authorized adapters own observation and attestation.

## Start from the actual v1 contract

`schemas/workflow-control/execution-result-v1.schema.json` already bounds effects
and evidence to 1,000 entries each. An effect has `effect_id`, `class`, `state`,
optional `idempotency_key`, `enforced_by`, `enforcement_evidence_ref` and
`compensation`. Classes are none/local_reversible/external_idempotent/
external_non_idempotent/destructive. States are not_started/started/committed/
compensated/failed_before_commit/unknown. These describe replay properties more
than business operations; replacing them with provider names would be a mistake.

Dispatch `src/core/intervention.kujo::retry_is_effect_safe` rejects unknown
non-idempotent effects and requires nonempty enforcement fields for an external
idempotent effect. **It permits an unknown completion state with those enforcement
references.** `tests/failure_gate_safety_tests.kujo` and the actual sink deduplication
in `tests/reexecution_lifecycle_fixture.kujo` intentionally cover that exception.
This is not universal rejection of every unknown state, nor cryptographic
verification of arbitrary producer assertions. At present trusted adapter inputs
and the operator-owned transport are part of the trust boundary. The new review
checkpoint neither strengthens those assertions nor grants permission to replay.
The golden path uses explicit acceptance without repeating the uncertain action.

## Proposed compatible direction

Retain v1 unchanged until consumers negotiate a version. Prototype an additive
effect-assurance document referenced by existing evidence refs before deciding
whether execution-result/v2 is necessary. Use a schema name only after agreement
across Dispatch, Workcell, SDK/MCP and two real adapter implementations.

Proposed fields and boundaries:

| Field group | Purpose / owner |
| --- | --- |
| Subject | Run, step, attempt, effect ID and execution-result digest; producer binds exact bytes |
| Operation | Stable neutral operation (`read`, `write`, `send`, `deploy`, `transfer`, `unknown`) plus bounded resource kind; adapter maps its API |
| Target / scope | Redacted logical target or digest, tenant/environment scope; no tokens, raw URLs with credentials, message bodies or arbitrary payloads |
| Completion | Existing state plus bounded uncertainty code and external transaction reference; a timeout is not proof of failure-before-commit |
| Replay | Sink-enforced key scope, expiration and receipt; unknown enforcement is never promoted from an asserted key alone |
| Compensation | Supported operation and evidence of its actual completion; availability alone does not mark the original effect rolled back |
| Preconditions | Source/version/transaction conditions and evidence refs; Dispatch verifies applicability to the current attempt |
| Assurance | Claimed / observed / adapter-attested / independently verified as provenance labels, plus issuer and verification evidence; labels alone confer no trust |
| Integrity | Canonical document digest, schema version, issuer key/verification mechanism and validity interval, bounded evidence references |

Suggested initial ceilings: 1 MiB envelope, no more than existing 1,000 effects,
32 evidence refs per assurance, 200-byte identifiers, 4 KiB redacted target refs,
and nesting at most the runtime's existing 64. These are proposed design limits,
not an implemented validator. Never fetch a producer-controlled evidence URL
implicitly; resolution must use an authorized bounded adapter and egress policy.

Dispatch resolves trusted issuer/adapter identity outside the producer payload,
checks signature or authenticated local-channel provenance, subject/digest/key
scope and freshness, then evaluates replay. An absent, malformed, expired or
unverifiable attestation is unknown. Conflicting observations preserve uncertainty.
An assurance upgrade is an append-only evidence/control record, not an edit that
erases the original unknown effect or reclassifies a past attempt as successful.

The `external_idempotent` unknown-state exception can remain only when the chosen
adapter proves that replay uses the same live sink scope/key and deduplication
window. Financial transfers, sends and deployments are not safe merely because
the producer supplies those strings. This requires a compatibility review of
legacy trusted adapters; do not silently change v1 consumers under the same schema.

## Test before adopting

Use a local durable sink with two independent controller processes and a real
transaction log, plus a filesystem adapter. Cover commit-before-reply loss,
expired keys, wrong target/tenant, changed request under same key, forged issuer,
tampered receipt, compensation failure and missing verifier. Assert retained
unknown state and blocked replay for every unverified case. Then add MCP/HTTP
mapping conformance fixtures without importing provider SDKs into core.

This composes with future typed graph nodes and context provenance: edges carry
artifact references and assurance facts, not hidden model reasoning. Universal
interoperability, remote authentication and hosted services remain later ecosystem
work. No exactly-once external effects or universal rollback guarantee follows.
