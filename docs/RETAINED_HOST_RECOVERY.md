# Retained-host recovery and operator reconciliation

Dispatch now provides an experimental local operator path:
`recover inspect` → `recover plan` → `recover apply` → separate review/admission.
The [architecture contract](https://github.com/kujolang/dispatch/blob/main/docs/contracts/recovery/retained-host.md)
and [validation receipt](https://github.com/kujolang/dispatch/blob/main/docs/evidence/retained-recovery/validation.json)
are the authority for exact implementation scope and verification.

A surviving checkpoint and exact immutable control history can support journal
index rebuilding and bounded sequential lifecycle checkpoint reconstruction.
The existing run lock serializes repair; source identities are rechecked, original
sources and operator decision are retained, and authoritative state publishes last.
Consumed claims stay consumed through lost replies, cancellation/rebinding recovery,
controller replacement and interrupted repair. Repair does not call an adapter,
resume a workflow, select a later effect, or infer that a mutation occurred.

Missing authoritative state, torn/conflicting history, corrupt immutable evidence,
missing consumed claims, and unsupported workflow history are not guessed back
into existence. Lifecycle artifacts without their control record require review.
Current authority, evidence freshness and preservation remain separate admission
predicates. Workcell owns workspace/materialization and Git verification;
retained filesystem availability is not replay permission.

Parent finalization remains deferred: independently completed effects do not
provide required parent outputs, evaluation, evidence publication, descendant
barrier satisfaction or a terminal policy decision. The recommended next phase
is a bounded parent-finalization contract toward Wave F, using the existing
execution/evaluation/review vocabulary without collapsing child completion into
parent success. This is more valuable than further local selection variants.

Legacy one-effect selected state remains readable through its existing path but
is not migrated into the sequential lifecycle. Historical bytes and claims remain
unchanged. One negotiated assurance profile per run remains the contract; two
supported sink families do not establish mixed per-effect profiles.

RunLedger, CaseFile and Watchdog can reference Dispatch's existing recovery
receipts as correlation, preserved failure evidence and observation. They do not
own reconstruction or continuation authority. No new integration service or
runtime language feature was needed.

Unsolved: full machine-loss recovery, distributed restore, remote authenticated
trust, hostile storage, multi-host authority, general scheduling, compensation,
cross-sink atomicity, exactly-once, universal rollback, protocol freeze, A2A and
stable/public participant SDKs. The published Kujo 1.6 runtime is unchanged.

Recovery is not replay. Repair is not truth. Reconciliation is not admission.
Preservation is not execution authority. Consumed work stays consumed.
Uncertainty survives until independent evidence resolves it.

One real integration limit was discovered: Workcell v1 preservation evidence uses
`$ref` keys that the existing sequential-selection preservation commitment's
`portable-json/v1` key grammar rejects. Recovery through the review/checkpoint path
works with the unmodified document; richer Workcell preservation plus sequential
selection is not claimed. The next parent-lifecycle architecture slice must first
specify a compatible preservation binding with portable vectors, rather than
changing historical codec bytes or discarding owner evidence.

## Subsequent parent-lifecycle slice

The historical limitations above describe this recovery tranche's original scope.
[Parent finalization](PARENT_FINALIZATION.md) subsequently adds compatible Workcell
`$ref` preservation binding and an explicit locked terminal decision. Recovery can
reconstruct that exact durable event from its predecessor; prerequisites alone still
do not authorize terminal state. General parent/descendant graph composition remains
outside the bounded last-unresolved-parent API.
