# Kujo Enterprise AI-Native Polish - Historical Backlog (2026-06-27)

Status: superseded by `docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md`.

This document is retained as historical planning evidence. The current release path is the canonical checklist in `docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md`.

## Goal

Move Kujo from a strong AI-native release-candidate runtime into a more broadly enterprise-ready, highly polished language showcase. The next work should preserve the mechanism-first core boundary while improving performance, security, functionality, presentation, and evidence quality.

## Current Readiness Answer

Kujo now has the core AI-native mechanisms listed in the enhancement proposal: deterministic AI request hashing, offline AI record/replay, structured response metadata, schema validation, vector math, token budgeting, redacted secrets, AI egress controls, streaming callbacks, and multimodal message builders.

That does not by itself make Kujo universally enterprise-ready. The current posture is: strong pre-tag `1.0.0` release candidate with substantial security and determinism foundations, but still needing final release evidence, support-matrix clarity, operator-facing polish, stronger performance baselines, and showcase-quality end-to-end examples before claiming universal enterprise readiness.

## Session Review Snapshot

1. Root hygiene is intentionally minimal for tracked files, and the audit now catches ignored root clutter such as `blocked.txt` and `.DS_Store`.
2. README now names the implemented AI-native primitives directly and preserves an honest readiness boundary.
3. The repo still has many active readiness/backlog docs. Future sessions should consolidate current-facing status where possible so new users see one crisp path instead of multiple historical checklists.
4. The current codebase is VM-first, with interpreter fallback and experimental JIT posture clearly documented. That remains the right default presentation for a language showcase.
5. The next showcase value is less about adding broad AI policy to core and more about making the existing mechanisms obvious, easy to verify, and delightful to use.

## Priority 0 - Release Trust And Enterprise Evidence

1. Run and publish a fresh full verification matrix on the current AI-native branch:
   - `cargo fmt --check`
   - `cargo check`
   - `cargo test`
   - `cargo test --test docs_examples`
   - `cargo test --test readme_contracts`
   - `cargo test --test cli_contracts`
   - `cargo test --test cli_json_contracts`
   - `cargo test --test diagnostics_golden`
   - `cargo run -- test --runtime vm`
   - `cargo run -- test --runtime dual`
   - `bash scripts/release_gate.sh`
2. Add a concise release-evidence index that points to the latest passing verification commands, generated inventories, and release-artifact readiness.
3. Reduce or archive historical readiness docs that are superseded by current generated evidence and the roadmap.
4. Add CI guidance for the AI replay suite so no AI test can accidentally use a live socket.

## Priority 1 - Security Hardening

1. Add a replay-hermeticity regression that fails if `ai_*` replay tests attempt network access.
2. Extend redaction tests to cover nested `options.body`, headers, replay cassettes, structured error dictionaries, and multimodal messages.
3. Add an operator-facing "secure AI script" guide covering `--untrusted`, `--allow-ai`, `KUJO_AI_ALLOWED_ENDPOINTS`, `secret`, replay cassettes, and private-network denial.
4. Review static HTML/Markdown/SSG rendering surfaces and make raw HTML behavior explicit as safe-by-default, opt-in raw, or unsupported.
5. Add a security-response file or docs page covering vulnerability reporting, embargo handling, supported versions, and patch-release expectations.

## Priority 2 - Performance And Scalability

1. Add reproducible benchmarks for the new AI-native pure helpers:
   - `ai_request_hash` with large message arrays,
   - `json_schema_validate` with nested objects and bounded failures,
   - `vec_top_k` with realistic embedding counts,
   - `ai_fit_context` with large prompt corpora.
2. Add regression thresholds for VM startup, import-heavy execution, parser throughput, and hot native helper surfaces.
3. Audit allocation-heavy paths in AI message normalization and schema validation for avoidable clones while preserving behavior.
4. Add memory-footprint checks for large arrays/dictionaries, generated strings, JSON parsing, and replay cassette loading.

## Priority 3 - Functionality And Universal Usefulness

1. Add a polished "local AI tool" example that combines secrets, replay, schema validation, token budgeting, streaming, and multimodal messages without any live network dependency.
2. Add a small enterprise template under examples or tools that demonstrates:
   - capability-minimal execution,
   - deterministic config loading,
   - replay-backed AI calls,
   - structured JSON output,
   - clear failure modes.
3. Improve package/project templates with policy presets for untrusted execution, AI egress, local replay fixtures, and deterministic lockfiles.
4. Add first-class helpers or cookbook patterns for typed JSON request parsing and validated CLI argument maps.
5. Rank and repair the most visible expected-fail examples so new users and agents learn from current syntax first.

## Priority 4 - Presentation And Developer Experience

1. Create a first-10-minutes README path:
   - install,
   - run hello,
   - run an AI replay example,
   - run an untrusted secure example,
   - run tests.
2. Add a "Production readiness" section that cleanly separates:
   - good for local automation and controlled services,
   - still release-candidate until final artifact evidence,
   - not a sandbox,
   - experimental JIT and deferred surfaces.
3. Add diagrams for:
   - VM/interpreter/JIT selection,
   - native capability enforcement,
   - AI replay flow,
   - package install and lockfile verification.
4. Consolidate high-signal docs into the README path and move old checklist-only context to clearly historical docs.
5. Add screenshots or terminal transcripts for the strongest examples so the repo reads as a finished product, not only a test corpus.

## Priority 5 - Quality Gates

1. Add README contract coverage for AI-native positioning, production-readiness wording, and secure AI quickstart links.
2. Add docs contract coverage for this backlog and future enterprise readiness docs so dates and readiness boundaries do not drift.
3. Add exact-output tests for high-visibility human renderers where only fragment tests exist today.
4. Add a single compact "enterprise verify" command or script that wraps the key release, security, AI replay, and docs checks.

## Non-Goals For Core

Do not add provider routing, retry policy, RAG pipelines, agent orchestration, MCP servers, eval systems, observability dashboards, or public registry behavior to core unless a new approved proposal changes scope. Keep core focused on deterministic, secure, reusable mechanisms that ecosystem packages can compose.

## Next Starting Point

1. Run the full verification matrix on `feat/core-ai-native`.
2. Build the replay-only AI showcase example and docs around it.
3. Add performance benchmarks for the new pure AI helpers.
4. Consolidate readiness presentation so README, roadmap, and current enterprise docs tell one story.
5. Prepare the branch for push/PR review with clean evidence and no root clutter.
