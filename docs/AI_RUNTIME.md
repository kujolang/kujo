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

## Egress Controls

The high-level AI helpers use the `network-ai` capability. In `--untrusted` mode, grant them with `--allow-ai`; `--allow-net-client` grants general HTTP/TCP/UDP client APIs but does not unlock AI helpers.

Set `KUJO_AI_ALLOWED_ENDPOINTS` to a comma-separated allowlist of approved AI endpoints. Entries match scheme, host, optional port, and optional path prefix:

```bash
export KUJO_AI_ALLOWED_ENDPOINTS=https://api.example.test/v1,http://localhost:11434/api
kujo run --untrusted --allow-ai examples/ai_egress_allowlist.kujo
```

When the allowlist is unset, trusted-mode behavior remains backward compatible. When it is set, non-matching AI helper endpoints return `kind:"endpoint_denied"`; callers using `options.structured_errors: true` receive the standard structured AI error dictionary. Live AI requests still honor `KUJO_NET_DESTINATION_POLICY` and `--deny-private-net`.

## Secrets And Redaction

`secret(value)` wraps a string in a redacted runtime value:

```kujo
api_key := secret(env_required("OPENAI_API_KEY"))
```

Secrets print and serialize as `***` through the standard display and JSON/TOML/YAML/CSV conversion paths, including when nested inside arrays or dictionaries. Debug formatting renders them as `Secret(***)`. Secret values clone normally and compare by inner value, but `reveal(secret_value)` is the only documented builtin that unwraps plaintext. Use `is_secret(value)` to test the wrapper.

AI helpers accept secrets in `options.api_key` as well as plain strings. The runtime unwraps the key only at the request boundary, and AI errors, response body excerpts, cassette request metadata, cassette response bodies, and sensitive response headers redact configured API keys and authorization material before returning or writing them.

## Response Envelope

The four AI HTTP helpers keep their original success keys and add normalized metadata when providers return OpenAI-compatible fields.

Success dictionaries include:

- Existing keys such as `status`, `model`, `message`, `text`, `json`, `headers`, `vector`, `chunks`, and `messages` where the helper already returned them.
- `usage` when `json.usage` contains `prompt_tokens`, `completion_tokens`, or `total_tokens`.
- `finish_reason`, copied from `json.choices[0].finish_reason` or `null` when absent.
- `provider`, copied from `options.provider` or `""`.
- `tool_calls` for `ai_chat` and `ai_tool_loop`, normalized as `{id, name, arguments_json}` entries.

By default, AI failures still return `Result.err(String)` for backward compatibility. Set `options.structured_errors: true` to opt into a structured error dictionary:

```kujo
result := ai_chat("Hello", {
    "endpoint": "https://api.example.test/v1/chat/completions",
    "model": "gpt-demo",
    "structured_errors": true
})
```

Structured error dictionaries contain `kind`, `message`, `http_status`, `retry_after_ms`, `provider_code`, and `body_excerpt`. Known `kind` values are `http_error`, `rate_limited`, `timeout`, `network`, `decode_error`, and `invalid_response`.

HTTP status `429` maps to `rate_limited`; other 4xx/5xx statuses map to `http_error`. `Retry-After` seconds or HTTP-date values are exposed as `retry_after_ms` when present. Non-JSON successful responses produce `decode_error`; successful JSON that lacks required helper-specific fields can produce `invalid_response`.

Migration note: `structured_errors` is opt-in for this release so existing string error handling keeps working. A future major release may make structured errors the default.

## Structured Output Validation

`json_schema_validate(value, schema)` is the core runtime primitive for validating structured model output, tool arguments, and ordinary JSON-like config. It is intentionally generic rather than `ai_`-prefixed and has no capability gate.

The supported JSON Schema subset is documented in `docs/STANDARD_LIBRARY.md`. It covers practical local validation keywords including `type`, `required`, `properties`, `additionalProperties`, `items`, `enum`, `const`, numeric/string/array bounds, `pattern`, combinators, and local `$ref`. It rejects unsupported keywords and remote references instead of silently passing them.

Validation returns:

```kujo
{
    "valid": true,
    "errors": []
}
```

Each error has `path`, `message`, and `keyword`; paths are JSON-pointer-like instance paths such as `/items/0/name`.

## Vector Math

The core runtime includes generic vector helpers for embedding-style numeric arrays:

```kujo
score := vec_cosine([1.0, 0.0], [0.5, 0.0])
matches := vec_top_k([1.0, 0.0], [[1.0, 0.0], [0.0, 1.0]], 1)
```

Available helpers are `vec_dot`, `vec_norm`, `vec_normalize`, `vec_cosine`, and `vec_top_k`.
They operate on arrays of finite numbers, promote integers to floats, and have no capability gate.

`vec_top_k` returns rows scored by cosine similarity as `{index, score}` dictionaries sorted by descending score. It is a numeric primitive only; vector storage, ANN indexes, persistence, and retrieval policy remain ecosystem concerns.

## Token Estimation And Context Fitting

`ai_count_tokens(text_or_messages, options?)` returns a deterministic estimate for budgeting. It is not exact provider tokenization and should not be presented as a billing-grade count.

The estimator selects a small heuristic family from `options.model`:

- `gpt*`
- `text-embedding*`
- default

All current families estimate one token per four weighted characters, with non-ASCII characters counted as two weighted characters. Chat-message estimates also count role and content text plus family-specific message overhead. This intentionally favors stable local budgeting over provider-specific BPE behavior.

`ai_fit_context(messages, max_tokens, options?)` applies the same estimator to chat messages and drops the oldest non-system messages until the estimate fits:

```kujo
fit := ai_fit_context(messages, 4096, {"model": "gpt-4o"})
```

The result is:

```text
{
    "messages": [...],
    "dropped": 2,
    "est_tokens": 3821,
    "fits": true
}
```

`ai_fit_context` never drops `system` messages and preserves the last `user` message. If that minimum preserved context is still over budget, it returns it with `fits: false`. Exact tokenization, downloadable tokenizer models, retry policy, routing, RAG, and provider selection remain ecosystem concerns.
