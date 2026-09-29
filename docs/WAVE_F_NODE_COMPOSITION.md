# Wave F: bounded producer / consumer composition

Dispatch implementation source: `e4d17263e73890a683e95ed0cb32f785f27df05e`.
The validation receipt identifies the final documentation/evidence publication.

Dispatch extends its existing DAG runner with opt-in **run-backed steps**. Each
node references a separate protected run with its own action, Workcell, effects,
evaluation and parent finalization. This preserves the established single-parent
terminal contract rather than making descendants part of that parent's completion
requirements. Kujo runtime and language syntax are unchanged.

The implementation and verification record live in Dispatch:

- `docs/contracts/node-composition/decision.md`: pre-implementation ownership audit;
- `docs/contracts/node-composition/protocol.md`: identity, edges, admission and limits;
- `docs/audits/node-composition.md`: validation and failure matrix;
- `docs/evidence/node-composition/validation.json`: exact commands, source pins and results.

## Composition boundary

A producer's original execution-result/v1 names an output evidence reference.
Existing parent finalization commits that metadata and its content digest after
independent effects, preservation, evaluation and operator decision are satisfied.
The bounded output family is a JSON document with an explicitly declared schema.
A path, process exit or participant success claim cannot satisfy this contract.

The consumer records an immutable input binding to producer run/step/attempt,
original result, successful finalization identity/revision, output name/type,
schema, evidence metadata and content digest. Raw artifacts remain inspectable by
content identity. The consumer's own run lock and one-use initial admission are
separate from binding. Its effects still cross ordinary Wave C admission and it
must independently evaluate and finalize.

Scheduling remains `depends_on`. Data edges separately name typed producer outputs.
Completion-only edges carry no artifact. This slice bounds composition to static
serial graphs of at most eight required nodes and inputs from scheduling ancestors.
A graph completes only after every declared required node has a durable successful
terminal observation. Producer finalization itself never starts a consumer.

Retained recovery projects exact recorded node transitions under the existing
operator workflow. It cannot invent an edge, input binding, terminal decision or
unconsumed attempt. A permanent claim survives a crash before execution. Recovery
and graph progression remain explainable from retained records, not process memory.

## Minimum reusable node concerns

| Concern | Owner |
| --- | --- |
| Work definition and scheduling edges | Anchored Dispatch workflow/step |
| Data inputs and outputs | Exact artifact identities and typed declarations |
| Execution instance | Independent protected node run and attempt |
| Host capabilities and budgets | Node's installed policy; never inherited through edges |
| External effects | Existing effect lifecycle and sink verification |
| Evaluation | Eval/evaluation-result producer plus Dispatch policy |
| Terminal decision | Existing Dispatch parent finalization |
| Context provenance | Scent/RAG may supply source artifacts; no admission authority |

Programs, tools and agents can produce bounded artifacts through adapters. Human
approval and evaluator nodes need their own terminal predicates; they should not
be forced to fabricate program execution results. This proof does not implement
all those node types or make the participant SDK stable/public.

## Next architecture direction

The follow-on [heterogeneous terminal tranche](WAVE_F_HETEROGENEOUS_TERMINALS.md)
now implements bounded Eval/human terminals and separate graph outcome authority.
The original tranche identified this direction: **heterogeneous node contracts and
graph-level terminal policy**.
Static composition retains enough provenance for this bounded family. The next
substantial gap is expressing evaluation-only and human-decision units, their
failure/completion predicates, and required graph outcomes without treating every
node as a single-parent program run. Keep Wave E source/context provenance as an
explicit input concern. Remote authentication and machine migration remain separate
projects; neither is required to explain the local static graph.

Retain these boundaries: no dynamic graph rewriting, distributed graph execution,
remote trust, machine-loss recovery, mixed assurance profiles within a run,
compensation, exactly-once, universal rollback, public/stable SDK, legacy selected
state migration or new syntax. A successful bounded graph is not universal graph
execution.

Output existence is not producer completion. Producer completion is not consumer
execution. Dependency satisfaction is not effect authority. Provenance is not trust
by itself. A consumer owns its own execution lifecycle.
