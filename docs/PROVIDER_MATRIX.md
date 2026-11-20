# Provider capability and verification matrix

The [machine-readable certification manifest](https://github.com/kujolang/ai-sdk/blob/c699a283f4fe4d97f584488aff78a6dc824f168b/providers/certification.json) is the source for this summary, the SDK registry and the website directory. It records each capability independently. **Verified** capabilities passed against the specific named model and server/API build. **Experimental** capabilities have an implementation or documented package surface pending applicable live evidence. Omitted capability columns are available in the manifest as unknown or unsupported.

**35 distinct integrations** are inventoried. **2 have scoped live capability evidence**, and **0 meet the full certification gate**. The count excludes model families available through a router.

| Provider | Integration | Live status | Verified scope | API reference |
| --- | --- | --- | --- | --- |
| Ollama | Native | Partial (2026-11-19) | text generation, chat, streaming, model discovery | [API docs](https://ollama.com/docs) |
| Anthropic | Native | Not Tested | — | [API docs](https://docs.anthropic.com/en/api/messages) |
| Gemini | Native | Not Tested | — | [API docs](https://ai.google.dev/gemini-api/docs) |
| OpenAI | Native | Not Tested | — | [API docs](https://platform.openai.com/docs/api-reference/responses) |
| OpenRouter | Router | Not Tested | — | [API docs](https://openrouter.ai/docs) |
| xAI | Compatible | Not Tested | — | [API docs](https://docs.x.ai/docs) |
| Groq | Compatible | Not Tested | — | [API docs](https://console.groq.com/docs) |
| Together AI | Compatible | Not Tested | — | [API docs](https://docs.together.ai/docs) |
| Fireworks AI | Compatible | Not Tested | — | [API docs](https://docs.fireworks.ai) |
| Mistral | Compatible | Not Tested | — | [API docs](https://docs.mistral.ai) |
| Cerebras | Compatible | Not Tested | — | [API docs](https://inference-docs.cerebras.ai) |
| DeepInfra | Compatible | Not Tested | — | [API docs](https://deepinfra.com/docs) |
| DeepSeek | Compatible | Not Tested | — | [API docs](https://api-docs.deepseek.com) |
| Cloudflare Workers AI | Compatible | Not Tested | — | [API docs](https://developers.cloudflare.com/workers-ai/) |
| Hugging Face | Router | Not Tested | — | [API docs](https://huggingface.co/docs/inference-providers) |
| Replicate | Native | Not Tested | — | [API docs](https://replicate.com/docs) |
| Cohere | Native | Not Tested | — | [API docs](https://docs.cohere.com) |
| Amazon Bedrock | Native | Not Tested | — | [API docs](https://docs.aws.amazon.com/bedrock/) |
| Azure AI | Native | Not Tested | — | [API docs](https://learn.microsoft.com/azure/ai-services/openai/) |
| Vertex AI | Native | Not Tested | — | [API docs](https://cloud.google.com/vertex-ai/docs) |
| NVIDIA NIM | Compatible | Not Tested | — | [API docs](https://docs.nvidia.com/nim/) |
| Perplexity | Native | Not Tested | — | [API docs](https://docs.perplexity.ai) |
| Z.ai / GLM | Compatible | Not Tested | — | [API docs](https://docs.z.ai) |
| fal.ai | Native | Not Tested | — | [API docs](https://fal.ai/docs) |
| Baseten | Compatible | Not Tested | — | [API docs](https://docs.baseten.co) |
| LM Studio | Compatible | Not Tested | — | [API docs](https://lmstudio.ai/docs/developer/rest) |
| vLLM | Compatible | Not Tested | — | [API docs](https://docs.vllm.ai/en/latest/serving/openai_compatible_server/) |
| llama.cpp | Compatible | Partial (2026-11-19) | text generation, chat, streaming, json schema, embeddings, token usage, cancellation, model discovery | [API docs](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md) |
| SGLang | Compatible | Not Tested | — | [API docs](https://docs.sglang.io/docs/advanced_features/structured_outputs) |
| Moonshot AI / Kimi | Compatible | Not Tested | — | [API docs](https://platform.kimi.ai/docs/get-your-api-key) |
| MiniMax | Compatible | Not Tested | — | [API docs](https://platform.minimax.io/docs/api-reference/text-openai-api) |
| SiliconFlow | Compatible | Not Tested | — | [API docs](https://docs.siliconflow.com/en/api-reference/chat-completions/chat-completions) |
| Novita AI | Compatible | Not Tested | — | [API docs](https://docs.novita.ai/guides/llm-api) |
| Voyage AI | Native | Not Tested | — | [API docs](https://docs.voyageai.com/reference/embeddings-api) |
| Jina AI | Native | Not Tested | — | [API docs](https://docs.jina.ai/) |

## Deployment constraints

- **Ollama**. Native chat, streaming and installed model discovery passed. The Qwen2.5-Coder deployment rejected the embedding request. Other native operations have package fixture coverage.
- **LM Studio**. Load a chat or embedding model. Tool calling depends on the template. REST v1 model management requires LM Studio 0.4.0 or later.
- **vLLM**. Use the served model ID. Auto tool choice requires the model-specific parser and server flags. Embeddings require a pooling runner.
- **llama.cpp**. Chat, streaming, schema, model list, cancellation and pooled embeddings passed on b12556. Qwen2.5-Coder 1.5B returned text for a required named tool instead of a tool-call envelope. Tool calling remains experimental.
- **SGLang**. Use a supported serving backend. Tool parsing, constrained generation and embedding mode depend on deployment settings.
- **Moonshot AI / Kimi**. Thinking models may require carrying reasoning_content across tool turns. Use a model ID available to the account.
- **MiniMax**. Preserve reasoning_content across tool turns when required. Audio uses synchronous hex-encoded speech.
- **SiliconFlow**. Model and regional account availability determine tool support. The preset uses the international endpoint.
- **Novita AI**. Reasoning controls and schema support vary by model. The preset uses the documented OpenAI v1 endpoint.
- **Voyage AI**. Text embedding input_type is query or document. Dimensions and input token limits depend on the model.
- **Jina AI**. Text embeddings use a model task where applicable. Reader sends the chosen URL to Jina. Anonymous Reader was rejected from the test network; use a valid Jina key.
