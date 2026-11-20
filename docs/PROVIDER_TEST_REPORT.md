# Provider verification report

Evidence date is 2026-11-19. The main runtime for verification was the published
Kujo 1.9.0 macOS x64 binary. Its release checksum matched. The starting package
inventory and checkout revisions are in [the audit](PROVIDER_AUDIT.md).

## Automated gates

| Scope | Command | Result |
| --- | --- | --- |
| AI SDK | `KUJO_BIN=/path/to/kujo bash scripts/release_quality_gates.sh` | Passed. 194 Kujo tests, including nine provider contract tests, and the wrapper, runtime HTTP, schema, documentation and benchmark gates. `bash scripts/supply_chain_policy_check.sh` passed. The standard live smoke used offline behavior with no key configured. |
| Existing providers | `KUJO_MODULE_PATH="$PWD/src:$SDK_ROOT:$SDK_ROOT/src" kujo test-run tests/<suite>_tests.kujo` from each package in the audit | Passed. 45 suites across 25 packages on the updated SDK. Module-root discovery for Anthropic requires the explicit SDK root. |
| RAG | `KUJO_RAG_TEST_FILES=tests/test_unit.kujo,tests/test_integration.kujo,tests/test_provider_retrieval.kujo,tests/test_api_contract.kujo,tests/test_security.kujo KUJO_BIN=/path/to/kujo kujo run scripts/run_tests.kujo` | Passed. Five suites, zero warnings. |
| Website | `FACTORIES_OUTPUT=output npm test`, `npm run providers:check`, pinned certification projection check, `kujo run ./build.kujo -- --site-url https://kujolang.ai` | Passed against website `76a3db1`. All 35 Node tests passed against built output. The build produced 73 pages and the Providers page contained 35 cards with two partial and 33 pending live statuses. |
| Kujo core | `cargo fmt --check`, `git diff --check` | Passed. Core changes are reports only. A source build was stopped when disk space fell below 1 GiB. |

The provider-specific SDK fixtures exercise all ten presets and the chat,
tool, schema, embedding, rerank, model management, speech and Reader wire
contracts. They cover failure paths including absent credentials/capabilities,
unsafe endpoints, malformed vectors, missing required tool calls, stream
interruption, protected headers and rate-limit handling. HTTP transport tests
use a controlled local server for bounds, redirects and stream delivery.

## Live results

- **llama.cpp b12556, Qwen2.5-Coder 1.5B Instruct.** Real localhost model list,
  chat, SSE streaming, schema-constrained generation and callback cancellation
  passed. A second deployment with `--embedding --pooling mean` passed batched
  embeddings. The named-tool check failed because the model/template returned
  a JSON-looking text answer with `finish_reason: stop`. The SDK returns
  `required_tool_call_missing`. Keep that capability experimental.
- **Ollama 0.40.16, Qwen2.5-Coder 1.5B Instruct.** Existing native package chat,
  model discovery and streaming passed. The model server rejected embeddings
  for this deployment. Other package capabilities remain uncertified.
- **LM Studio, vLLM and SGLang.** Not Tested. No configured running servers
  were available. The local Docker daemon was stopped. The Intel test machine is outside [LM Studio’s macOS requirements](https://lmstudio.ai/docs/app/system-requirements) and [vLLM’s native macOS support](https://docs.vllm.ai/en/latest/getting_started/installation/cpu/).
- **Moonshot, MiniMax, SiliconFlow, Novita, Voyage and Jina.** Authenticated
  requests were Not Tested because the provider credentials and live model
  selections were unavailable. A separate anonymous Jina Reader request
  hit a transport failure through Kujo. A control request returned
  `AuthenticationRequiredError` due to the test network's IP reputation.
- **Remaining existing providers.** Package fixtures passed. Live
  certification was unavailable in this session.

Reproduce the llama.cpp checks by starting two localhost servers using b12556
and the GGUF for Qwen2.5-Coder 1.5B Instruct. The tested model blob SHA-256 was
`29d8c98fa6b098e200069bfb88b9508dc3e85586d20cba59f8dda9a808165104`.

```sh
llama-server --host 127.0.0.1 --port 8080 --jinja --ctx-size 2048 --threads 3 --alias qwen-test --model /path/to/model.gguf
llama-server --host 127.0.0.1 --port 8083 --embedding --pooling mean --ctx-size 1024 --threads 2 --alias qwen-embed-test --model /path/to/model.gguf
```

From AI SDK, use separate terminals for each server and run:

```sh
LLAMA_CPP_MODEL=qwen-test KUJO_BIN=/path/to/kujo bash scripts/verify_provider_live.sh
KUJO_MODULE_PATH="$PWD/src" KUJO_PROVIDER_BASE_URL=http://127.0.0.1:8083/v1 \
  kujo run examples/provider_quickstart.kujo -- llama-cpp embed qwen-embed-test
```

The live runner returns nonzero for a failed operation and prints missing
prerequisites as `Not Tested`. Site and manifest records retain both
capability-level and integration-level verification state.

## Performance and limits

The SDK's release benchmark passed all fixture, parsing, and retry thresholds.
A separate VM A/B smoke compared warmed clients sharing the same injected JSON
chat transport, with three alternating batches of 300 requests on each path.
Legacy batches took 3,094–3,513 ms, median 3,193 ms. Registry batches took
3,265–4,624 ms, median 3,673 ms. That is approximately 1.6 ms higher median
per fixture call on this run. Host contention and the small sample preclude
a production inference claim. Cancellation passed against real llama.cpp.
Long-running concurrent resource leak tests and provider-specific throughput
measurements remain open.
