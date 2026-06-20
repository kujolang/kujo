# Kujo Field Notes - v1.0 Benchmark Publication Strategy

Date: 2026-06-20
Checklist item: `docs/V1_0_RELEASE_READINESS_GAP_CHECKLIST.md` `V1RR-P2-003`
Status: completed

## Decision

Kujo v1.0 launch benchmark claims are limited to committed import-heavy
module-resolution artifacts:

- `docs/PERF_HOT_PATH_AUDIT_2026-05-26.md`
- `docs/generated/VM_IMPORT_HEAVY_PERF_COMPARISON.md`
- `docs/generated/VM_IMPORT_HEAVY_CACHE_LOOKUP.md`

These artifacts support only narrow claims about the named module-resolution
workloads. They do not support broad VM/JIT speedup ranges, cross-language
"faster than" claims, or SSG throughput comparisons.

SSG, cross-language, JIT example, `kujo bench`, host/pricing, and historical
field-note benchmark material are internal regression or exploratory signals
until a fresh reproducible benchmark campaign preserves raw logs, commands,
correctness checks, environment metadata, and repeated-run statistics.

## Changes

- Added `docs/BENCHMARK_PUBLICATION_POLICY.md` as the maintained benchmark
  claim boundary for v1.0.
- Updated `docs/PERFORMANCE.md` to link the policy and keep launch-safe evidence
  limited to committed artifacts.
- Marked `docs/SSG_BENCHMARK_NEXT_STEPS.md` as future campaign planning, not
  launch evidence.
- Marked `docs/HETZNER_BENCHMARK_SETUP_AND_PRICING.md` as future host planning
  with historical pricing snapshots that must be revalidated before
  publication.
- Downgraded example benchmark docs, JIT examples, cross-language helper text,
  and historical SSG docs from fixed speedup promises to local signals or
  historical context.
- Added `tests/benchmark_publication_policy_contract.rs` so launch/evidence
  boundaries and stale-claim removals remain guarded.

## Validation

Command logs and exit codes are recorded in
`notes/release_evidence/2026-06-20_p2-003/status.tsv`.
