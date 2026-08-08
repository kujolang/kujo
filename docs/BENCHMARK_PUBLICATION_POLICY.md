# Benchmark Publication Policy

Date: 2026-06-20
Status: v1.0 release-readiness benchmark claim policy
Owner: Kujo core/release maintainers

## Purpose

This policy separates benchmark evidence that is safe to cite during the Kujo
v1.0 launch from benchmark material that is useful only for local regression
work, historical analysis, or future publication campaigns.

## V1.0 Launch-Safe Claims

Only the following benchmark evidence is launch-safe for v1.0:

- `docs/PERF_HOT_PATH_AUDIT_2026-05-26.md`
- `docs/generated/VM_IMPORT_HEAVY_PERF_COMPARISON.md`
- `docs/generated/VM_IMPORT_HEAVY_CACHE_LOOKUP.md`

These artifacts support narrow claims about the named import-heavy
module-resolution workloads only. They do not support broad claims that Kujo is
always faster than another language, faster for all programs, or faster by a
fixed multiplier outside the recorded workload.

Launch-safe wording should stay close to:

- "The committed import-heavy module-resolution benchmark artifact reports
  median startup improvement for the named workload."
- "The committed warm-cache artifact reports cached nested import lookup
  behavior for the named workload."

## Internal Regression Signals

The following benchmark surfaces are internal regression or exploratory signals
for v1.0, not launch claims:

- `kujo bench ...` ad hoc script runs.
- `examples/benchmarks/**` micro-benchmarks, JIT examples, and cross-language
  comparison helpers.
- `examples/ssg/**` historical static-site-generator benchmark notes.
- `docs/SSG_BENCHMARK_NEXT_STEPS.md` future campaign planning.
- `docs/HETZNER_BENCHMARK_SETUP_AND_PRICING.md` future host planning and
  historical pricing snapshots.
- Historical `notes/**` benchmark, JIT, SSG, or cross-language timing notes.

Those surfaces can guide optimization and regression work, but they must not be
quoted as v1.0 public performance promises without a fresh publication campaign.

## Curated Cross-Language Inputs

The built-in `bench-cross` and `bench-ssg` commands retain four reviewed source
inputs under `benchmarks/cross-language/`; their README defines the supported
commands and prerequisites. The former ad hoc runners, unrelated workloads, and
hand-recorded result files were removed before v1.0 because they did not meet
the publication requirements below. Do not restore or cite those artifacts.

A future cross-language campaign should start from
`docs/SSG_BENCHMARK_NEXT_STEPS.md`, use correctness-equivalent workloads, and
commit only reviewed methodology plus reproducible source inputs. Raw run output
belongs in ignored local results or immutable release attachments.

## Publication Requirements

Before publishing SSG, cross-language, JIT, or broad runtime performance claims,
create a reproducible benchmark campaign with:

1. Pinned Kujo commit or release version.
2. Pinned hardware, OS, power mode, storage type, and tool versions.
3. Versioned datasets, templates, and commands.
4. Separate cold and warm runs.
5. Median, p90, min, and max from repeated measured runs.
6. Correctness checks for each compared tool before accepting timing numbers.
7. Raw logs, structured results, checksums, and environment metadata preserved
   as committed or release-attached artifacts.

If the methodology changes, start a new benchmark series and do not mix old and
new results in one headline claim.

## Forbidden Without Fresh Evidence

Do not publish these as v1.0 claims unless a fresh campaign above supports
them:

- fixed VM/JIT speedup ranges for arbitrary programs;
- "faster than Python", "competitive with Node", "near Go", or similar broad
  cross-language claims;
- SSG throughput comparisons against Hugo, Jekyll, Gatsby, Stattic, or any
  other tool;
- cloud-provider pricing or instance recommendations without revalidating the
  provider pages at publication time.

## Validation

This policy is guarded by `tests/benchmark_publication_policy_contract.rs`.
