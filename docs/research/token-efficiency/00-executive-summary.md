# Kujo Token Efficiency & Agent Context Architecture

## Current state

This is a fact-finding audit, not an implementation. Repository state at inspection:

- `kujo`, `main`, `5c8819895c722256cb2aa9088c9b4a1c74b77cb7`; stable release documented as v1.0.1.
- Sibling ecosystem SHAs are recorded in [01-current-architecture.md](01-current-architecture.md).
- Existing unrelated working-tree changes were present before the audit; no source or generated project files were changed.

The largest evidenced model-facing cost is not the Kujo lexer or syntax. It is the outer operating context: `AGENTS.md` (estimated 3,640 tokens), `.github/AGENT_INSTRUCTIONS.md` (4,203), and `.github/IMPLEMENTATION_GUIDE.md` (2,722) can total 10,565 estimated tokens before task-specific material. The 84 installed Kujo skill files total 348,787 bytes, or about 87,197 heuristic tokens, although only selected skills should be loaded. The agent inventory contains 654,638 bytes of agent/manifest/schema material, about 163,660 heuristic tokens, and is a catalog-scale asset rather than a per-run payload.

Kujo already has strong foundations: deterministic AI request hashes and replay cassettes (`docs/AI_RUNTIME.md`), local token estimation, structured errors, explicit capability gates, Scout minimal/full profiles, Scent bounded packs, Spec agent export, Agents SDK typed contracts, budget counters, and Dispatch resume/routing state. The main gap is composition: these mechanisms are not yet one measured, end-to-end context contract.

The most important unknown is actual provider prompt accounting across live agent runs. Current numbers below are measured bytes/characters and deterministic estimates, not provider billing data.

## Top opportunities

| Rank | Opportunity | Priority | Expected reduction | Difficulty | Regression risk | Evidence |
|---:|---|---|---|---|---|---|
| 1 | Instrument every model request as a content-addressed token-cost tree | P0 | Enables reliable savings; amount unknown | Medium | Low if observe-only | SDK usage exists, but no component-level ledger |
| 2 | Establish one canonical layered context contract across Scout/Scent/Spec/Agents SDK | P0 | High on multi-agent runs; estimate after instrumentation | High | Medium | All four already emit partial structured artifacts |
| 3 | Replace full parent/child replay with typed handoff references plus selective evidence | P0 | High for agent-heavy workflows | High | Medium/high | Agents SDK and Dispatch handoffs exist but payloads retain full values |
| 4 | Make tool registration capability/task scoped and expose summary/detail views | P1 | Medium per model call | Medium | Medium | Registry emits full metadata/schema views |
| 5 | Add deterministic repository symbol/dependency retrieval and hash invalidation | P1 | Medium/high for large repositories | High | Medium | Scout has dependency/route analysis; no shared retrieval contract |
| 6 | Separate session state from model-visible message history on resume | P1 | High on retries/resumes | High | High | Session state stores messages/context/run_state; runner persists lifecycle state |
| 7 | Move rare skill references behind explicit load-on-demand references | P1 | Medium for skill invocations | Medium | Medium | Skills are prose-heavy; references are not standardized |
| 8 | Add a CI token ratchet for normalized skills, tools, and scenario payloads | P1 | Prevents future bloat | Medium | Low | No repository-wide prompt baseline found |
| 9 | Verify dynamic dispatch with live structured receipts | P1 | Little direct reduction; prevents wrong-context work | Medium | Low | Dispatch routing and evaluation contracts exist |
| 10 | Evaluate language/stdlib boilerplate only after harness evidence | P2 | Likely low relative to context | High | High | Small examples are compact; SDK implementations are verbose |

## Expected impact

No defensible percentage can be claimed before provider-call traces exist. The current baseline is therefore an evidence boundary, not a cost claim:

| Case | Current baseline | Expected impact | Confidence |
|---|---|---|---|
| Conservative | Exact provider usage unavailable; measured bytes plus Kujo heuristic only | 0% claimed reduction; attribution becomes complete | High |
| Reasonable | Multi-layer duplication confirmed, but not counted per run | Material reduction is plausible from scoped context/handoffs | Medium |
| Optimistic | Several independent payloads collapse to references and selective retrieval | Large cumulative reduction is plausible, not quantified | Low |

Any percentage must be derived from scenario medians and accepted only with task-success/security gates.

## GO

- Instrumentation and normalized component accounting.
- A shared, versioned context manifest that references Scout/Scent/Spec artifacts by hash.
- Typed handoff envelopes carrying objective, constraints, paths, evidence references, and receipts—not full history.
- Task/capability-scoped tool views with explicit detail retrieval.

## EXPERIMENT FIRST

- Progressive disclosure of skill references.
- Repository symbol/dependency retrieval.
- Compact resume state versus full history.
- Any tokenizer-specific compaction or language syntax change.

## DO NOT CHANGE

- Capability gates, secret redaction, replay strictness, structured diagnostics, approval gates, evidence retention, or deterministic ordering merely to save tokens.
- Safety-critical instructions that are always required.
- Human-readable audit artifacts unless a compact model view is added alongside them.

## NEEDS MORE DATA

- Actual live provider token counts by component.
- Cross-provider tokenizer behavior.
- Real dispatch traces proving which roles/tools/skills ran.
- Failure/retry distributions and cache hit rates.

## Single best first implementation project

Build an observe-only “context cost ledger” in the Agents SDK/AI SDK boundary, with adapters for Scout/Scent/Spec artifact manifests. It should hash and count each model input component, tool schema, retrieval injection, handoff, retry, and output reference without changing behavior. This has the lowest regression risk, unlocks all later decisions, and provides rollback by disabling observation. Acceptance requires deterministic JSON receipts, exact-provider usage when available, heuristic estimates otherwise, redaction, replay coverage, and no changed model payloads.
