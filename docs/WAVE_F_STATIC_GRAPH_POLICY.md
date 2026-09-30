# Wave F static failure strategy, branches and subgraphs

Experimental Dispatch-owned composition extends the existing heterogeneous static DAG.
The runtime gains no syntax or scheduler. The implementation contract lives in Dispatch
`docs/contracts/static-graph-policy/`; the prior heterogeneous terminal bytes remain
historical and unchanged.

Failure remains a durable node fact. An anchored host policy separately chooses failure,
blocking, optional continuation, a declared branch or a new authorized review. Closed
conditions inspect finalized node outcomes, exact Eval verdicts or human actions.
Uncertain effects and incomplete evaluation cannot become terminal branch sources.

Branch activation is an immutable, revision-bound control decision under the existing
run lock. All paths exist before the run. The runner schedules only selected paths,
with conditional joins omitting proved inactive predecessors. Inactive nodes remain
pending and are explained as `not_selected`, never success or cancellation. Optional
work and conditional membership remain separate.

A disjoint one-level subgraph groups existing nodes with explicit entry requirements
and a terminal policy. Its internal facts do not imply completion: a separate locked
terminal receipt is required. Parent input bindings retain that exact receipt and the
exact typed output they consume. Subgraph and graph terminal decisions remain Dispatch
control; neither grants a new node effect permission.

The bounded budget counts durable program dispatch reservations. Exhaustion blocks new
dispatch while permitting already-consumed work to be resolved. It does not guess model
usage, costs or elapsed-time guarantees. Same-node retries remain deferred because the
current immutable input/attempt-1 and permanent admission claims need an explicit new
attempt binding/retirement contract. A repair node is independently declared work, not
a retry loophole.

Additive input-binding/v1alpha3 carries branch/subgraph receipt references. Local
terminal-input/v1alpha2 and graph explanation/v1alpha2 are additive. Portable commitments
remain independently checked in Kujo, TypeScript, Python and Go; historical vectors and
execution-result/v1 remain unchanged. Provenance follows source/output → evaluation or
review → branch decision → selected node → output/effect, using externally inspectable
facts, never hidden reasoning.

Current boundary: static serial graphs of at most eight nodes; at most two branches
with unconditional sources and two disjoint unconditional subgraphs. Nested or
branch-containing subgraphs, same-node retries, loops and usage-based graph budgets
remain separate slices. No dynamic topology, participant-owned policy, distributed
scheduling, remote trust, machine-loss recovery, mixed profiles, compensation,
exactly-once, universal rollback, stable/public SDK or new Kujo syntax.

Next architectural question: composition of these bounded groups with explicit graph
budget/attempt accounting and declared failure paths. Extend only from concrete static
use cases; this proof does not justify dynamic graph generation. Typed source/context
lineage remains the Wave E candidate when a retrieval-backed workflow requires it.
