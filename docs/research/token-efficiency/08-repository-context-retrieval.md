# Repository context and retrieval

Scout emits structure, dependencies, routes, security findings, `llms.txt`, `intelligence.json`, and optional full artifacts; minimal mode emits only `README.md`, `llms.txt`, `intelligence.json`, and `scan_manifest.json` (`scout_runtime.kujo:2588-2596`). Scent adds task, selected files, changed files, commands, constraints, redactions, exclusions, and artifact names to `context.json` (`scent.kujo:2011-2027`). PackWrite caps directory summaries and only reads a short README summary (`packwrite/src/repo_context.kujo:138-145,175-189`).

These are good local-first primitives. The gap is shared identity and invalidation: no inspected contract makes a model-visible source reference uniformly carry content hash, repository revision, selection reason, and freshness policy.

## Correctness model for caching

An artifact may be reused only when repository root identity, commit/working-tree fingerprint, tool version, configuration, redaction policy, and source hash all match. If any input changes, the reference is stale and must be reloaded or the run must fail closed. Retrieval must return paths/spans/citations so the agent can audit what it received.

Prefer structural retrieval (symbols/imports/changed files/dependencies) before semantic retrieval. Semantic indexes can be useful but must not replace exact source evidence for edits or security decisions.
