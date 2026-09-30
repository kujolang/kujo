# Wave F explicit graph attempts and budget accounting

Experimental, retained trusted-host Dispatch work. This document records the bounded
contract. Dispatch retains the exact validation record in
`docs/evidence/graph-attempt-accounting/validation.json` and the proof summary in
`docs/audits/graph-attempt-accounting.md`. Neither constitutes a stable runtime claim.

Dispatch owns logical graph nodes, durable dispatch decisions and their accounting.
Each executable attempt uses an independently controlled child run. The child action
and effect attempt IDs remain separate. A retry does not reset a child, clear a claim
or turn a failed node into a successful historical execution.

The opt-in graph contract separates held reservations, permanently consumed dispatches
and released reservations. Reservation occurs under the existing graph lock before
dispatch. A durable dispatch consumes its unit even if delivery, admission or effect
completion is uncertain. Availability is the anchored ceiling minus held and consumed
units, derived from the journal. Read-only assessments and denied duplicate requests
consume nothing. No counter can refund uncertain work into executable capacity.

A program may declare at most two child runs for one logical node. The initial retry
proof accepts only a Dispatch-recorded refusal before node admission, produced under
the child lock when an installed resource preflight fails. The refusal permanently
seals that child. It is an admission fact, not an execution result. Ordinary program
success still requires its own effects, typed outputs, Eval and parent finalization.

An operator explicitly assesses and applies retry. The new child has a fresh run,
input binding and admission, while binding the same producer outputs, predecessor
terminals and policy receipts. Portable input-binding/v1alpha4 adds ordinal and prior
terminal identity. The additive node-terminal/v1alpha2 commitment describes the
program-attempt terminal path; older terminal variants retain their original bytes.
Historical commitments and execution-result/v1 do not change.
A claimed retryable failure, absent reply, consumed admission, unknown effect, stale
producer or remaining budget is insufficient to authorize retry.

Release is narrower: graph and child locks must prove that no dispatch, admission
claim, result, refusal or running work exists. A released never-dispatched child may
be reserved again. Every release remains in history. A lost dispatch reply is never
a release proof and does not cause automatic redelivery.

Branch activation, human review and subgraph finalization are control events, not
program attempts. Inactive paths consume zero units; subgraph members count once per
dispatch without charging their group receipt again. Eval and decision instances
remain separate categories. No inferred monetary, model-token or timing budget is
introduced. RunLedger reports and Watchdog observes; neither owns accounting authority.

Graph failure policy remains independent. A declared retry with allowance remaining
keeps the graph blocked for an operator decision. Node exhaustion exposes failure to
ordinary graph policy. Graph exhaustion blocks new work; it does not prevent already
consumed work from completing. Finalization binds the exact accounting history and
requires held reservations to be resolved. Recovery reconstructs recorded transitions,
never missing attempts, refunds, retry eligibility or graph terminal authority.

Next major direction: graph resource planning with independently sourced runtime and
Eval measurements, bounded evaluator budgets and fan-out quotas. This work does not
justify dynamic topology or arbitrary loops. Remote trust, distributed scheduling,
machine-loss recovery, mixed assurance profiles, compensation, exactly-once effects,
universal rollback, stable/public SDKs and new language syntax remain outside scope.
