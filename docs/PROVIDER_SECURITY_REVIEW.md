# Provider security review

Review scope is the new AI SDK registry/operation paths, the fallback change,
and RAG callback boundaries. Existing standalone package release tests
remain the baseline.

| Boundary | Behavior | Evidence |
| --- | --- | --- |
| Endpoint and credentials | Hosted presets use fixed HTTPS origins. A local override can opt into a separately named credential. Loopback HTTP supports explicitly configured localhost inference. | Registry endpoint and authorization tests. |
| Redirects | Registered calls use no redirects. The SDK host allowlist applies per selected endpoint, including the Jina Reader origin. | SDK transport policy and local HTTP transport tests. |
| Fallback | Each fallback resolves its own credential or explicit override. It receives only its own custom headers. | Fallback test asserts the alternate key and omission of primary headers. |
| HTTP headers | The registered path protects Authorization, content type and request routing/framing headers. Header validation rejects control characters. | Driver security and registry contract tests. |
| Private addresses | The runtime enforces `--deny-private-net` or `KUJO_NET_DESTINATION_POLICY=deny_private` when enabled. Enable it independently for a managed workload. | SDK host allowlist test and live Kujo loopback denial with `--deny-private-net`. The same localhost request succeeded when the setting was omitted. |
| Diagnostics | Registered HTTP errors suppress provider error text, which can echo prompts. Evidence hooks report normalized metadata. Successful result data may contain output, reasoning or documents. Persist audit fields explicitly. | Error sanitizer and Watchdog event tests. |
| Bounds and cancellation | The registered path uses a bounded response size, bounded retry count, timeouts and terminal stream checks. The native callback returns false to cancel transfer without a replay. | Contract tests and llama.cpp cancellation smoke. |
| RAG provider access | Only trusted host code supplies callbacks. They receive document/query text for their explicit operation. The retrieval path validates returned dimensions and indices, omits callback diagnostics from API errors and rejects mixed index identity. | RAG adapter and API regression tests. |

Managed endpoint configuration belongs in trusted host code. Third-party
drivers and injected transports are executable dependencies. The host should
protect their configuration and enforce its own outbound destination policy.
For self-hosted services on private remote networks, grant access through an
approved host policy. TLS certificate verification stays with the runtime.
Jina Reader callers must authorize the chosen public URL before transmitting
it. Live TLS and credentialed error-path coverage for the hosted providers
remains pending.
