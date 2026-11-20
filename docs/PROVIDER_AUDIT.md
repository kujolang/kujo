# Provider audit

Baseline inspected on 2026-11-19. Kujo `7a132070`, AI SDK `9083c0a`, and website `eb624904` are the starting points. The website registry lists 25 separate configure-and-invoke packages. Model families served through a package are excluded. Package contract tests use fixtures. After correcting module-root discovery for Anthropic, all 45 baseline package suites passed on Kujo 1.9.0. A passing suite establishes the package contract.

| Provider | Source revision | Integration | Entry points examined | Fixture suites passing |
| --- | --- | --- | --- | --- |
| Ollama | `5821d7b` | Native + AI SDK | request_native, call_native, stream_events_native, chat, generate, embed, embeddings, list_models, running_models, show_model, version, pull, push, copy_model, delete_model, parse_stream, ollama_provider, ollama_driver | 2/2 |
| Anthropic | `eaad591` | Native + AI SDK | messages, messages_stream, count_tokens, redact, parse_stream, anthropic_provider, anthropic_driver, encode_messages, encode_tools, output_text, normalized_tools, finish_reason, usage | 2/2 |
| Gemini | `329942f` | Native + AI SDK | generate_content, stream_generate_content, list_models, count_tokens, embed_content, parse_stream, gemini_provider, gemini_driver | 2/2 |
| OpenAI | `c57fab7` | Native + AI SDK | responses, list_models, embeddings, parse_stream, openai_provider, openai_driver | 2/2 |
| OpenRouter | `8f6c239` | Router and OpenAI-compatible | chat, parse_stream, openrouter_provider, openrouter_driver | 2/2 |
| xAI | `3eb8a4a` | OpenAI-compatible | chat, parse_stream, xai_provider, xai_driver | 2/2 |
| Groq | `b2fa992` | OpenAI-compatible | chat, parse_stream, groq_provider, groq_driver | 2/2 |
| Together AI | `573a2b1` | OpenAI-compatible | chat, embeddings, parse_stream, together_provider, together_driver | 2/2 |
| Fireworks AI | `101c9f2` | OpenAI-compatible | chat, embeddings, parse_stream, fireworks_provider, fireworks_driver | 2/2 |
| Mistral | `d8f98a0` | OpenAI-compatible | chat, embeddings, parse_stream, mistral_provider, mistral_driver | 2/2 |
| Cerebras | `3b2fe9d` | OpenAI-compatible | chat, parse_stream, cerebras_provider, cerebras_driver | 2/2 |
| DeepInfra | `70b914e` | OpenAI-compatible | chat, parse_stream, deepinfra_provider, deepinfra_driver | 2/2 |
| DeepSeek | `0339355` | OpenAI-compatible | chat, parse_stream, deepseek_provider, deepseek_driver | 2/2 |
| Cloudflare Workers AI | `3193948` | OpenAI-compatible | chat, embeddings, parse_stream, cloudflare_ai_provider, cloudflare_ai_driver | 2/2 |
| Hugging Face | `cf6f72f` | Router and Native + AI SDK | chat, feature_extraction, list_models, huggingface_provider, huggingface_driver | 2/2 |
| Replicate | `1598bc7` | Native | get_prediction, cancel_prediction, list_models, list_deployments | 1/1 |
| Cohere | `97f6e90` | Native + AI SDK | chat, embed, rerank, cohere_provider, cohere_driver | 2/2 |
| Amazon Bedrock | `ea21064` | Native | converse, invoke_model | 1/1 |
| Azure AI | `a11891c` | Native | chat | 1/1 |
| Vertex AI | `677eb9b` | Native | generate_content, predict | 1/1 |
| NVIDIA NIM | `d706ea6` | OpenAI-compatible | chat, nvidia_nim_provider, nvidia_nim_driver | 2/2 |
| Perplexity | `459184f` | Native + AI SDK | chat, perplexity_provider, perplexity_driver | 2/2 |
| Z.ai / GLM | `798a5ea` | OpenAI-compatible | chat, zai_provider, zai_driver | 2/2 |
| fal.ai | `9403318` | Native | submit, status, result, cancel | 1/1 |
| Baseten | `5c6fc17` | OpenAI-compatible | chat, baseten_provider, baseten_driver | 2/2 |

## Architecture and gaps

- Kujo core owns capability-gated HTTP, destination policy, bounded response collection, incremental HTTP, deterministic AI hashes and cassettes, typed AI inputs, and schema checks. Its native AI builtins accept endpoints rather than consulting a provider package registry. Private-network denial is an independent runtime control.
- AI SDK owns provider/config dictionaries, pure driver hooks, injected and native transport, timeout and retry budgets, SSE and tool-call normalization, structured-response validation, governance, preference resolution, and Watchdog projections. Compatibility presets historically overdeclare capabilities. The built-in encoder always sends bearer authentication and requests streaming usage unconditionally. Native provider packages use descriptors for an independent wire format. Older request paths accept some incomplete or malformed success envelopes.
- Some baseline SDK numeric usage fields use zero as a compatibility default. Downstream consumers need explicit availability semantics. True transport cancellation is available inside native streaming callbacks. Buffered SDK callbacks only replay accepted responses.
- Provider attempt hooks exist. Dispatch owns route policy and attempt recording, Workcell owns admission and artifacts, and Watchdog consumes metadata through the SDK projection. RunLedger correlation, Eval comparisons, Spec requirements, and Shipcheck gates belong at those layers. Agents SDK owns tool invocation and approval. MCP exposes tools and their own schemas. No provider integration should weaken those boundaries.
- RAG embeds through its own hash or native AI path (`src/embeddings.kujo`, `4069658`). Its structured fallback and index dimension invariants need to survive any embedding replacement. It has no SDK rerank stage. Embedding and reranking need independent clients and model IDs.
- Website source already offers name/category/protocol selection. Its published registry is hand maintained independently of SDK. The ten requested candidates are absent from the public registry and SDK presets. Local Ollama was reachable. LM Studio, vLLM, llama.cpp, SGLang and hosted keys require separate live evidence.

## Implementation direction

Extend the existing compatible driver, use a certification manifest as the source of preset metadata, and keep capability status separate from the wire encoder. Use native operations for reranking, Reader, speech, and management. Classify baseline packages by integration type and separately retain live evidence. An SDK mock, a deployment's model list, and a routed model alone cannot certify a provider.
