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
