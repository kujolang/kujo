# Rejected and deferred candidates

| Candidate | Why it looked reusable | Decision and change condition |
|---|---|---|
| `group_by` / `index_by` | Tribunal and agent code build indexes; collection APIs are broad | Defer. Evidence is mostly domain-specific and current `map`/loops are adequate. Revisit after a cross-repo use-case with stable key/collision semantics. |
| shell quoting | Six repos define `shell_quote`/`shq`-like helpers | Reject as a core helper. Quoting is shell/platform-specific and can encourage unsafe shell execution. Use `spawn_process` argv arrays; only a package may expose a clearly named shell-specific escape with tests. |
| broad HTTP client policy | AI SDK, RAG, Watchdog, and app code retry HTTP | Reject for core. Timeouts, auth, pagination, retryability, and response policy belong to SDKs. Revisit a shared retry package after policy convergence. |
| pluralization/domain text | Many reports format prose | Reject. No stable universal semantics; keep local or domain package. |
| convenience aliases (`is_trueish`, `str_trim`, `str_join`) | Repeated in Lens, PatchBrief, Muzzle, Fence, and tools | Reject aliases. Improve docs and predicate return contracts; aliases would multiply names. |
| universal `safe_write` | CaseFile, MCP, and utilities use the name | Split into atomicity, root policy, size limits, and overwrite behavior. A single broad helper would hide important failure modes. |
| generic retry builtin | Agents SDK, AI SDK, Dispatch have retry loops | Defer to package. Retry requires idempotency, error classification, jitter, deadlines, and observability. |
| automatic redaction on every string | Security-sensitive repos redact outputs | Reject as implicit core behavior. Keep explicit `secret` and explicit redaction profiles. |
| test fixture/golden helpers in core | Several repos use temp dirs and snapshots | Package-only/test support. Evidence is not broad enough and test semantics should not burden production runtime. |

Future evidence that could change these conclusions is a stable public contract,
at least three independent mature consumers, and tests demonstrating the same
edge-case semantics rather than just the same function name.
