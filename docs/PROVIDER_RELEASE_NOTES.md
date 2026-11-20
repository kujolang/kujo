# Provider ecosystem release candidate

Ten provider presets share the Kujo AI SDK transport. Run local inference
with LM Studio, vLLM, llama.cpp or SGLang. Configure a hosted model from Kimi,
MiniMax, SiliconFlow or Novita. Use Voyage or Jina for embeddings and ranking.
The SDK also offers LM Studio model management, MiniMax synchronous speech and
Jina Reader.

The SDK directory and website derive verification from the same manifest.
The directory represents 35 integrations. Ollama and llama.cpp have scoped
live evidence. All other integrations still require applicable live tests.
Use the [SDK setup guide](https://github.com/kujolang/ai-sdk/blob/c699a283f4fe4d97f584488aff78a6dc824f168b/docs/PROVIDER_SETUP.md)
and the [capability matrix](PROVIDER_MATRIX.md) when selecting a deployment.

RAG can use an independent embedding adapter and reranker. Existing built-in
embedding and ranking configuration remains available. Programmatic adapters
record an embedding identity in their index. Rebuild an index when changing
that identity or its output dimensions, and use the new identity for queries.

## Migration

An existing fallback must resolve its own key via its provider's environment
variable or its explicit `api_key`. Move any needed custom fallback headers
into that entry's `options.headers`. The primary key and headers are no longer
forwarded. Existing programs that already isolate credentials need no change.
The provider examples were verified on Kujo 1.9.0.

## Readiness

- **25** existing packages passed fixture regression.
- **10** new presets passed their automated contracts.
- **2** integrations have scoped live verification, one existing and one new.
  **0** meet the full certification gate as applied to this release candidate.
- SDK, RAG, and website verification is documented in the
  [test report](PROVIDER_TEST_REPORT.md). Missing server access, hosted keys,
  the observed llama.cpp named-tool compatibility failure, and outstanding
  end-to-end accountability checks keep the milestone open.
- **Recommendation: hold the 35-verified-provider release.** Complete
  applicable live capability tests and record the tested deployments in the
  manifest before publishing a certification claim.

The source candidates are AI SDK `c699a28` and `c222391`, RAG `befd4cb`, and website `76a3db1`. The audit and release artifacts live in the Kujo repository.
Review and release the SDK before pinning it in published RAG examples.
The website pins the SDK certification manifest at `c699a28` and should be released only after
the relevant SDK distribution and verification are ready. This work does
not publish a package, create a release tag, or deploy the website.
