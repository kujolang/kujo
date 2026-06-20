# AI Runtime

Status: active draft
Last updated: 2026-06-20

This document tracks core Kujo AI runtime mechanisms. Core owns deterministic, security-sensitive primitives; provider policy, retry strategy, routing, RAG, agents, eval, observability, and registries stay in ecosystem packages.

## Deterministic Request Hashes

`ai_request_hash(prompt_or_messages, options)` returns the hex SHA-256 of a normalized AI request without performing network I/O.

The normalized request includes:

- `_hash_version: 1`
- `endpoint`, trimmed exactly as supplied
- `model`
- request `body`, built from normalized `messages`, `model`, and `options.body`
- relevant `options.headers`

The hash excludes credentials and volatile headers:

- `options.api_key`
- `Authorization`
- `Proxy-Authorization`
- `api-key`
- `x-api-key`
- `Date`
- `User-Agent`
- `X-Request-Id`
- `Request-Id`
- `Idempotency-Key`

Dictionary keys are serialized deterministically through Kujo's JSON conversion. Header names included in the hash are lowercased and sorted with their values. The `_hash_version` field is part of the stability contract so future normalization changes can be versioned explicitly.

`ai_request_hash` is pure, deterministic, and has no capability gate.

## Deterministic AI (Record/Replay)

Kujo AI helpers can replay committed response cassettes without opening a socket. This keeps `ai_chat`, `ai_stream_chat`, `ai_embedding`, and `ai_tool_loop` deterministic in tests and examples.

Environment controls:

- `KUJO_AI_RECORD=<dir>` writes successful AI responses to `<dir>/<hash>.json`.
- `KUJO_AI_REPLAY=<dir>` serves AI responses from `<dir>/<hash>.json`.
- `KUJO_AI_REPLAY_MODE=strict|fallthrough` controls replay misses. `strict` is the default and returns a deterministic `Result.err(String)` containing `kind:"replay_miss"` without using the network. `fallthrough` uses the network on misses and records the response.

Per-call override:

```kujo
result := ai_chat("Hello", {
    "endpoint": "https://api.example.test/v1/chat/completions",
    "model": "gpt-demo",
    "cassette": {
        "mode": "replay",
        "dir": "tests/fixtures/ai_cassettes"
    }
})
```

Supported cassette modes are `off`, `record`, `replay`, `strict`, and `fallthrough`. `replay` and `strict` are equivalent.

Cassette files are JSON and use `_cassette_version: 1`. Each stores credential-free request metadata, the HTTP status, redacted response headers, and the raw response body. Authorization and API-key material are redacted or omitted before writing. Cassettes can still contain model output, so review them before sharing.

Replay lookup happens before destination-policy checks or HTTP client creation, so strict replay remains hermetic even when outbound network access is disabled.
