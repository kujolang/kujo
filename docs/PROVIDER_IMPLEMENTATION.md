# Provider ecosystem implementation

The AI SDK extends its existing OpenAI-compatible request path. Ten provider
presets require an explicit model ID. The certification manifest includes
the 25 standalone packages and the ten presets. The SDK builds an importable
registry projection without reading files during a request, and the website
checks its directory against the same pinned manifest.

## Common inference contract

`configure_provider` returns the provider dictionary consumed by
`create_client`, `chat_completion` and `embeddings`. The existing custom
compatible provider remains available for user-defined API paths and private
gateways. The new presets pin hosted origins and permit explicitly configured
HTTPS or loopback HTTP for local deployments. They preserve optional
authentication, bounded transport, explicit fallback, client-provided
timeouts, custom non-protected headers and explicit model choice. They limit
retries, normalize rate limits, and read bounded integer Retry-After hints.

Capabilities are declared per provider in the manifest. The configuration
preflight uses declared features and explicit deployment opt-ins.
`require_provider_capabilities` allows an orchestration host to check features
before any request. Chat accepts documented model-specific settings; the
shared driver sends request fields and preserves provider-specific reasoning
in the response data. Structured responses go through Kujo's schema validator.
A required tool request fails unless the provider returns a matching tool
envelope with object arguments. SSE completion requires a terminal choice.
Malformed frames and partial streams fail with a normalized error.
`on_transport_chunk` is the live cancellation boundary on Kujo 1.9.0.
The SDK retains the bounded stream for final normalization.

## Independent provider operations

- LM Studio native list, load and unload use REST v1 on the configured server.
- MiniMax synchronous speech uses its native endpoint and returns validated
  hex audio. Requests are explicit and do not auto-retry.
- Voyage and Jina use the existing embeddings call with provider-specific
  option mapping. They enforce valid result count, vector shape, dimensions
  and indices. The separate rerank operation maps response indices and scores
  and preserves provider-reported token totals when supplied.
- Jina Reader accepts an explicit public URL and returns text. It sends the
  URL to Jina's separate origin.
- RAG takes independent trusted embedding and reranking callbacks. It
  validates embedding dimension and ordering, binds the index to an adapter
  ID, re-embeds on a change of ID or dimension, and retains the existing index
  if a rebuild fails. Reranking consumes only the filtered, bounded
  shortlist. The answer-generation configuration is independent.

## Accountability boundaries

SDK completion hooks include provider ID, model, host, timing, error category,
usage and stream completion for registry chat and embeddings. The retry
hook captures attempts. Use caller-selected trace IDs to link attempts and the
terminal result to a RunLedger receipt or a Dispatch run. The normalized
terminal model result can feed the existing Watchdog projection; costs are
available when the host supplies source-qualified data. Native operation
hooks report HTTP execution. Their returned result reports dialect-level
validation. No new telemetry service or automatic receipt writer was added.

Existing Agents SDK and Dispatch code can use configured clients and tool
approval boundaries. Workcell continues to own admission and execution.
Eval and Spec can capture provider/model config and declared requirements.
No third-party tooling was granted a new execution authority. End-to-end
Watchdog ingestion, independent RunLedger receipt generation and cross-repo
live workflow certification remain open. The implementation does not add a
new Kujo syntax or require a core runtime change.
