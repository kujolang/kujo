# Bounded parent finalization and Wave F foundation

This ecosystem slice keeps runtime 1.6.0 and workflow-control v1 wire unchanged.
Dispatch owns terminal authority; Workcell owns preservation observations; Eval
owns evaluation facts. Participant SDKs remain experimental and unpublished.

Dispatch's opt-in finalization policy lives in the existing immutable workflow
configuration. An installed operator API assesses a paused parent against its
fresh complete effect set, consumed/cancelled attempt history, committed required
outputs, current evaluator/configuration and exact evaluated inputs, preservation,
other required steps, and a revision-bound authorized review decision. The current
slice supports one last unresolved parent under one negotiated run profile.

Eligibility is informational. Explicit finalization reacquires the run lock,
revalidates the exact candidate, retains prerequisite bytes, writes an immutable
control event, and publishes terminal state last. Existing policy maps evaluation
to completed/failed/cancelled; review/retry cannot schedule work from this API.
The original execution-result/v1 remains historical, including indeterminate status.

Workcell preservation `$ref` compatibility uses an additive exact-byte document
identity plus separately authorized evidence identities. Historical portable-json/v1
and preservation bytes are unchanged. A bounded installed resolver handles evidence;
no producer URL/path triggers implicit network access. Workcell's new read-only
owner/materialization observation reports current Git HEAD and validates local
retention identity. Workcell retention never grants Dispatch replay authority.

An orphaned **terminal decision** can be reconstructed by explicit retained-host
reconciliation if its predecessor, journal and evidence remain exact. Prerequisites
without a decision leave the parent nonterminal. Consumed attempts remain consumed;
finalization and recovery neither rerun effects nor select later work.

Dispatch owns the detailed [contract](https://github.com/kujolang/dispatch/blob/main/docs/contracts/parent-finalization/protocol.md),
[Wave F crosswalk](https://github.com/kujolang/dispatch/blob/main/docs/contracts/parent-finalization/wave-f-crosswalk.md)
and validation receipt at `docs/evidence/parent-finalization/validation.json`.

## Next major architecture phase

Begin a bounded Wave F composition proof using existing Dispatch DAGs: a producer
node finalizes committed outputs, and a dependent consumer/evaluator receives those
exact references and records its own decision. Define typed artifact/input links
and distinguish scheduling edges from parent completion obligations first. Existing
`depends_on` cannot simultaneously mean “start after parent” and “parent waits for
child”; treating it that way can create a lifecycle cycle.

Keep four linked surfaces separate: execution facts, policy, evidence/provenance,
and orchestration edges. Wave E provenance is a concrete prerequisite within this
composition proof, not a reason to replace the DAG or bundle every concept into one
universal node document.

General graph execution/scheduling, arbitrary parallel effects, mixed per-effect
profiles, legacy selected-state migration, remote authentication, machine migration,
hostile storage, distributed/multi-host authority, compensation, exactly-once,
universal rollback and stable/public participant SDKs remain unsolved.
