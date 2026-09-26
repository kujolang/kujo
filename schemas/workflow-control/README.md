# Kujo workflow-control contracts

These versioned JSON Schemas define the portable boundary between result
producers, evaluators, policy engines, orchestrators, execution providers, and
human-review systems. They are data contracts, not a required service.

The contracts deliberately separate:

- what an action did (`execution-result`);
- what an evaluator concluded (`evaluation-result`);
- where supporting artifacts live (`evidence-ref`);
- what policy requires (`policy-decision`);
- what execution state was actually retained (`preservation-outcome`);
- what review is requested and what was decided (`intervention-*`); and
- whether another attempt is inspectable, re-executable, or deterministically
  replayable (`reexecution-descriptor`).

Within a major version, producers may add fields and consumers must ignore
unknown fields. Required fields and enum meanings remain stable. A consumer
must reject an unknown major version when the document controls execution.

Evidence URIs are opaque. Consumers must authorize and resolve allowed schemes;
they must not fetch an arbitrary URI solely because it appears in a conforming
document. Secret values are forbidden from these contracts. Use secret-store
references in re-execution descriptors.

Canonical design and lifecycle semantics are documented in
`docs/FAILURE_GATE_EVIDENCE_REVIEW_PLAN.md`.

## Consumer semantics

The September 2026 completion adds `control-event/v1` and optional fields without
renaming existing contracts. `evaluation_status: error|blocked` requires
`verdict: indeterminate`; `fail` remains a completed negative judgment. Stable
`rule_ids` and bounded `metrics` are evaluator-owned extensions.

An execution failure never means no effects occurred. An empty effects list
means no facts were supplied, not proof of purity. A submitted idempotency key
is not enforcement: replay of external effects requires the sink identity and
`enforcement_evidence_ref`. Unknown/partial non-idempotent effects forbid replay.
An explicit `none` effect means the producer attests there were no host effects.
These are trusted producer facts, not authentication guarantees of JSON Schema.

Optional descriptor `mode` distinguishes `evaluator_only`, `same_workspace`,
`clean_workspace`, and `prohibited`; continuation without replay remains an
intervention action. Evaluator mode binds input evidence IDs; same-workspace mode
requires preservation reference; prohibited mode disables automatic retry.
Source/target attempt identities must differ when execution is repeated.
Legacy descriptors remain readable but never grant automatic replay permission.

`control-event/v1` is a portable audit record with sequence, run/revision,
subject, kind, references, details and time. Hash linking and immutable backing
records are optional consumer mechanisms. Consumers must bound document bytes
and nesting in addition to schema collection/string limits, authorize evidence
resolution, and validate legal transitions. A valid actor field is a claim;
authentication and authorization happen at the caller/transport boundary.

Dispatch's reference consumer bounds control documents to 1 MiB, evidence and
effect lists to 1,000, and each active journal to 8 MiB. Control events omit raw
producer `output`. Reference reads do not execute artifact commands or fetch
URLs. See `docs/FAILURE_GATE_IMPLEMENTATION.md` for integration and validation.

Evaluation v1 retains its additional-field compatibility policy. Schema validity
does not grant executable authority to extension fields; control consumers reject
or ignore producer-supplied orchestration commands at admission.

Intervention request v2 optionally includes `unavailable_actions` with an action,
rejection code and explanation. `allowed_actions` is policy authorization;
availability describes current preservation/effect facts and never grants new
authority. Consumers must revalidate admission after the operator responds.
Unknown policy reason codes use the `other` reason type, preserving the original
code as additional metadata.
