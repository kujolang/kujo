# State, resume, retry, and replay

Agents SDK persists session messages, context, run state, and last run ID (`sessions/store.kujo:207-249`), and the runner restores state before beginning execution (`runner.kujo:1598-1640,1892-1919`). It also persists state at many lifecycle boundaries, including failure and completion (`runner.kujo:1914-1924`, `3217-3228`, `3357-3369`). Dispatch computes an attempt envelope and reserves input/output capacity for retries (`dispatch/src/core/runner.kujo:858-922`).

This is strong auditability, but message history, context, run state, events, and artifacts can all carry overlapping content. A retry should not require replaying every prior tool result inline if an immutable artifact reference and hash are sufficient.

## Recommended experiment

Compare full-history replay with structured state plus selective evidence retrieval on identical failure/recovery tasks. State must include completed steps, decisions, source hashes, test outcomes, failure code, next action, and artifact IDs. Acceptance requires identical or better recovery success, no stale-source use, preserved audit retrieval, and lower median input tokens.

Provider prompt caching is billing/latency optimization, not logical reduction. Measure both separately.
