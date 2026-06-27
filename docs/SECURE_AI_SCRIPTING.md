# Secure AI Scripting

Status: active operator guide
Last updated: 2026-06-27

This guide shows the secure default posture for Kujo scripts that use AI helper APIs. Kujo provides deterministic AI mechanisms and capability controls, but it is not a sandbox; use OS, container, identity, and network policy controls for hostile multi-tenant workloads.

## Secure Default Command

Prefer AI-specific egress over broad network access:

```bash
export KUJO_AI_ALLOWED_ENDPOINTS=https://api.example.test/v1
kujo run --untrusted --allow-ai script.kujo
```

Use strict replay for deterministic tests and demos:

```bash
KUJO_AI_REPLAY=tests/fixtures/ai_cassettes \
KUJO_AI_REPLAY_MODE=strict \
kujo run examples/ai_enterprise_replay_showcase.kujo
```

Add `--deny-private-net` when a trusted script still must not reach loopback, private, link-local, multicast, or unspecified destinations:

```bash
kujo run --deny-private-net --allow-ai script.kujo
```

## Capabilities

- `--allow-ai` unlocks `ai_chat`, `ai_stream_chat`, `ai_embedding`, and `ai_tool_loop` in untrusted mode.
- `--allow-net-client` unlocks general HTTP/TCP/UDP client APIs, but does not unlock AI helpers.
- `--untrusted` starts from deny-by-default capabilities.
- Explicit `--allow-*` flags restrict execution to the listed capabilities.

## Endpoint Allowlist

Set `KUJO_AI_ALLOWED_ENDPOINTS` to a comma-separated list of approved AI endpoint prefixes:

```bash
export KUJO_AI_ALLOWED_ENDPOINTS=https://api.example.test/v1,http://localhost:11434/api
```

Entries match scheme, host, optional port, and optional path prefix. Keep the allowlist as narrow as the deployment allows.

## Secrets

Wrap API keys with `secret(...)` inside Kujo code:

```kujo
let api_key := secret(env_required("OPENAI_API_KEY"))
```

Secrets print and serialize as redacted values. `reveal(...)` is the only documented builtin that unwraps plaintext, and should be confined to provider-boundary code.

## Replay Cassettes

Use strict replay in CI and release examples:

- `KUJO_AI_REPLAY=<dir>` reads committed cassettes.
- `KUJO_AI_REPLAY_MODE=strict` is deterministic and never falls through to the network.
- `KUJO_AI_RECORD=<dir>` and `fallthrough` are fixture-authoring tools, not CI defaults.

Cassettes omit or redact credentials, but they can contain model output and prompt-derived content. Review cassette files before sharing them.

## Structured Errors

Use `structured_errors: true` when operators need typed failure handling:

```kujo
let result := ai_chat("Hello", {
    "endpoint": "https://api.example.test/v1/chat/completions",
    "model": "gpt-demo",
    "structured_errors": true
})
```

The default error shape remains string-based for backward compatibility. Structured errors expose fields such as `kind`, `message`, `http_status`, `retry_after_ms`, `provider_code`, and `body_excerpt`.

## Recommended CI Pattern

```bash
bash scripts/enterprise_verify.sh --minimal
```

For release-candidate evidence:

```bash
bash scripts/enterprise_verify.sh --full
```

Do not run deterministic AI test lanes with live provider credentials or `KUJO_AI_REPLAY_MODE=fallthrough`.
