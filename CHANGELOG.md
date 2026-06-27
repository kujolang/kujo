# CHANGELOG

All notable changes to the Kujo programming language will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added `scripts/enterprise_verify.sh`, `docs/AI_NATIVE_ENTERPRISE_RELEASE_EVIDENCE.md`, and `examples/ai_enterprise_replay_showcase.kujo` to make AI-native release-candidate evidence, strict replay demos, and product-hardening checks repeatable.
- Added `docs/SECURE_AI_SCRIPTING.md`, `docs/SECURITY_RESPONSE.md`, and `docs/AI_NATIVE_PRODUCT_HARDENING_STATUS_2026-06-27.md` to clarify secure AI operation, vulnerability-response expectations, and the current product-hardening boundary.
- Added `tests/ai_replay_hermeticity_contract.rs` to guard strict AI replay against live-socket fallthrough and scan committed cassettes for common credential markers.
- Added `ai_native_helpers` Criterion workloads for AI request hashing, schema validation, vector top-k scoring, and context fitting regression checks.
- Added `docs/ENTERPRISE_AI_NATIVE_POLISH_NEXT_SESSION_2026-06-27.md` to capture the next enterprise/readiness polish backlog after the core AI-native enhancement track.
- Added `ai_request_hash(prompt_or_messages, options)` for deterministic, credential-independent AI request cache/cassette keys without network I/O.
- Added native AI record/replay cassettes for `ai_chat`, `ai_stream_chat`, `ai_embedding`, and `ai_tool_loop` via `KUJO_AI_RECORD`, `KUJO_AI_REPLAY`, `KUJO_AI_REPLAY_MODE`, and per-call `options.cassette`.
- Added AI response envelope metadata (`usage`, `finish_reason`, `tool_calls`, `provider`) and opt-in `options.structured_errors` dictionaries while preserving default string errors.
- Added `json_schema_validate(value, schema)` for pure, bounded validation of JSON-like Kujo values against a documented JSON Schema subset.
- Added native vector math helpers `vec_dot`, `vec_norm`, `vec_normalize`, `vec_cosine`, and `vec_top_k` for finite numeric arrays.
- Added deterministic token estimation and context fitting helpers `ai_count_tokens` and `ai_fit_context` for local AI prompt budgeting without provider tokenizers.
- Added `secret`, `reveal`, and `is_secret` for redacted runtime secret values, including AI `options.api_key` support and cassette/error redaction.
- Added a dedicated `network-ai` / `--allow-ai` capability for high-level AI helpers plus `KUJO_AI_ALLOWED_ENDPOINTS` endpoint allowlist enforcement.
- Added optional `ai_stream_chat(prompt_or_messages, options, on_chunk)` chunk callbacks with replay-backed ordered delivery and `false` cancellation.
- Added pure multimodal AI message builders `ai_text`, `ai_image_url`, and `ai_message`.

### Changed

- Updated README positioning to document the completed core AI-native mechanism set while preserving the pre-tag release-candidate enterprise-readiness boundary.

### Fixed

- Hardened flaky test contracts for AI environment isolation, UDP loopback binding, benchmark timer assertions, VM fixture snapshot normalization, and generated VM inventory side effects, and refreshed the LSP completion contract for the completed AI builtin surface.

## [1.0.0] - 2026-06-19

Release-candidate note: the crate metadata is staged at `1.0.0`, but the final `v1.0.0` tag, crate publication, and binary artifact sign-off remain incomplete until the release artifact checklist has dated evidence.

### Added

- VM-first execution posture for ordinary `kujo run` workflows, with explicit `kujo test --runtime vm|dual|interpreter` strategies for fixture validation and migration work.
- Deterministic `kujo test` fixture outcome summary fields: `passed`, `failed`, `skipped`, `expected_fail`, `runnable`, and `discovered`.
- Machine-readable CLI/runtime diagnostic contracts, diagnostics goldens, and release/readiness contract suites for docs, examples, CLI JSON, security boundaries, generated artifacts, and VM/interpreter parity.
- Standard-library and native helper coverage across filesystem, process, HTTP/network, archive, image, database, datetime, crypto, concurrency, JSON, collection, string, AI helper, and SSG-oriented rendering surfaces.
- Deterministic package workflow support, including `kujo package-install --frozen` and lockfile drift detection.
- LSP capability expansion for diagnostics, completion, rename contracts, semantic tokens, inlay hints, and code lens behavior.
- Static `kujo serve` coverage for MIME policy, range requests, cache validators, request limits, dotfile blocking, traversal defense, symlink handling, and safe default headers.
- Release-candidate gates, generated evidence inventories, and documented artifact publication workflow for final tag-time sign-off.

### Changed

- Release state is now documented as pre-tag `1.0.0` release-candidate readiness: `Cargo.toml` remains at `1.0.0`, while publication and artifact evidence stay explicitly open.
- `kujo test` now reports skipped `test-run` framework fixtures separately from runnable snapshot fixtures, removing ambiguity from summaries such as `Passed 144/144 tests`.
- Default documentation now presents stable, preview, experimental, and deferred surfaces with explicit v1 compatibility boundaries.
- Generated TODO, unsafe, and VM runtime mismatch inventories are treated as source-of-truth release evidence and are contract-tested for freshness.
- JIT remains experimental/opt-in and bounded by unsafe-boundary contracts rather than described as a default release guarantee.

### Fixed

- Reconciled release-state drift across README, roadmap, architecture, scope, release-process, and readiness docs.
- Stabilized `tests/test_stdlib_system.kujo` by removing host-dependent printed argument counts and coarse timing output from snapshot assertions.
- Burned down large VM/interpreter parity gaps and documented current residual mismatch ownership instead of relying on stale zero-count claims.
- Updated standard-library documentation so runtime builtins such as `escape_xml`, `render_markdown`, `render_listing_card`, and `render_layout_native` are represented in generated/reference contracts.
- Fixed docs/example smoke drift by keeping expected-fail examples explicit, reasoned, and mirrored between `examples/README_examples.md` and `tests/docs_examples.rs`.
- Fixed multiple release-gate blockers found during final candidate validation, including formatting drift and generated artifact freshness drift.

### Security

- Added and validated trusted/untrusted runtime capability boundaries for filesystem, process, network/HTTP, archive, image, database, and clock-sensitive APIs.
- Hardened static serving against traversal, encoded traversal, dotfiles, backup/swap files, symlink escape targets, oversized request lines/headers/bodies, unsafe method handling, and ambiguous MIME fallback.
- Added deterministic runtime/resource limits for parser, interpreter, VM, native IO, string literals, collection literals, and call depth.
- Documented remaining executable `unsafe` boundaries as concentrated in JIT runtime paths and kept generated unsafe inventory at `0` unknown classifications.
- Updated `tokio-postgres`/`postgres-protocol` lockfile entries for 2026 RustSec advisories and kept the no-fixed-upgrade `rsa` advisory as an explicit release-gate audit exception.

### Performance

- Added Criterion benchmark coverage for lexer, parser, interpreter, VM, module-resolution, collection/string, and static-server workloads.
- Improved module-resolution loading-stack bookkeeping to avoid avoidable deep-chain scan behavior.
- Added runtime/resource limit defaults and benchmark smoke commands to make performance claims reproducible rather than implicit.
- Continued VM-first and JIT-measurement groundwork while keeping unsupported JIT surfaces outside the v1 guarantee.

### Removed

- Removed ambiguous release-ready wording that implied the final `v1.0.0` tag or artifacts were already published.
- Removed stale current-blocker language from active release-readiness surfaces when newer tests/docs superseded it.
- Removed silent fixture-count ambiguity from `kujo test` summaries; skipped framework fixtures and expected-fail counts are now explicit.
