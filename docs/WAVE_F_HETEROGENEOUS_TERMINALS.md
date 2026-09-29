# Wave F: heterogeneous terminals and graph outcome policy

The follow-on to [bounded composition](WAVE_F_NODE_COMPOSITION.md) keeps Dispatch's
existing static DAG and independently protected executable nodes. It adds two
non-executing terminal forms and a separate graph-level outcome decision. Kujo
runtime and syntax are unchanged. This is experimental trusted-host architecture,
not universal graph execution or a stable/public node API.

## Tested terminal forms

- **Program/tool/agent-compatible executable:** existing parent receipt after own
  execution, effects, outputs, evaluation, preservation and review as configured.
- **Evaluator:** actual evaluation-result/v1 over an exact finalized producer subject,
  original result and output digest, with pinned evaluator configuration. Dispatch's
  existing policy mapping determines whether the node can complete, fail, cancel,
  or remain nonterminal for review.
- **Human review:** existing intervention-request/v2 and intervention-decision/v2,
  exact graph/node/input subject, current revision and installed actor authorization.
  Approval, rejection and cancellation remain distinct facts.

Node kind is descriptive. The anchored terminal contract selects a closed built-in
validator; participant data cannot install one. Evaluator and human nodes do not
fabricate execution-results, Workcells or external effects. Their terminal decisions
can satisfy a consumer prerequisite but never supply that consumer's effect authority.

## Graph outcome and provenance

Scheduling remains `depends_on`, requiring successful terminal prerequisites.
Typed inputs bind exact producer artifacts separately. Anchored `optional` determines
contribution to the graph outcome; it cannot be changed to waive required work.
Optional cancellation does not satisfy a success dependency or erase a prior attempt.

All required successful terminals make a graph eligible for completion. Required
failure/rejection or cancellation can make it eligible for the corresponding bounded
unsuccessful policy outcome. Outstanding dispatches or consumed unfinished executable
work prevent graph finalization, including optional work. Pending review, unresolved
evaluation and missing current evidence remain blocked/nonterminal.

The runner's readiness scan does not publish graph success. A separate operator
assessment and locked, revision-bound revalidation append graph terminal authority.
The receipt binds exact node outcomes, immutable definition and requirement policy,
authority and actor evidence. Failure does not rewrite producer output, Eval, review
history or consumed work. Recovery projects an actual durable terminal event; all
facts without that event remain nonterminal. Competing finalizers use the existing
run lock.

Consumer provenance now explains both data and heterogeneous prerequisites:
producer original result/output → exact input → Eval/human decision → node terminal
→ consumer binding → independent execution/effect/finalization → graph decision.
No chain-of-thought or implicit trust propagation is recorded. Wave E source/context
lineage may later attach Scent/RAG artifacts without granting them control authority.

## Universal and non-universal concerns

Universal across tested nodes: identity, exact subject/instance, configured terminal
predicate, durable facts/evidence, outcome, authority and finalized revision.
Not universal: process exit, execution-result, Workcell, effects, Eval, artifact
output, human actor or model usage. Those remain variant-specific requirements.
The common terminal identity is not a replacement execution-result schema.

Dispatch owns definition, dependency resolution, admission, outcome policy and graph
terminal decision. Eval owns judgments; Workcell owns environment facts; authorized
human adapters produce decision evidence. RunLedger can report these receipts but
cannot decide outcomes. Existing participant bytes, parent receipts, alpha/beta
contracts and one profile per run remain unchanged. New portable identities have
additive Kujo/TypeScript/Python/Go vectors.

Canonical implementation/evidence is in Dispatch:

- `docs/contracts/heterogeneous-nodes/decision.md`: source-based crosswalk;
- `docs/contracts/heterogeneous-nodes/protocol.md`: contracts, ownership and policy;
- `docs/contracts/heterogeneous-nodes/crash-matrix.md`: retained-host boundaries;
- `docs/evidence/heterogeneous-nodes/validation.json`: tested commits and gates.

## Next major phase

Continue Wave F with **graph failure strategy and bounded conditional branches or
subgraphs**, including explicit terminal and budget policy. The current static
heterogeneous proof supplies distinct facts and terminal authority; the next useful
question is how declared alternate paths contribute to outcome. It does not justify
agent-owned topology or arbitrary dynamic graph creation. Wave E typed source/context
provenance remains a separate candidate when a real retrieval workflow needs it.

Still out of scope: distributed scheduling, dynamic arbitrary mutation, remote trust,
machine-loss recovery, mixed profiles, compensation, exactly-once, rollback,
public/stable SDK, new syntax and legacy selection migration.
