# Core Kujo AI-Native Enhancements — Implementation Proposal

> **Status:** Proposal for review. **Type:** design + implementation spec.
> **Audience:** a frontier coding model (and human reviewers) who will implement the work.
> **Scope:** the **core Kujo language/runtime only** (`kujo` repo). Explicitly **not** the
> ecosystem repos (`ai-sdk`, `rag`, `mcp`, `dispatch`, `eval`, `watchdog`, `kennel`, …).
> **Do not implement from this draft until the owner approves it.**

## Progress Tracker

| Item | Status | Commit |
| --- | --- | --- |
| Item 1 — Structured response envelope | Completed | `556bb93` |
| Item 2 — Native AI record/replay | Completed | `a7652ff` |
| Item 3 — JSON Schema validation builtin | Completed | `0fbab11` |
| Item 4 — Native vector math for embeddings | Pending | TBD |
| Item 5 — Token estimation & context fitting | Pending | TBD |
| Item 6 — `secret(...)` value type with runtime-enforced redaction | Pending | TBD |
| Item 7 — AI egress capability + endpoint allowlist | Pending | TBD |
| Item 8 — Deterministic request hashing / cache key | Completed | `1e669be` |
| Item 9 — True streaming with a Kujo callback (or chunk iterator) | Pending | TBD |
| Item 10 — Portable multimodal message builders | Pending | TBD |

This document specifies a set of additions to core Kujo that make the language meaningfully
**more AI-native** by strengthening the runtime *mechanisms* every AI library needs — **without
duplicating the policy/orchestration work already done in the ecosystem**. Each item is
self-contained, file-referenced, and carries acceptance criteria so it can be implemented and
verified independently.

Background analysis this builds on (read first): `KUJO_LANGUAGE_DEEP_DIVE.md`,
`AI_NATIVE_CAPABILITY_MATRIX.md`, `KUJO_VS_ECOSYSTEM_REASSESSMENT.md`,
`ECOSYSTEM_AI_CAPABILITY_MATRIX.md`.

---

## 0. Guiding principle: core owns *mechanism*, ecosystem owns *policy*

The ecosystem already implements the *policy/orchestration* layer (retry strategies, provider
catalogs, fallback/routing, RAG pipelines, agent loops, MCP servers, eval, observability
dashboards). Re-implementing any of that in core would duplicate work and create two competing
sources of truth.

Core should instead provide the **low-level primitives that are (a) hard or impossible to do
well in pure Kujo, (b) security- or determinism-sensitive, or (c) performance-critical** — the
things *every* AI library currently has to hack around the 4 existing `ai_*` builtins to get.

**The litmus test for "belongs in core":** _Would `ai-sdk`, `rag`, `dispatch`, `watchdog`, and
a hand-written user script **all** benefit from this being a single, native, deterministic
primitive — and is it painful or unsafe to do in userland today?_ If yes → core. If it is a
strategy, a catalog, a workflow, or an opinion → ecosystem.

Every proposal below is **win/win/win**:
- **Core wins:** more robust, more testable (notably, fixes the skip-prone AI tests), more secure.
- **Ecosystem wins:** libraries build policy on a solid mechanism instead of re-parsing HTTP/JSON.
- **End users win:** determinism, portability, security, and lower token/cost surprises.

---

## 1. Current state (authoritative reference for the implementer)

All AI builtins live in `src/interpreter/native_functions/http.rs` and are dispatched from
`handle(name, arg_values)` (line ~464). There are exactly 4:

| Builtin | Line | Returns (today) |
| --- | --- | --- |
| `ai_chat(prompt_or_messages, options)` | 727 | `Result.ok(Dict{status, model, message, text, json, headers})` / `Result.err(Str)` |
| `ai_stream_chat(prompt_or_messages, options)` | 787 | `Result.ok(Dict{status, model, …, text, json, headers})` / `Result.err(Str)` |
| `ai_embedding(input, options)` | 856 | `Result.ok(Dict{status, model, vector, text, json, headers})` / `Result.err(Str)` |
| `ai_tool_loop(prompt_or_messages, options)` | 926 | `Result.ok(Dict{…})` / `Result.err(Str)` |

Key internal helpers (reuse these; do not fork them):
- `struct AiRequestConfig { endpoint, model, api_key, timeout_seconds, headers }` (line 15).
- `parse_ai_request_config(options, surface)` (line 105) — validates `endpoint`/`model`/`api_key`/`timeout`/`headers`.
- `parse_ai_messages(input, surface)` (line 174) — string → `[{role:"user",content}]`, or passthrough array.
- `merge_ai_extra_body(payload, options, reserved, surface)` (line 242) — merges `options.body`.
- `run_ai_request(surface, &config, body) -> Result<(status:i64, headers:Dict, text:String, json:Value), String>` (line 286).
- `ai_ok_result(value)` / `ai_err_result(message)` (lines 23–29) — wrap in `Value::Result`.
- `extract_chat_content(json)` — pulls assistant text from a chat response.

**Gaps observed (verified):** `ai_*` drop token **usage**, `finish_reason`, and (for `ai_chat`)
`tool_calls`; non-2xx errors collapse to a **flat string** (losing http status / `Retry-After`
/ provider error code); there is **no** tokenizer, **no** vector math, **no** JSON-Schema
validation, **no** secret/redaction type, **no** record/replay, and the AI unit tests
**self-skip** when the sandbox denies a local TCP bind (so a green suite can hide regressions).

Registration touch-points the implementer will use repeatedly (the "add a builtin" recipe):
1. **Dispatch:** add a `"name" => { … }` arm in the appropriate `native_functions/*.rs` `handle()`.
2. **Public registry:** add the name to `Interpreter::get_builtin_names()` (`src/interpreter/mod.rs:487–878`).
3. **Arity:** add an entry in `Interpreter::native_callable_arity()` (`src/interpreter/mod.rs:2639`).
4. **Capability (if side-effecting):** map it in `capabilities::capability_for_native_function()`
   (`src/interpreter/capabilities.rs:109`) and, if a new capability is introduced, extend
   `enum NativeCapability`, `RuntimeCapabilityPolicy`, and the CLI flag wiring in `src/main.rs`.
5. **Alias (optional):** add to `canonical_native_function_name()` (`src/interpreter/mod.rs:345`).
6. **Docs:** add a row to `docs/STANDARD_LIBRARY.md` (keep the registry↔docs parity).
7. **Tests:** unit tests next to the impl + a `.kujo` fixture under `examples/` and/or `tests/`.

---

## 2. Cross-cutting requirements (apply to every item)

- **Backward compatibility is mandatory.** Existing `ai_*` return keys (`status`, `model`,
  `message`, `text`, `json`, `headers`, `vector`) must keep working unchanged. New behavior is
  **additive** (new keys) or **opt-in** (new `options.*` flags). Any change to the *error* shape
  must be opt-in for this release (see Item 1).
- **Feature gating.** Put genuinely new/heavy subsystems behind a Cargo feature
  (`runtime-ai`, default-on) so the language can be built without them. Pure-additive helpers
  (vector math, schema validation) can be unconditional.
- **Determinism contract.** Any builtin that does not perform I/O must be pure and deterministic.
  Builtins that perform model I/O must be fully replayable offline (Item 2) and must never
  introduce hidden global state beyond the documented record/replay store.
- **Security contract.** No builtin may print, log, or serialize an API key or `secret(...)`
  value (Item 6). Network egress remains capability-gated; AI endpoints respect the allowlist
  (Item 7). Schema/tokenizer/vector builtins must bound allocation and reject NaN/Inf/oversized
  inputs (follow the existing "bounded native helper" pattern in `builtins.rs`).
- **Error style.** Validation failures return `Value::Error("<surface>() requires …")` exactly
  like the existing helpers. Model-call failures return `Result.err(...)`.
- **Docs + tests are part of "done".** Each item updates `docs/STANDARD_LIBRARY.md`, adds an
  example, adds unit tests, and (for AI I/O) adds a replay fixture so CI runs offline.

---

## 3. Proposals

Priority key: **P0** = highest leverage / unblockers, **P1** = high value, **P2** = valuable.
Each item: *Gap → Why core → Win/win/win → API → Behavior → Implementation → Edge cases →
Security/determinism → Tests → Docs → Acceptance criteria → Non-goals.*

---

### Item 1 — Structured response envelope: usage, finish_reason, tool_calls, and typed errors (P0)

**Gap.** `ai_chat`/`ai_stream_chat`/`ai_embedding`/`ai_tool_loop` discard token **usage** and
`finish_reason`; `ai_chat` discards `tool_calls`; failures collapse to a flat string so callers
cannot see HTTP status, `Retry-After`, or provider error codes. Every retry policy, cost
ledger, and observability tool in the ecosystem re-parses `result["json"]` to recover these.

**Why core.** The data is already in `run_ai_request`'s `(status, headers, text, json)` tuple;
normalizing it once, natively, removes N re-implementations and makes the contract stable.

**Win/win/win.** ai-sdk builds retry/backoff on `error.kind`/`error.retry_after_ms` without
HTTP parsing; runledger/watchdog read `usage` directly; users get cost/limits for free.

**API (additive).** On **success**, add keys to the existing OK dict (do not remove any):
```
usage:        { prompt_tokens, completion_tokens, total_tokens }   # ints; absent fields omitted
finish_reason: <string|null>                                       # e.g. "stop","length","tool_calls"
tool_calls:    [ { id, name, arguments_json } ]                    # ai_chat + ai_tool_loop; [] if none
provider:      <string>                                            # echo of options.provider if given, else ""
```
On **failure**, introduce an opt-in structured error via `options.structured_errors: true`
(default `false` this release; plan default `true` in next major). When enabled, `Result.err`
carries a **Dict** instead of a String:
```
{ kind, message, http_status, retry_after_ms, provider_code, body_excerpt }
# kind ∈ { "http_error","rate_limited","timeout","network","decode_error","invalid_response" }
# retry_after_ms parsed from Retry-After header (seconds or HTTP-date) when present, else null
```
When `structured_errors` is unset/false, keep today's `Result.err(Str)` verbatim (no break).

**Behavior.** Map status 429 → `kind:"rate_limited"`; 5xx/4xx → `"http_error"`; timeout/connect
failures from `run_ai_request` → `"timeout"`/`"network"`; non-JSON 2xx → `"decode_error"`;
2xx-but-missing-expected-fields → `"invalid_response"`. Extract `usage` from
`json.usage.{prompt_tokens,completion_tokens,total_tokens}` (OpenAI shape) and, if absent, leave
`usage` omitted (do not fabricate). Extract `finish_reason` from `json.choices[0].finish_reason`.

**Implementation.** In `http.rs`: add `fn extract_usage(json)`, `fn extract_finish_reason(json)`,
`fn extract_tool_calls(json)`, `fn classify_ai_error(status, headers, text) -> Dict`,
`fn parse_retry_after(headers) -> Option<i64>`. Wire them into the four `handle` arms after the
`run_ai_request` match. Add a `structured_errors: bool` field parsed in `parse_ai_request_config`
(or a small `parse_ai_result_options`). Reuse `ai_ok_result`/`ai_err_result`; add
`ai_err_structured(dict)`.

**Edge cases.** Streaming usage often arrives in a terminal SSE chunk — capture it there
(coordinate with Item 9). Some providers nest usage differently; only support the
OpenAI-compatible shape and document that. Never panic on missing/odd JSON.

**Security/determinism.** `body_excerpt` must be truncated (reuse `truncate_for_error`, ≤240
chars) and must pass through redaction (Item 6) so it cannot leak keys.

**Tests.** Unit tests with mocked JSON for: usage present/absent, finish_reason, tool_calls,
429 + `Retry-After: 3`, 500, non-JSON 2xx, and `structured_errors` on/off backward-compat. Use
the record/replay store (Item 2) so they run without a live socket.

**Docs.** Update the `ai_*` rows in `docs/STANDARD_LIBRARY.md`; add a "Response envelope"
subsection documenting every key and the error taxonomy.

**Acceptance criteria.** (1) Existing keys unchanged; existing tests pass untouched.
(2) `usage`/`finish_reason`/`tool_calls` populated when present in provider JSON. (3) With
`structured_errors:true`, a 429 yields `kind:"rate_limited"` and numeric `retry_after_ms`.
(4) No key material appears in any error field.

**Non-goals.** Retry execution, backoff timing, provider-specific JSON dialects beyond
OpenAI-compatible — those stay in `ai-sdk`.

---

### Item 2 — Native AI record/replay ("cassettes") for deterministic, offline runs (P0)

**Gap.** AI calls are nondeterministic and require network; core's own AI unit tests **skip when
local TCP bind is denied**, and every ecosystem tool re-invents fixture mode. There is no
runtime-level way to record a real model interaction and replay it byte-for-byte.

**Why core.** Only the runtime sits at the actual HTTP boundary inside `run_ai_request`. A
runtime cassette makes *all* `ai_*` calls (in any library, test, or script) deterministic with
zero code changes — something no library can provide for code that doesn't use it.

**Win/win/win.** Core tests stop skipping; ai-sdk/dispatch fixture modes can delegate to it;
users get reproducible demos/CI and can share captured sessions.

**API.** Env-gated, no syntax:
```
KUJO_AI_RECORD=<dir>     # capture each request+response to <dir>/<hash>.json
KUJO_AI_REPLAY=<dir>     # serve responses from <dir>; miss → deterministic error (no network)
KUJO_AI_REPLAY_MODE=strict|fallthrough   # strict (default): replay-miss errors; fallthrough: hit network then record
```
Optional per-call override: `options.cassette: { mode, dir }`.

**Behavior.** Compute a stable request key = hash of `{normalized_endpoint, model, sorted_body,
relevant_headers_excluding_auth}` (Item 8 provides the hash). In RECORD mode, after a successful
`run_ai_request`, persist `{request_meta, status, headers(redacted), body}`. In REPLAY mode,
short-circuit `run_ai_request` to return the stored tuple; a miss returns a structured
`kind:"replay_miss"` error (never a silent network call in strict mode).

**Implementation.** Add `src/interpreter/native_functions/ai_cassette.rs` (or a submodule of
`http.rs`): `fn cassette_mode() -> Mode`, `fn lookup(key) -> Option<StoredResponse>`,
`fn store(key, resp)`. Insert a hook at the top of `run_ai_request` (replay) and after success
(record). **Redact** `Authorization`/`api-key` headers before writing (Item 6). Keep the store
format JSON and human-diffable.

**Edge cases.** Streaming (Item 9): record the ordered chunk list; replay re-emits them.
Concurrency: file writes must be atomic (temp file + rename). Hash stability across runs is the
contract — pin the normalization rules and version the cassette format (`"_cassette_version":1`).

**Security/determinism.** Never record secrets. Replay must be hermetic (no socket in strict
mode, enforced even in trusted mode). Document that cassettes may contain model output and
should be reviewed before sharing.

**Tests.** Round-trip: record (against a mock), then replay with networking disabled and assert
identical envelope. Convert the existing skip-prone `ai_*` tests to ship a committed cassette so
they run everywhere.

**Docs.** New `docs/AI_RUNTIME.md` section "Deterministic AI (record/replay)".

**Acceptance criteria.** (1) `KUJO_AI_REPLAY` makes all four `ai_*` builtins run with networking
fully disabled. (2) Cassette files contain no `Authorization` material. (3) The previously
skip-prone AI tests now pass in a no-network environment via committed cassettes.

**Non-goals.** A caching *policy* (TTL, semantic cache, dedupe heuristics) — that's ecosystem.

---

### Item 3 — JSON Schema validation builtin: `json_schema_validate` (P0)

**Gap.** Structured-output and tool-argument validation is impossible to do robustly in pure
Kujo; nothing in core validates a value against a schema. (Useful far beyond AI.)

**Why core.** Schema validation is a generic, perf-sensitive, widely-needed primitive; doing it
natively once is safer and faster than N userland validators.

**Win/win/win.** ai-sdk validates structured outputs and tool args; eval/spec validate
artifacts with one engine; users validate any JSON/config.

**API (generic name — not `ai_`):**
```
json_schema_validate(value, schema) -> { valid: bool, errors: [ { path, message, keyword } ] }
```
Support a practical JSON-Schema **subset**: `type`, `required`, `properties`,
`additionalProperties`, `items`, `enum`, `const`, `minimum`/`maximum`,
`minLength`/`maxLength`, `pattern`, `minItems`/`maxItems`, `anyOf`/`oneOf`/`allOf`, `$ref`
(local only). Document the supported subset explicitly; reject unknown keywords with a clear
error rather than silently passing.

**Implementation.** New `src/interpreter/native_functions/schema.rs` with `handle(name, args)`;
wire into the dispatch chain in `native_functions/mod.rs`. Operate on Kujo `Value` directly (no
serde round-trip) for speed. No new capability (pure compute). Register name + arity + docs.

**Edge cases.** Recursive schemas via `$ref` must guard against cycles (depth limit). Numbers:
respect Kujo Int/Float promotion. `pattern` uses the existing `regex` dependency; bound regex
size. Bound recursion/array sizes to avoid DoS.

**Tests.** Valid/invalid for each keyword; nested objects; `anyOf`; `$ref` cycle guard; large
input bound. Pure unit tests (no network).

**Docs.** New row + a "JSON Schema subset" reference section.

**Acceptance criteria.** (1) Correct `valid`/`errors` for the documented subset. (2) Error paths
are JSON-pointer-like (`/items/0/name`). (3) Bounded on adversarial input. (4) Zero new capability.

**Non-goals.** Full Draft 2020-12 conformance, remote `$ref` resolution, schema *generation*.

---

### Item 4 — Native vector math for embeddings: `vec_cosine`, `vec_dot`, `vec_norm`, `vec_normalize`, `vec_top_k` (P1)

**Gap.** RAG/semantic search needs fast similarity; pure-Kujo loops over float arrays are slow
and verbose. There is no native vector math.

**Why core.** Numeric kernels are the canonical "native for performance" case; they are generic
(not RAG-specific) and small.

**Win/win/win.** `rag` gets fast similarity without core shipping a vector DB; any numeric user
benefits; core stays free of storage/indexing concerns.

**API (generic numeric):**
```
vec_dot(a, b) -> float
vec_norm(a) -> float
vec_normalize(a) -> [float]
vec_cosine(a, b) -> float                       # in [-1, 1]
vec_top_k(query, matrix, k) -> [ { index, score } ]   # cosine; matrix = [[float]]
```
Inputs are Kujo arrays of numbers (Int promoted to Float).

**Implementation.** New `src/interpreter/native_functions/vector.rs`. Use `rayon` (already a
dep) for `vec_top_k` over large matrices. Validate equal dimensions; reject NaN/Inf; bound
matrix size. No capability (pure compute).

**Edge cases.** Zero vectors → cosine `0.0` (documented), not NaN. Dimension mismatch →
`Value::Error`. `k > rows` → return all, sorted desc.

**Tests.** Known-value cosine/dot/norm; orthogonal/identical vectors; top_k ordering &
truncation; dimension-mismatch error; zero-vector handling.

**Docs.** New "Vector math" section.

**Acceptance criteria.** (1) Numerically correct within 1e-9. (2) `vec_top_k` returns indices
sorted by descending cosine. (3) No NaN escapes. (4) Parallelizes for large matrices.

**Non-goals.** Vector storage, ANN indexes (HNSW/IVF), persistence — all stay in `rag`.

---

### Item 5 — Token estimation & context fitting: `ai_count_tokens`, `ai_fit_context` (P1)

**Gap.** Tools must budget context windows and avoid `length` truncation, but core offers no
token counting; pure-Kujo char/4 hacks are everywhere.

**Why core.** A consistent, deterministic estimator native to the runtime gives every tool the
same budgeting math; doing it once avoids drift.

**Honesty constraint.** A *true* BPE tokenizer is heavy and model-specific. **Do not overclaim.**
Ship a documented, deterministic **estimator** with selectable heuristics per model family, and
label it an estimate (with a stated typical error band). Leave exact tokenization to providers
or an optional ecosystem package.

**API.**
```
ai_count_tokens(text_or_messages, options?) -> int          # options.model selects heuristic family
ai_fit_context(messages, max_tokens, options?) -> { messages, dropped: int, est_tokens: int }
```
`ai_fit_context` trims oldest non-system messages (documented strategy) until the estimate fits.

**Implementation.** New module or extend `http.rs`. Heuristic table keyed by model-family prefix
(`gpt`, `text-embedding`, default). Deterministic, no I/O, no capability.

**Edge cases.** Empty input → 0. Message arrays: count role + content per the documented model.
`ai_fit_context` must never drop `system` messages; if even system+last user exceeds budget,
return them with `dropped` reflecting reality and a flag.

**Tests.** Stable counts for fixed inputs (golden); fit-context drop order; system preserved.

**Docs.** Document the estimator, its heuristics, and the accuracy caveat prominently.

**Acceptance criteria.** (1) Deterministic, documented estimates. (2) `ai_fit_context` never
drops system messages and reports `dropped`/`est_tokens`. (3) Clear "estimate, not exact" docs.

**Non-goals.** Exact provider tokenization, downloadable tokenizer models — ecosystem/optional.

---

### Item 6 — `secret(...)` value type with runtime-enforced redaction (P1)

**Gap.** API keys and sensitive strings leak via `print`, `to_json`, error messages, and (now)
cassettes. Libraries redact ad hoc; only the runtime can enforce it everywhere.

**Why core.** Redaction must be enforced at every output site (`print`, `to_string`, `to_json`,
error formatting, cassette writes). Only the runtime controls all of those.

**Win/win/win.** watchdog/casefile/ai-sdk get guaranteed non-leakage; users get safe-by-default
secrets; core's own error/cassette paths become safe.

**API.**
```
secret(value: string) -> Secret       # wraps a string as a redacted value
reveal(s: Secret) -> string           # explicit, audited unwrap (use only when sending to a provider)
is_secret(v) -> bool
```
`options.api_key` should accept either a plain string (today) **or** a `Secret`.

**Implementation.** Add `Value::Secret(Arc<String>)` to `src/interpreter/value.rs` (~40 existing
variants). Render as `"***"` (or `"secret(***)"`) in `Display`, `Debug`, `to_json`, and error
formatting (search `src/interpreter/mod.rs` value→string paths and `http.rs` error builders).
Equality: secrets compare by inner value but never print it. `reveal` is the only path to the
plaintext; consider gating `reveal` behind a capability or at least logging its use.

**Edge cases.** Secrets inside arrays/dicts must redact when the container is printed/serialized.
Cassette writer (Item 2) and `body_excerpt` (Item 1) must treat secrets and `Authorization`
headers as redacted.

**Security/determinism.** Document that `reveal` is the trust boundary. Ensure no `Debug` derive
anywhere prints the inner string (audit `#[derive(Debug)]` on `Value`).

**Tests.** `print(secret("k"))` → `***`; `to_json` of a dict containing a secret redacts; key
sent to a (mock/replayed) provider still authenticates via `reveal`; secret survives clone/eq.

**Docs.** New "Secrets & redaction" section; update env/credentials guidance.

**Acceptance criteria.** (1) A `Secret` never appears in print/to_json/errors/cassettes.
(2) `options.api_key` accepts `Secret`. (3) `reveal` is the sole, documented unwrap.

**Non-goals.** Key management, vaults, rotation — ecosystem/ops.

---

### Item 7 — AI egress capability + endpoint allowlist: `NetworkAi` / `--allow-ai` (P2)

**Gap.** Calling models requires the broad `NetworkClient` capability, so granting "may talk to
approved LLM endpoints" to untrusted/agent code also grants general outbound HTTP.

**Why core.** Capability gating and egress allowlists are runtime security mechanisms; the
runtime already maps builtins→capabilities (`capabilities.rs:109`).

**Win/win/win.** Orchestrators (dispatch) run agent code with *least* privilege; users get a
tight blast radius; core security story strengthens.

**API.**
```
CLI:   --allow-ai
ENV:   KUJO_AI_ALLOWED_ENDPOINTS="https://api.openai.com,https://api.example.com"
```
`ai_*` builtins require `NetworkAi` (not `NetworkClient`) when present; if the allowlist is set,
the resolved `options.endpoint` must match (scheme+host, optional path prefix) or the call fails
with a structured `kind:"endpoint_denied"` error.

**Implementation.** Add `NativeCapability::NetworkAi` (enum + name + flag string in
`capabilities.rs`); add `network_ai` to `RuntimeCapabilityPolicy`; wire `--allow-ai` in
`src/main.rs`; map the four `ai_*` names to `NetworkAi` in `capability_for_native_function`.
Add `network_policy::ai_endpoint_allowed(endpoint)` consulted inside `parse_ai_request_config`
or `run_ai_request`. Honor the existing `--deny-private-net` and destination policy.

**Backward compatibility.** In **trusted** mode (default), `ai_*` keep working with no flag.
The new gating only bites in `--untrusted` runs and when the allowlist is set. Decide and
document whether trusted mode also enforces the allowlist when the env var is present
(recommended: yes, env opt-in).

**Tests.** Untrusted + `--allow-ai` permits a replayed call; untrusted without it is denied;
allowlist hit/miss; private-net interaction.

**Docs.** Update the security posture doc and README capability section.

**Acceptance criteria.** (1) `ai_*` callable under `--untrusted --allow-ai` but not under
`--untrusted` alone. (2) Allowlist denies non-listed endpoints with a structured error.
(3) Trusted-mode default behavior unchanged unless the allowlist env is set.

**Non-goals.** Provider auth management, per-key quotas — ecosystem.

---

### Item 8 — Deterministic request hashing / cache key: `ai_request_hash` (P2)

**Gap.** Record/replay (Item 2) and any userland cache need a stable content-addressed key for
an AI request; there's no canonical hashing of `(prompt/messages, options)`.

**Why core.** The normalization rules must match what record/replay uses; exposing the same hash
lets libraries key their own caches identically.

**Win/win/win.** Item 2 reuses it internally; ai-sdk/rag key caches consistently; users get
reproducible cache keys.

**API.**
```
ai_request_hash(prompt_or_messages, options) -> string   # hex sha256 of the normalized request
```
Normalization: canonical-JSON the body (sorted keys), include `endpoint`+`model`, **exclude**
`api_key`/`Authorization` and volatile headers. Reuse `sha256` (already in `crypto`).

**Implementation.** Small function in `http.rs` reusing `parse_ai_messages`/`merge_ai_extra_body`
plus a canonical-JSON serializer (add `fn canonical_json(value) -> String` if not present).

**Tests.** Same logical request → same hash regardless of key/header order; differing
model/messages → different hash; api_key changes do **not** change the hash.

**Docs.** Document the normalization contract (it is a stability guarantee; version it).

**Acceptance criteria.** (1) Stable across runs/orderings. (2) Independent of credentials.
(3) Matches the key used by Item 2.

**Non-goals.** Cache storage/eviction — ecosystem.

---

### Item 9 — True streaming with a Kujo callback (or chunk iterator) (P2)

**Gap.** `ai_stream_chat` does not give Kujo code incremental tokens; SSE parsing and
cancellation are not feasible in pure Kujo. Streaming usage (Item 1) also lives in the terminal
chunk.

**Why core.** SSE framing, backpressure, and cancellation are runtime concerns.

**Win/win/win.** UIs/agents stream tokens; ai-sdk exposes ergonomic streaming; record/replay
captures chunk order.

**API (pick one; callback recommended for simplicity):**
```
ai_stream_chat(prompt_or_messages, options, on_chunk)   # on_chunk(delta_text, raw_chunk_json) -> (bool? to continue)
# returns the same aggregated envelope as today (back-compat) plus usage/finish_reason from the final chunk
```
If a callback is omitted, behavior is exactly today's (aggregate-and-return) — **back-compat**.
Returning `false` from `on_chunk` cancels the stream.

**Implementation.** Parse `text/event-stream` in `run_ai_request`'s streaming path; for each
`data:` line, decode JSON, extract `choices[0].delta.content`, invoke the callback via the
interpreter (see how `ai_tool_loop`/native callbacks invoke Kujo functions). Aggregate for the
return value. Integrate with cassette record/replay (store ordered chunks).

**Edge cases.** `[DONE]` sentinel; partial lines across buffers; callback errors abort cleanly;
cancellation closes the connection. Bound total buffered size.

**Tests.** Replayed multi-chunk stream invokes callback in order; cancellation stops early;
aggregate matches non-streaming; final usage captured.

**Docs.** "Streaming" section with the callback contract and cancellation semantics.

**Acceptance criteria.** (1) Without a callback, identical to today. (2) With a callback, deltas
arrive in order; `false` cancels. (3) Works fully under replay.

**Non-goals.** UI rendering, token-rate shaping — ecosystem.

---

### Item 10 — Portable multimodal message builders: `ai_message`, `ai_text`, `ai_image_url` (P2, optional)

**Gap.** Building OpenAI-compatible multimodal message arrays by hand is error-prone; there's no
portable constructor.

**Why core.** Cheap, pure helpers that standardize the message shape `parse_ai_messages` already
consumes; reduces malformed-request bugs in generated code.

**Win/win/win.** Agents/users build valid messages reliably; ai-sdk reuses them; core's message
parsing stays the single shape.

**API.**
```
ai_text(content) -> ContentBlock
ai_image_url(url, detail?) -> ContentBlock
ai_message(role, content_or_blocks) -> Message     # content can be a string or [ContentBlock]
```

**Implementation.** Pure dict builders in `http.rs`; ensure `parse_ai_messages` accepts the
produced shape. No capability.

**Tests.** Round-trip through `parse_ai_messages`; mixed text+image; string-content shortcut.

**Docs.** "Building messages" section.

**Acceptance criteria.** Built messages are accepted unchanged by `ai_chat`/`ai_tool_loop`.

**Non-goals.** Prompt templating, message stores — ecosystem.

---

## 4. Explicit non-goals (these stay in the ecosystem — do **not** build in core)

- Retry/backoff **policy**, fallback, model **routing** → `ai-sdk`/`dispatch`.
- Provider **catalogs**/registries, per-provider JSON dialects → `ai-sdk`.
- **RAG** pipelines, chunking strategies, vector **storage**/ANN indexes, persistence → `rag`.
- **Agent loops**, memory schemas, recall APIs, planning → `agents-sdk`.
- **MCP** servers/clients → `mcp`.
- **Evaluation** harness, scoring, CI gates → `eval`.
- **Observability** dashboards, proxies, telemetry storage → `watchdog`.
- **Package registry** transport → `kennel`.
- Prompt templating/management, semantic caching policy, cost catalogs (price tables) → ecosystem.

Core provides *mechanism* (parse, validate, hash, count, redact, record/replay, vector math,
typed envelope, egress capability); ecosystem provides *policy* (when/how/which).

---

## 5. Phasing, dependencies, and priority

```
Phase A (P0, unblockers):      Item 1 (envelope/usage/typed errors)
                               Item 2 (record/replay)        ← depends on Item 8's hash
                               Item 8 (request hash)          ← do alongside Item 2
                               Item 3 (json_schema_validate)  ← independent
Phase B (P1, high value):      Item 4 (vector math)           ← independent
                               Item 5 (token estimate/fit)    ← independent
                               Item 6 (secret/redaction)      ← Items 1 & 2 must honor it
Phase C (P2, hardening/ergo):  Item 7 (NetworkAi capability)
                               Item 9 (streaming callback)    ← integrates with Items 1 & 2
                               Item 10 (message builders)     ← optional
```
**Recommended first PR:** Items 8 + 2 + 1 together — they convert the skip-prone AI tests into
deterministic, offline tests and deliver the typed envelope every ecosystem tool wants.

---

## 6. Global definition of done

A change set implementing any item is "done" when **all** hold:
1. `cargo build --release` succeeds; `cargo clippy` clean for touched files.
2. `cargo test` passes, **including** new offline (replay-backed) AI tests; no test relies on a
   live socket; previously skip-prone `ai_*` tests now run via committed cassettes.
3. `Interpreter::get_builtin_names()` updated; `test_builtin_names_do_not_contain_duplicates`
   still passes; arity registered; capabilities mapped.
4. `docs/STANDARD_LIBRARY.md` updated (registry↔docs parity preserved) and a new
   `docs/AI_RUNTIME.md` covers the AI runtime contracts (envelope, errors, cassettes, secrets,
   streaming, egress).
5. At least one runnable `examples/*.kujo` per user-facing builtin; `kujo check` passes on it.
6. Backward compatibility verified: existing `ai_*` return keys and default error shape
   unchanged unless explicitly opted in.
7. Security review: no key/secret can appear in print/to_json/errors/cassettes; egress remains
   capability-gated; new pure builtins are allocation-bounded and reject NaN/Inf/oversized input.
8. `CHANGELOG.md` and `ROADMAP.md` updated; if a new major-version behavior is introduced
   (e.g. default `structured_errors`), it is documented with a migration note.

---

## 7. Appendix — quick file map for the implementer

| Concern | File:location |
| --- | --- |
| AI builtins + helpers | `src/interpreter/native_functions/http.rs` (`handle` ~464; `run_ai_request` 286; `parse_ai_request_config` 105; `ai_ok_result`/`ai_err_result` 23–29) |
| Builtin registry | `src/interpreter/mod.rs:487` (`get_builtin_names`) |
| Arity | `src/interpreter/mod.rs:2639` (`native_callable_arity`) |
| Aliases | `src/interpreter/mod.rs:345` (`canonical_native_function_name`) |
| Dispatch chain | `src/interpreter/native_functions/mod.rs:42` (`call_native_function`) |
| Capabilities | `src/interpreter/capabilities.rs` (enum line 2; mapping line 109) |
| CLI flags / policy | `src/main.rs` (clap `--allow-*`), `RuntimeCapabilityPolicy` in `capabilities.rs:54` |
| Value variants | `src/interpreter/value.rs:542–696` (add `Secret` here) |
| Value→string / serialization | `src/interpreter/mod.rs` (display/to_json paths), `src/builtins.rs` (json) |
| Network policy | `src/network_policy.rs` |
| Crypto (sha256 for hashing) | `src/builtins.rs` / `native_functions/crypto.rs` |
| Docs to update | `docs/STANDARD_LIBRARY.md`, new `docs/AI_RUNTIME.md`, `README.md`, `CHANGELOG.md`, `ROADMAP.md` |

---

## 8. Summary table (for review triage)

| # | Builtin(s) / change | Pri | New capability? | Back-compat risk | Primary ecosystem beneficiary |
| --- | --- | --- | --- | --- | --- |
| 1 | typed envelope: usage/finish_reason/tool_calls/errors | P0 | no | low (opt-in errors) | ai-sdk, watchdog, runledger |
| 2 | `KUJO_AI_RECORD/REPLAY` cassettes | P0 | no | none | all + core tests |
| 3 | `json_schema_validate` | P0 | no | none | ai-sdk, eval, spec |
| 4 | `vec_cosine/dot/norm/normalize/top_k` | P1 | no | none | rag |
| 5 | `ai_count_tokens`, `ai_fit_context` | P1 | no | none | ai-sdk, rag, dispatch |
| 6 | `secret`/`reveal`/`is_secret` + redaction | P1 | maybe (reveal) | low | watchdog, casefile, ai-sdk |
| 7 | `NetworkAi` / `--allow-ai` + endpoint allowlist | P2 | **yes** | low (untrusted only) | dispatch |
| 8 | `ai_request_hash` | P2 | no | none | ai-sdk, rag |
| 9 | streaming callback | P2 | no | none (callback optional) | ai-chat, ai-sdk |
| 10 | `ai_message/ai_text/ai_image_url` | P2 | no | none | ai-sdk, agents-sdk |

*End of proposal.*
