# Kujo SSG Reproducible Benchmark Suite - Next Steps

Status: future benchmark campaign plan; not v1.0 launch evidence.

## Related Document
- See `docs/BENCHMARK_PUBLICATION_POLICY.md` for the v1.0 distinction between
  launch-safe claims and internal regression signals.
- See `docs/HETZNER_BENCHMARK_SETUP_AND_PRICING.md` for dedicated benchmark host selection, pricing snapshots, and campaign setup guidance.

## Objective
Create a benchmark process that is:
- Reproducible across runs and machines
- Fair across Kujo, Hugo, Pelican, and Astro
- Credible for future marketing claims only after raw artifacts, correctness
  checks, and environment metadata are preserved
- Useful for catching performance regressions over time

## Success Criteria
- You can run one command set and get the same measurement workflow every time.
- Cold and warm metrics are reported separately.
- Results are summarized with median and p90 (not single best run).
- Cross-tool comparisons use equivalent site complexity and outputs.
- Raw logs and environment details are preserved for auditability.

## Phase 1: Lock Benchmark Definitions

### 1) Freeze benchmark scenarios
Define at least three scenarios:
- Small: 1,000 pages
- Medium: 10,000 pages
- Large: 50,000 pages

Keep these fixed per release cycle.

### 2) Freeze content and template complexity
Use deterministic generators with fixed seeds so each run has equivalent:
- Frontmatter density
- Markdown body length
- Link count
- Template placeholders/components

Document all generator parameters in versioned config files.

### 3) Freeze required outputs
For every tool, include the same output classes when possible:
- HTML pages
- Index/list page
- Feed
- Sitemap

If a tool cannot produce one output natively, document the caveat clearly.

## Phase 2: Define Measurement Protocol

### 4) Cold vs warm protocol
- Cold run: remove output artifacts and tool caches before each run.
- Warm run: execute immediately after a cold run with no cleanup.

Store both metrics independently. Never blend them into one number.

### 5) Iteration count and statistics
Per scenario and per tool:
- Run 10 cold iterations
- Run 10 warm iterations

Report:
- median
- p90
- min
- max

Median should be the primary headline metric.

### 6) Environment capture
For each benchmark session, record:
- Date/time
- Machine model
- CPU and RAM
- OS version
- Kujo commit hash / release version
- Tool versions for competitors

Write this metadata into every result artifact.

## Phase 3: Build the Benchmark Harness

### 7) Add benchmark workspace layout
Suggested structure:
- benchmarks/cross-language/datasets/
- benchmarks/cross-language/templates/
- benchmarks/cross-language/runners/
- benchmarks/cross-language/results/
- benchmarks/cross-language/reports/

### 8) Add runner scripts (one per tool)
Each runner should:
- Prepare dataset
- Execute cold and warm loops
- Emit structured JSON result rows
- Append command output logs

Keep command flags explicit and version-controlled.

### 9) Add a result aggregator
Build one script that:
- Reads all JSON rows
- Calculates median/p90/min/max
- Produces CSV and markdown summaries

Use this aggregator as the single source of truth for published numbers.

## Phase 4: Fair Cross-Tool Comparisons

### 10) Fairness rules
- Same machine and power mode
- No background heavy workloads
- Same filesystem location type (local SSD)
- Comparable feature scope enabled
- No hidden precomputation unless documented for all tools

### 11) Comparison dimensions
Track at minimum:
- Total build time
- Pages per second
- Peak memory (if available)
- Output correctness checks passed/failed

Correctness must be validated before accepting performance numbers.

## Phase 5: Publishable Reporting

### 12) Report format
For each scenario, include:
- Table of median/p90 for each tool
- Hardware and software environment block
- Exact commands used
- Raw log links/paths
- Notes on caveats and non-equivalent features

### 13) Marketing-safe claim format
Use this section only after satisfying `docs/BENCHMARK_PUBLICATION_POLICY.md`.

Prefer:
- "In our reproducible benchmark suite, Kujo built 10,000 pages in Xs median cold time on <hardware>."

Avoid:
- "Always Xs"
- "Fastest everywhere"

## Phase 6: CI Regression Protection

### 14) Add performance gates
Create CI jobs for Kujo-only scenarios (1k, 10k, 50k) with thresholds, for example:
- Fail if median cold time regresses by more than 10% vs baseline
- Warn if warm time regresses by more than 15%

Store baseline snapshots in version control.

### 15) Add trend tracking
Persist benchmark history so you can visualize:
- Median trend over commits/releases
- p90 stability
- Regression points tied to commit hashes

## Immediate Action Checklist (Do This Next)
1. Create benchmark dataset generator configs with fixed seeds.
2. Implement Kujo cold/warm runner that writes JSON lines.
3. Implement Hugo/Pelican/Astro runners with equivalent output scope.
4. Build aggregator for median/p90/min/max reporting.
5. Run first full 10k comparison batch and publish raw artifacts.
6. Set initial CI thresholds from median of at least 3 baseline sessions.

## Optional Stretch Goals
- Add 100k-page stress scenario.
- Add asset pipeline scenario (images/CSS bundling) as a separate benchmark track.
- Add benchmark replay script for a specific historical commit range.

## Notes
- Keep "cold" and "warm" metrics separate in every chart and claim.
- Keep raw artifacts immutable once published.
- If benchmark methodology changes, start a new series version and do not mix with old data.
