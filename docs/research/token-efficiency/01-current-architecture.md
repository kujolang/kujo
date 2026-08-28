# Current architecture and evidence boundary

## Scope

The audit inspected the Kujo runtime and the local sibling repositories that provide the agent execution path. It did not inspect a hosted Codex/Claude control plane or undocumented provider-side prompt assembly; those are outside this checkout.

| Repository | Branch | SHA | Relevant role |
|---|---|---|---|
| `kujo` | main | `5c8819895c722256cb2aa9088c9b4a1c74b77cb7` | language, VM/interpreter, CLI, core AI primitives |
| `agents-sdk` | main | `d3904d348754b492bda298b6c30f49c1eb24b7ea` | agent runner, tools, retrieval, sessions, handoffs, budgets |
| `ai-sdk` | main | `be9617a32344728919b1394b80f72f46559d69a7` | provider adapter and usage normalization |
| `kennel` | main | `012c0a251dafc4984899fe320c0407cdb492fc05` | package/project metadata and lock/index contracts |
| `dispatch` | main | `662417c264bd55f8d802eef3fc21f9f372590753` | workflow routing, retries, resume, state, reports |
| `scout` | main | `682490313f0e1424d731eaf7892455569e171cee` | repository intelligence and context packs |
| `scent` | main | `6e72cd06bf422a29e93148cf4edaddd98d937392` | task-scoped context packaging and redaction |
| `packwrite` | main | `f63425187d39580b36032b1f31731e3f086db9f1` | mega-prompt to agent-pack compiler |
| `spec` | main | `d5103347e83289634a6e6423007f8fe52ea24759` | structured task contracts and agent export |
| `kujo-agents` | main | `abe00dcffee2933877cf5a85d3c68c2176515194` | role/agent manifests, skills, schemas |
| `kujo-skills` | main | `f41e39c49a9228a1e7c763f804016fbb21675f30` | installable skills and routing index |
| `workcell` | main | `7bcdb7f29ddf74843aec6b70eafbf33cc7944c6f` | execution boundary and receipts |
| `kujolang-mcp` | main | `9009b7d9d4527bf2f57fd4d318061a72c48e31e6` | MCP manifest surface |

## Context lifecycle map

```text
task/spec/mega prompt
  -> Spec export or PackWrite prompt
  -> Scout repository map + Scent task pack
  -> agent config + selected tools + retrieval
  -> Agents SDK runner builds messages
  -> AI SDK/provider adapter sends messages and normalizes usage
  -> tool/retrieval/handoff/retry events
  -> session/memory/artifact stores + Dispatch/Workcell receipts
  -> resume/retry/report
```

Evidence: Kujo core explicitly leaves routing, RAG, agents, retry, and observability to ecosystem packages (`docs/AI_RUNTIME.md:6`). Agents SDK documents the same boundary in `docs/ARCHITECTURE.md`. The runner calls `build_model_messages`, optional retrieval injection, then the AI adapter (`agents-sdk/src/agents/runner.kujo:2101-2166`).

## Unrelated or out of scope

The lexer, parser, VM, and ordinary standard-library execution do not transmit model context. They matter for language token density and agent repair ergonomics, but not for prompt assembly unless their diagnostics or docgen outputs are exposed to an agent.
