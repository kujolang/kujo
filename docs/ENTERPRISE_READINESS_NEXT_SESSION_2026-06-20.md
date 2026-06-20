# Kujo Enterprise Readiness - Next Session Backlog (2026-06-20)

## Goal

Continue moving Kujo from a strong pre-1.0 language/runtime into an enterprise-grade showcase that is robust, broadly useful, and compelling as an entry point into the Kujo ecosystem.

## Current Session Outcome Snapshot

1. Fixed resource-exhaustion and panic edges in generated runtime helpers:
   - `range()` now rejects non-finite numeric bounds, caps generated sequence length, and avoids overflow at integer edges.
   - `substring()`, `repeat()`, `pad_start()`, `pad_end()`, and `truncate()` now reject negative, non-finite, reversed, or oversized inputs where applicable.
   - `random_int()` now rejects non-finite and reversed bounds.
   - `random_id()` now caps generated output length.
2. Fixed generated-artifact freshness contract drift so date-only regeneration does not fail unchanged generated reports.
3. Fixed standard-library presentation drift by documenting SSG-native helper registrations and making the system fixture deterministic across VM and dual runtime sweeps.
4. Updated README positioning to mention bounded generated helper behavior and linked this backlog.
5. Removed ignored local root clutter (`.DS_Store`, `blocked.txt`) from the working directory.

## Priority 0 - Release Trust And Verification

1. Add signed release artifacts and verification instructions for every supported binary target.
2. Define a stable support matrix covering Rust toolchain version, OS targets, CPU architectures, and feature flags.
3. Add a public security response policy with vulnerability reporting, embargo, CVE, and patch-release handling.
4. Keep `cargo test`, generated-artifact freshness, VM/dual fixture sweeps, and docs contracts green in CI after every readiness change.

## Priority 1 - Security Hardening

1. Add a CLI flag for strict outbound policy (`--deny-private-net`) that does not depend on environment variables.
2. Add optional host allowlist/denylist policy files for HTTP and TCP client surfaces.
3. Add audit-event hooks for sensitive native calls such as process execution, outbound network, filesystem mutation, and database connection creation.
4. Add redaction rules for machine-readable diagnostics and logs that may contain headers, tokens, passwords, or connection strings.
5. Review SSG and Markdown-rendering helpers for HTML escaping guarantees and document any intentionally unsafe/raw rendering surfaces.

## Priority 2 - Performance And Scalability

1. Add persistent benchmark baselines with CI regression thresholds for startup time, import-heavy execution, and native helper hot paths.
2. Benchmark the new generated-output guards under normal and near-limit workloads to confirm no meaningful overhead on common paths.
3. Expand concurrent VM-context performance tests with realistic automation scripts rather than only synthetic microbenchmarks.
4. Add memory-footprint tracking for range-heavy, string-heavy, JSON-heavy, and HTTP-heavy scripts.

## Priority 3 - Functionality And Universal Usefulness

1. Add first-class HTTP request parsing helpers for JSON bodies, typed query extraction, and validated form data.
2. Expand database reliability contracts with connection timeouts, cancellation behavior, pool telemetry, and deterministic error shapes.
3. Improve package/project templates with enterprise policy presets for untrusted execution, network policy, logging, and deterministic lockfiles.
4. Add stable extension/plugin points for editor adapters, doctor profiles, docgen adapters, and workflow packs.

## Priority 4 - Presentation And Developer Experience

1. Publish an enterprise quickstart with secure defaults, policy examples, deployment modes, and CI verification commands.
2. Add polished end-to-end showcases:
   - internal tools API,
   - workflow automation,
   - secure local agent service,
   - static content pipeline.
3. Add diagrams for VM/interpreter/JIT selection, native capability flow, package workflow, and LSP/editor integration.
4. Add a short README section that answers "Is Kujo production ready?" with the current pre-1.0 boundary, supported use cases, and remaining blockers.

## Priority 5 - Test And Quality Gates

1. Add an untrusted-mode integration matrix covering all capability allow/deny combinations.
2. Add fuzz targets for URL, path, query, range, generated-string, native-argument, and archive-entry parsing.
3. Add regression fixtures for the resource-boundary fixes from this session at CLI level, not only unit level.
4. Add a single release-readiness gate command that runs the core verification matrix and emits a compact machine-readable report.

## Next Starting Point

1. Run the full validation matrix on the latest branch.
2. Add CLI-level regression tests for the generated helper guards.
3. Review Markdown/SSG rendering security posture and decide whether raw HTML is a supported feature, an opt-in mode, or a bug to close.
4. Start the signed-release and support-matrix work before expanding feature scope.
