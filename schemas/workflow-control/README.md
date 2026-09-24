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
