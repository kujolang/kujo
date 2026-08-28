# Baseline measurements

## Method

Measurements were taken from the checked-out files at the SHAs in the architecture report. Bytes and lines are direct filesystem measurements. Heuristic tokens are `ceil(bytes/4)` for the ASCII-dominant material; this is comparable to Kujo's documented estimator but is not a provider tokenizer or billing count. Source implementation size is not automatically model-facing payload size.

## Skills and agents

- 84 `SKILL.md` files: 348,787 bytes, approximately 87,197 heuristic tokens total.
- The largest skill is `kujo-docgen-agent-readable/SKILL.md` at 9,398 bytes (~2,350 tokens); other large files include eval (9,212), dispatch (8,813), packwrite (8,570), and Kujo-way development (8,054).
- Agent manifests, `AGENT.md`, and JSON schemas under `kujo-agents`: 654,638 bytes, approximately 163,660 heuristic tokens total. This is a catalog inventory, not evidence that all are sent per run.
- Repeated vocabulary is widespread: “deterministic” appears in 42 skills, “security” in 41, “tests” in 65, “Do not” in 71, and “Use this skill” in 84. These are overlap indicators, not proof that prose is redundant.

## Kujo examples

| Fixture | Bytes | Estimated tokens | Lines |
|---|---:|---:|---:|
| `examples/hello.kujo` | 50 | 13 | 5 |
| `examples/test_simple_func.kujo` | 111 | 28 | 7 |
| `examples/testing_demo.kujo` | 613 | 154 | 29 |
| `agents-sdk/examples/hello_agent.kujo` | 1,115 | 279 | 37 |
| `agents-sdk/examples/tool_agent.kujo` | 1,437 | 360 | 53 |
| `agents-sdk/examples/handoff_agent.kujo` | 1,761 | 441 | 61 |
| `examples/database_transactions.kujo` | 6,738 | 1,685 | 177 |

## Interpretation

Tiny Kujo source is not the dominant cost in a fresh agent run. The 50-byte hello program is dwarfed by a single repository instruction file. Larger SDK examples are useful and should not be deleted without behavioral teaching tests; their cost is more relevant when examples are eagerly included in skills.

## Missing measurements

The following must be collected before setting numeric budgets: raw provider input/output usage, serialized tool schemas, actual selected skill payloads, retrieval payloads, parent/child messages, retries, cache hits, and task outcomes. A baseline update is not valid if it only counts source files or Markdown sizes.
