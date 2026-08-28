# Eager context findings

| Finding | Classification | Evidence | Impact | Direction |
|---|---|---|---|---|
| Repository instruction layers can be loaded together | COULD BE INDEXED / DEFERRED | `AGENTS.md`, `.github/AGENT_INSTRUCTIONS.md`, `.github/IMPLEMENTATION_GUIDE.md` are separately large and each instructs agents | ~10,565 heuristic tokens before task context | Define precedence and a compact always-present contract; fetch detailed sections by topic |
| Scent reads instruction files and emits their paths in context JSON | COULD BE STRUCTURED | `scent.kujo:1990-2027` | Repeated full instruction content is possible downstream | Pass references/hashes plus selected excerpts; preserve raw artifacts |
| Scout has explicit minimal/full profiles | ALREADY PARTIALLY SOLVED | `scout_runtime.kujo:197-208,2588-2596` | Full artifacts can be avoided for first pass | Make profile selection part of a shared context contract |
| PackWrite deliberately summarizes directories and caps subtrees | ALREADY PARTIALLY SOLVED | `packwrite/src/repo_context.kujo:1-7,175-189` | Reduces whole-repo dumping | Reuse its bounded selection semantics rather than inventing another heuristic |
| Spec agent export repeats priority in two sections and ends with prose instructions | COULD BE SUMMARIZED | `spec/src/export.kujo:32-75` | Small per-task overhead, potentially repeated across agents | Preserve contract fields; make rendered view selectable and canonical |
| Retrieval is injected as a new system message | UNKNOWN — NEEDS EXPERIMENT | `agents-sdk/src/agents/runner.kujo:1508-1522,2101-2105` | May duplicate facts already in messages/history | Add provenance/hash-aware dedupe and measure success impact |

Required eager context: capability/safety policy, current task objective, explicit constraints, and enough routing data to choose tools/roles. Everything else should be deferred, indexed, summarized, cached, or referenced only when a deterministic trigger requires it.
