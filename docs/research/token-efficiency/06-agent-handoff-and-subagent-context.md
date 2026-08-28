# Agent handoff and subagent context

## Current evidence

Agents SDK has explicit `HandoffRequest`, `HandoffResult`, loop/depth controls, and visited-target metadata (`agents-sdk/src/agents/handoffs/handoff.kujo:111-245`). Its `AgentContext` contains input, messages, runtime, and state (`core_types.kujo:357-370`). Dispatch handoffs store a full `payload` plus a `prior_context_summary` (`dispatch/src/core/handoff.kujo:1-10`), which is a duplication risk when the payload already contains the relevant state.

The runner builds model messages from the agent and request, then appends retrieval context, while session state separately retains messages/context/run_state (`runner.kujo:2101-2105`; `sessions/store.kujo:207-219`). This creates multiple representations whose relationship is not yet a documented compactness contract.

## Typed envelope recommendation

Use a versioned handoff with objective, constraints, relevant paths, evidence/artifact IDs, test commands, decisions, unresolved questions, capability grants, source hashes, and next action. Large content should be an artifact reference with explicit fetch authorization. The receiver must be able to discover detail without receiving it by default.

Required tests: schema validation, no implicit parent-history inheritance, source-hash mismatch refusal, loop/depth protection, redaction, receiver fetch behavior, and live dispatch receipt proving the target ran.
