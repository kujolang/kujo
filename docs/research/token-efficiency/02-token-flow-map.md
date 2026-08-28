# Token flow map

## Accounting status

No live provider request trace was available in the inspected repositories. Every numeric value in this report is either measured bytes/characters from checked-out files or an estimate using Kujo's documented `ceil(weighted_characters / 4)` heuristic (`docs/AI_RUNTIME.md:185-212`; `ai-sdk/src/ai_sdk.kujo:1055-1077`). Provider usage is marked unavailable rather than inferred.

## Representative scenarios

| Scenario | Model-facing inputs likely present | Evidence-backed estimate |
|---|---|---:|
| A tiny script | task + selected core instructions + 50-byte script | 3,700–8,000 tokens depending on instructions; the script itself is ~13 tokens |
| B narrow modification | repository instructions + changed files + targeted docs/tests | 8,000–25,000 tokens; exact selection not currently traced |
| C multi-file feature | instructions + spec + source/tests/docs + multiple validation outputs | 20,000–40,000+ tokens; modeled, not measured provider input |
| D agent-heavy workflow | agent instructions + tool schemas + retrieval + handoffs + history | unknown; highest-risk duplication path |
| E package/tool use | package metadata/lock data + tool contracts + source context | unknown; package data is structured but no shared token ledger exists |
| F failure/retry | prior messages/results + error + repeated model input | at least one full prompt replay per retry in the current AI boundary unless provider caching applies |

## Concrete component measurements

| Component | Bytes | Heuristic tokens | Status |
|---|---:|---:|---|
| `AGENTS.md` | 14,560 | 3,640 | measured/estimated |
| `.github/AGENT_INSTRUCTIONS.md` | 16,810 | 4,203 | measured/estimated |
| `.github/IMPLEMENTATION_GUIDE.md` | 10,886 | 2,722 | measured/estimated |
| `docs/STANDARD_LIBRARY.md` | 83,819 | 20,955 | measured/estimated |
| `docs/LANGUAGE_SPEC.md` | 22,800 | 5,700 | measured/estimated |
| `docs/AI_RUNTIME.md` | 10,649 | 2,663 | measured/estimated |
| `agents-sdk/src/agents/runner.kujo` | 121,771 | 30,443 | source size, not default payload |
| `dispatch/src/core/runner.kujo` | 65,951 | 16,488 | source size, not default payload |
| `scent/scent.kujo` | 109,725 | 27,432 | implementation size, not default payload |
| `scout/lib/scout_runtime.kujo` | 115,246 | 28,812 | implementation size, not default payload |

## Attribution tree to implement later

```text
run
├── stable policy/instructions
├── selected skill core
├── loaded references
├── tool schemas
├── task/spec
├── repository manifest/context
├── retrieved source/evidence
├── handoff references
├── prior state/history
├── tool results
├── retry/replay duplication
└── assistant output
```

Each node needs bytes, characters, measured/estimated tokens, source, load step, reason, cacheability, hash, and security classification. The absence of this tree is the primary measurement gap.

## Output-side flow

Current lifecycle events retain detailed steps, retrieval state, citations, errors, artifacts, and run state for auditability. The model-facing response should eventually be a concise status plus immutable artifact references; detailed evidence must remain fetchable. This is a representation decision to test, not permission to delete events.

## Provider independence

The current core estimator is provider-neutral but deliberately approximate. AI SDK usage normalization accepts both `prompt_tokens`/`completion_tokens` and `input_tokens`/`output_tokens` (`ai-sdk/src/ai_sdk.kujo:117-145`). Future measurements must keep logical payload size, provider-reported usage, and cache/billing savings as separate fields for OpenAI-style, Anthropic-compatible gateway, and open-weight/local tokenizers.
