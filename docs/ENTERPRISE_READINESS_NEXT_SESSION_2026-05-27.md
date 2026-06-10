# Kujo Enterprise Readiness - Next Session Backlog (2026-05-27)

## Goal
Continue hardening Kujo into a production-grade, enterprise-friendly showcase runtime and language ecosystem.

## Priority 0 - Release and Trust Boundaries
1. Add signed release artifacts and verification docs.
2. Define a stable LTS support matrix (Rust toolchain, OS targets, feature flags).
3. Add security response policy and CVE handling workflow.

## Priority 1 - Security Hardening
1. Add an explicit CLI flag to force strict outbound policy (`--deny-private-net`) independent of environment variables.
2. Add optional allowlist/denylist host policy file support for HTTP/TCP client surfaces.
3. Add audit-event hooks for sensitive native calls (`execute`, `spawn_process`, outbound network, DB connect).
4. Add redaction policy for logs and machine-readable diagnostics (headers/tokens/password-like values).

## Priority 2 - Performance and Scalability
1. Benchmark and optimize hot native-call paths (HTTP client creation, JSON conversion, map-heavy route handlers).
2. Add persistent benchmark baselines and CI regression thresholds.
3. Expand performance tests for concurrent VM contexts under realistic script workloads.
4. Add startup-time and memory-footprint dashboards for release candidates.

## Priority 3 - Functionality and Platform Reach
1. Add first-class request parsing helpers for HTTP servers (JSON/body parser helpers, typed query extraction).
2. Expand DB reliability contracts (timeouts, cancellation, connection-pool telemetry).
3. Improve module/package ergonomics with enterprise templates and policy presets.
4. Add stable plugin/extension points for ecosystem integrations.

## Priority 4 - Presentation and Developer Experience
1. Publish an "Enterprise Quickstart" guide (secure defaults, deployment modes, policy examples).
2. Create polished end-to-end showcases (internal tools API, workflow automation, secure agent service).
3. Add architecture diagrams and troubleshooting playbooks for VM/interpreter/JIT selection.
4. Strengthen "Why Kujo" positioning with reproducible benchmark and reliability evidence.

## Priority 5 - Test and Quality Gates
1. Add dedicated untrusted-mode integration test suite covering allow/deny capability combinations.
2. Add fuzz targets for URL/path/query parsing and native API arg-shape parsing.
3. Add regression tests for newly introduced request `query_decoded` semantics.
4. Run full release gate + docs parity + stdlib contract checks in a single CI workflow.

## Current Session Outcome Snapshot
1. Added untrusted-mode network destination hardening defaults for CLI `run` and `test-run`.
2. Added decoded query map support in HTTP request objects while preserving existing raw query behavior.
3. Added root hygiene audit script and prepared lockfile tracking for reproducible builds.
4. Updated README and repository hygiene guidance for clearer enterprise presentation.
