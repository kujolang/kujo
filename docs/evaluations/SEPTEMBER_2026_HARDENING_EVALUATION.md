# Kujo September 2026 hardening evaluation

## Executive summary

This evaluation compares Kujo immediately before and after the contiguous
September 13, 2026 documentation and recurring-gate hardening pass. The most
defensible baseline is `1a7b6ef0eacd4918912ef58cfb5ec8533bff9e91`, the release
sign-off commit and direct ancestor of the six-commit pass. CURRENT is
`1395f49ff794707380728df3b80ed363788461c0`. The worktree was clean when CURRENT
was fixed for measurement.

The pass improved the surfaces it actually changed. It did not optimize the
Kujo runtime. No production runtime file, dependency manifest, or lockfile
changed, so runtime-speed claims would be false. The principal measured result
is an 89.5% reduction in the active core-document context: README,
INSTALLATION, and ROADMAP fell from 190,433 to 20,072 bytes and from an
estimated 47,608 to 5,019 Kujo tokens. The current documents still pass the
release-status and security-boundary checks that passed at baseline, while also
exposing the primary installer and meeting explicit per-document context
budgets. Kujo Eval moved from 8/12 checks to 12/12.

Agent-facing repository searches also return much less material. Identical
searches over the three core documents produced 61.0% fewer bytes for
install/package/registry terms, 96.5% fewer bytes for security and capability
terms, and 88.7% fewer bytes for release/version terms. These are measured
tool-result reductions, not claims about provider token billing or end-to-end
agent task completion. Search latency itself was neutral: the 0.929 ms median
difference was smaller than run-to-run noise.

The remaining commits repaired the current ecosystem narrative, restored the
VM/interpreter mismatch inventory, routed recurring fuzzing through the
canonical target script, added a scheduled dependency-advisory gate, and
removed a fixed sleep from a network test. These changes improve maintenance
contracts and the probability of catching drift. The targeted affected tests
passed 97/97 at both revisions after correcting one detached-worktree runner
path. CURRENT additionally passed the full repository test run recorded for
this evaluation.

Runtime measurements were intentionally treated as controls. Because the
runtime inputs are byte-identical, one verified Kujo 1.4.0 debug artifact was
used for both checkouts. Median differences ranged from 0.3% slower to 3.4%
faster on the principal success workloads and 5.3% slower on the failure path;
all were small relative to variance or were produced by the same artifact.
Peak RSS differed by 0.5%. These are neutral or inconclusive, not improvements
or regressions. Scaling remained approximately linear after fixed startup cost.

No statistically or operationally meaningful regression was found in the
tested workloads. The main tradeoff is a small increase in maintenance surface:
three tracked files and ten Rust test lines, plus a scheduled CI job that will
consume runner time. The evaluation cannot demonstrate cleaner build times,
smaller binaries, lower CPU use, fewer dependencies, fewer live LLM calls, or a
higher production-agent completion rate. Those areas either did not change or
were not validly measurable from this pass.

For actual Kujo users, the practical improvement is faster comprehension, not
faster execution: installation, package-management boundaries, release state,
runtime recommendations, and security guidance now occupy far less context and
are less entangled with historical launch material. Maintainers also gain more
explicit recurring checks. The evidence supports calling this a successful
documentation and maintenance hardening pass, but not a runtime optimization.

## Before/after scorecard

| Metric | Baseline | Current | Change | Classification |
| --- | ---: | ---: | ---: | --- |
| Core-doc bytes | 190,433 | 20,072 | -170,361 (-89.5%) | **CLEAR IMPROVEMENT** |
| Core-doc estimated tokens | 47,608 | 5,019 | -42,589 (-89.5%) | **CLEAR IMPROVEMENT** |
| Core-doc words | 22,221 | 2,583 | -19,638 (-88.4%) | **CLEAR IMPROVEMENT** |
| Core-doc lines | 2,830 | 590 | -2,240 (-79.2%) | **CLEAR IMPROVEMENT** |
| Security-query tool output | 28,039 B | 982 B | -27,057 B (-96.5%) | **CLEAR IMPROVEMENT** |
| Release-query tool output | 37,531 B | 4,249 B | -33,282 B (-88.7%) | **CLEAR IMPROVEMENT** |
| Install/package query output | 17,103 B | 6,664 B | -10,439 B (-61.0%) | **CLEAR IMPROVEMENT** |
| Kujo Eval | 8/12 (66.7%) | 12/12 (100%) | +4 checks | **CLEAR IMPROVEMENT** |
| Targeted affected tests | 97/97 | 97/97 | no change | **NEUTRAL** |
| Typical runtime median | 75.070 ms | 75.292 ms | +0.222 ms (+0.3%) | **NEUTRAL** |
| Large runtime median | 1.150 s | 1.130 s | -20.6 ms (-1.8%) | **INCONCLUSIVE** |
| Stress peak RSS mean | 15,636,480 B | 15,552,512 B | -83,968 B (-0.5%) | **NEUTRAL** |
| Direct dependencies | 62 | 62 | 0 | **NEUTRAL** |
| Lockfile packages | 583 | 583 | 0 | **NEUTRAL** |
| Production Rust LOC | 119,982 | 119,982 | 0 | **NEUTRAL** |
| Total tracked bytes | 19,609,792 | 19,439,302 | -170,490 (-0.9%) | **LIKELY IMPROVEMENT** |

Only measured metrics appear in this table. The token counts are deterministic
Kujo estimates for context budgeting, not provider-exact billing counts.

## Evaluation boundary

### CURRENT

- Branch at measurement: `codex/humanize-core-docs`
- SHA: `1395f49ff794707380728df3b80ed363788461c0`
- Commit time: `2026-09-13T21:41:37-04:00`
- Tag: none; `v1.4.0` points to ancestor `266a890`
- Worktree: clean before evaluation artifacts were added

### BASELINE

- SHA: `1a7b6ef0eacd4918912ef58cfb5ec8533bff9e91`
- Commit time: `2026-09-09T11:06:32-04:00`
- Reason: release sign-off immediately before the uninterrupted six-commit
  September 13 series; the series begins directly after this SHA.

No competing boundary is equally persuasive. `37e06cd` is the first changed
commit, not a pre-change revision. `266a890` is the v1.4.0 tag, but choosing it
would mix installation/release publication work into the hardening pass.

## What changed and why it matters

### Onboarding and installation

`37e06cd` rewrote README and INSTALLATION around the shipped 1.4.0 experience.
The previous pages repeated release boundaries, carried verbose platform
instructions inline, and did not expose the current one-line installer in the
README. CURRENT routes readers from a short landing page to focused canonical
references, states the trusted/untrusted boundary, distinguishes Kujo's local
package commands from Kennel, and presents installer, npm, release-archive, and
source-build paths directly.

Expected metric: context bytes/tokens and search output. Measured result: README
fell from 17,302 to 9,021 bytes; INSTALLATION fell from 8,790 to 5,256 bytes; the
installer Eval check changed from fail to pass. Runtime impact: none expected or
observed.

### Roadmap replacement

`0e0db0c` replaced a 2,022-line historical launch ledger with a 137-line current
roadmap and moved release-history claims to the changelog. The old document was
not merely long: broad agent searches surfaced hundreds of historical matches
that competed with current state.

Expected metric: context and tool-result volume. Measured result: ROADMAP fell
from 164,341 bytes and 41,085 estimated tokens to 5,795 bytes and 1,449 tokens,
a 96.5% token reduction for that file. The whole-repository byte reduction is
only 0.9%, correctly showing that this is an active-context improvement rather
than a major distribution-size improvement.

### Ecosystem and package-boundary alignment

`63ff905` corrected current docs to distinguish the built-in deterministic local
manifest/lock workflow from the live Kennel package registry. Contract tests
were updated with the wording. `1395f49` restored the explicit stable-release
marker after that consolidation.

Expected metric: correctness and developer comprehension. Measured result:
current package-boundary and README contract tests pass. The Eval suite retains
the stable-release assertion and adds the primary-installer assertion. No live
developer study was run, so comprehension gain is inferred from the smaller,
more direct contract rather than directly measured.

### VM parity inventory repair

`ca8567c` added explicit output snapshots for charset fixtures, excluded a
live-PostgreSQL probe from the deterministic inventory, refreshed generated
inventory artifacts, and added a contract assertion for that exclusion.

Expected metric: deterministic inventory behavior and lower false drift. The
affected two-test inventory contract passes at CURRENT. The same baseline tests
also pass, so the evaluation demonstrates preserved behavior and a stronger
current assertion, not a numerical reliability-rate improvement.

### Recurring hardening gates

`a153b0c` routes the fuzz workflow through `scripts/fuzz_smoke.sh`, adds a weekly
`cargo audit --deny warnings` workflow, and replaces a fixed 250 ms server sleep
with a bounded read in a network test.

Expected metrics: broader fuzz-target coverage, dependency-advisory detection,
and test latency/flakiness. The source diff verifies the mechanisms and the
87-test native security target passes. Historical CI runs were not available in
this offline evaluation, so reductions in defect escape rate and flake rate are
not demonstrated. The scheduled audit also adds CI time and network dependence.

## Benchmark methodology

The comparison ran on Darwin 25.6.0 x86_64, an Intel i7-9750H with 12 logical
cores and 16 GiB RAM. Rust was 1.96.0, Cargo 1.96.0, and Kujo 1.4.0. Network was
not used. Commands ran from detached worktrees at the fixed SHAs.

Runtime inputs under `src`, `modules`, `config`, `schemas`, `tools`, `install.sh`,
`Cargo.toml`, and `Cargo.lock` were confirmed byte-identical. Reusing one debug
artifact was therefore the fairest control: separately compiled artifacts would
introduce build variance despite identical runtime source. Three warmups
preceded every Hyperfine group. Sample counts were 30 minimal, 30 typical, 20
large, 10 stress, 30 failure, and 10 at each scaling point. Fifty runs measured
search latency. Ten interleaved runs measured stress CPU time and peak RSS with
`/usr/bin/time -lp`.

Workloads:

- Minimal: `kujo --version`, isolating startup/fixed CLI overhead.
- Typical: the canonical string-interpolation example.
- Large: 50,000 deterministic arithmetic loop iterations.
- Stress: 100,000 deterministic iterations.
- Failure: an incomplete binding that must exit 3 with `KUJOPARSE001`.
- Agent-facing: whole-document context estimation and identical `rg` searches
  over README, INSTALLATION, and ROADMAP.
- Scaling: 100, 1,000, 10,000, 50,000, and 100,000 iterations.

Median is the primary latency statistic. Min, max, mean, standard deviation,
raw samples, and p95 where sample size supports it are preserved in the
Hyperfine JSON. p99 is not reported for 10- or 20-sample workloads because it
would collapse to the maximum. No confidence interval or significance test is
used to manufacture precision from same-binary host noise.

## Runtime, CPU, memory, and scaling

| Workload | n | Baseline median | Current median | Delta | Baseline p95 | Current p95 | Result |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| Minimal | 30 | 35.154 ms | 34.938 ms | -0.6% | 45.925 ms | 40.303 ms | Neutral |
| Typical | 30 | 75.070 ms | 75.292 ms | +0.3% | 92.288 ms | 82.098 ms | Neutral |
| Large | 20 | 1.150 s | 1.130 s | -1.8% | 1.345 s | 1.195 s | Inconclusive |
| Stress | 10 | 2.362 s | 2.283 s | -3.4% | not supported | not supported | Inconclusive |
| Failure | 30 | 33.401 ms | 35.181 ms | +5.3% | 36.664 ms | 39.056 ms | Inconclusive |

The failure delta is 1.779 ms and comparable to each group's 2.2–2.3 ms
standard deviation. The large and stress differences reverse in the independent
scaling series, where CURRENT was 1.0% slower at 50,000 and 7.0% slower at
100,000 iterations. Because the artifact is identical, these reversals are
direct evidence of host noise rather than a code regression.

Stress CPU means were 2.340 s user/0.035 s system at baseline and 2.386 s
user/0.033 s system at current. Mean peak RSS was 15,636,480 versus 15,552,512
bytes; ranges overlapped. CPU and memory are neutral.

After roughly 50 ms fixed startup cost, the scaling series grows approximately
linearly with iteration count. No curve change exists because the executed
runtime and workload code are identical.

## Token, context, and tool efficiency

Kujo's deterministic `ai_count_tokens` estimator, with a `gpt-4o` model hint,
estimated 47,608 tokens at baseline and 5,019 at CURRENT for the three core
documents. That saves 42,589 estimated tokens per full-context load.

| Repetitions of a full core-doc load | Baseline tokens | Current tokens | Tokens avoided |
| ---: | ---: | ---: | ---: |
| 1 | 47,608 | 5,019 | 42,589 |
| 100 | 4,760,800 | 501,900 | 4,258,900 |
| 1,000 | 47,608,000 | 5,019,000 | 42,589,000 |
| 10,000 | 476,080,000 | 50,190,000 | 425,890,000 |

These are deterministic context-budget estimates. They are not measured API
usage, cached-token counts, or dollar savings. No authoritative local pricing
configuration was used, so this report makes no cost claim.

Identical repository-search calls retained matches while sharply reducing bytes
returned. This demonstrates lower potential tool-result/context volume. It does
not demonstrate fewer tool calls, fewer reasoning cycles, or unchanged
production-agent task completion, because no live model/provider evaluation was
run. Raw evidence remains separate under `benchmarks/results` rather than being
forced into active context.

## Kujo Eval report

The reproducible suite contains twelve explicit checks:

- four successful runtime workloads;
- two deterministic failure/evidence checks;
- stable-release and trust-boundary retention;
- primary installer discoverability;
- three per-document context budgets.

Baseline passes all six runtime/failure checks and both retained-document
contracts. It fails installer discoverability and the three context budgets:
8/12. CURRENT passes 12/12. The score is Kujo Eval's actual pass/fail outcome;
no synthetic 0–100 category scores were invented.

| Category | Baseline | Current | Change |
| --- | ---: | ---: | ---: |
| Runtime correctness checks | 4/4 | 4/4 | 0 |
| Failure behavior checks | 2/2 | 2/2 | 0 |
| Retained documentation contracts | 2/2 | 2/2 | 0 |
| Installer discoverability | 0/1 | 1/1 | +1 |
| Context-budget checks | 0/3 | 3/3 | +3 |
| Overall | 8/12 | 12/12 | +4 |

The byte budgets are explicit evaluator policy: README under 12,000 bytes,
INSTALLATION under 7,000, and ROADMAP under 10,000. They reward bounded
agent-facing context while the separate content checks prevent a trivial empty
file from scoring well.

## Build, artifact, and dependency footprint

`Cargo.toml` and `Cargo.lock` hashes are identical. Both revisions have 62
direct dependencies and 583 lockfile packages. Production Rust LOC, TODO/FIXME
count, and unwrap/expect count are unchanged. Therefore the pass neither reduced
nor expanded runtime dependency or implementation complexity.

A valid clean-build comparison was not obtained. Parallel release builds hit
the host process-spawn limit, and a later isolated release build was stopped
before completion. Reporting either attempt as a comparative benchmark would
violate the equal-environment rule. Current debug binary size was measured at
99,441,608 bytes, but no independently built baseline artifact exists, so binary
size change is **not demonstrated**. Source and dependency identity strongly
imply no intended build or binary-size effect; implication is not measurement.

## Reliability and determinism

The targeted affected suite passed 97/97 at CURRENT. It also passed 97/97 at
baseline after the verified source-identical binary was placed at the detached
worktree path hardcoded by the inventory test. The initial two baseline failures
were harness path failures, preserved in the evaluation notes and not counted
as product regressions.

CURRENT passed `cargo fmt --check`, `cargo check`, the targeted suite, and the
full `cargo test -- --test-threads=1` run. Both revisions produce exit code 3 and
the same `KUJOPARSE001` evidence for invalid source. The inventory's new live
service exclusion and snapshots improve the determinism contract, but no
repeated historical flake-rate series exists.

## Change-to-result and commit attribution

| Commit | Change | Intended effect | Observed effect |
| --- | --- | --- | --- |
| `37e06cd` | Rewrite README and install guide | Faster onboarding, lower context | README/INSTALL bytes down 45.6%/40.2%; installer Eval check now passes |
| `0e0db0c` | Replace historical roadmap | Remove stale/noisy active context | ROADMAP tokens 41,085 → 1,449; primary cause of total 89.5% context reduction |
| `63ff905` | Align Kennel/package boundary | Correct current ecosystem claims | Current package-boundary tests pass; search results are more current and bounded |
| `ca8567c` | Restore parity inventory and snapshots | Deterministic, representative inventory | Inventory contract passes; external-service probe explicitly excluded |
| `a153b0c` | Canonical fuzz script, weekly audit, bounded network test | Better recurring detection and lower test brittleness | Mechanisms observed; 87/87 security-boundary tests pass; CI detection-rate gain not demonstrated |
| `1395f49` | Restore stable-release marker | Preserve current release state | Stable-release Eval and README contract pass |

The graph moved primarily because `37e06cd` and `0e0db0c` removed duplicated
and historical material from the default entry documents. The tradeoff is that
historical detail now requires following changelog/release-evidence links. That
is appropriate for active context, provided those canonical records remain.

## Regressions and tradeoffs

No statistically or operationally meaningful regressions were identified
within the tested workloads.

Tradeoffs and non-regressions that still matter:

- Three tracked files and ten Rust test lines were added; this is a small
  maintenance increase, not codebase simplification across every dimension.
- The scheduled supply-chain audit adds CI runner time and needs network access.
- Shorter front-door docs require link traversal for history and exhaustive
  detail. Current contract tests verify key links/claims, but this evaluation did
  not crawl every link.
- Failure latency appeared 5.3% worse, but the absolute delta was 1.779 ms, the
  same binary was used, and noise spans the difference. It is inconclusive.
- A 7.0% slower CURRENT result at the largest independent scaling point likewise
  came from the same binary and reversed another stress comparison. It is host
  noise, not evidence of a current-code regression.

## Remaining opportunities

### P0

None found.

### P1

- Add an agent-task corpus that asks installation, security, package, and
  release-state questions against both context sets, then records completion,
  citations, reasoning/action cycles, and provider usage. This would determine
  whether the 89.5% context reduction preserves real task quality.

### P2

- Add a deterministic link checker for the shortened front-door docs.
- Record scheduled fuzz/audit duration and findings so later passes can quantify
  detection value and CI cost.
- Run isolated release builds on a quiet CI host if build and artifact comparisons
  become relevant to a pass that actually changes build inputs.

### P3

- Consider a context ratchet for other frequently loaded maintainer documents,
  but require content-preservation checks before setting budgets.

## Reproduction guide and evidence

Run from the repository root with Kujo Eval checked out as sibling `eval`:

```bash
bash scripts/run_hardening_evaluation.sh
```

Set `KUJO_EVAL_ROOT` if Eval lives elsewhere. Set `KUJO_HARDENING_BIN` to reuse a
verified Kujo 1.4.0 artifact; otherwise the script builds one from the
source-identical baseline runtime inputs. The script creates detached worktrees,
copies the identical workloads into each, runs Kujo Eval, Hyperfine, context and
search measurements, records environment and memory evidence, and removes only
its temporary worktrees.

Canonical artifacts:

- Suite and workloads: `benchmarks/hardening-2026-09/`
- Raw results: `benchmarks/results/hardening-2026-09/`
- Machine summary: `benchmarks/results/hardening-2026-09/evaluation-results.json`
- Public case study: `docs/evaluations/SEPTEMBER_2026_HARDENING_CASE_STUDY.md`

Known limitation: raw Hyperfine measurements are specific to this shared Intel
Mac and debug profile. They are adequate to reject runtime-speed claims for an
unchanged runtime, not to publish absolute Kujo performance claims.

## Final assessment

> If we erase the commit messages and ignore what the hardening work intended to
> accomplish, does the empirical evidence independently demonstrate that CURRENT
> is a better engineered version than BASELINE?

**PARTIALLY.** The evidence independently demonstrates a much smaller and less
noisy default context, preserved critical documentation contracts, improved
installer discoverability, a stronger Eval result, and passing maintenance
tests. It does not demonstrate a faster runtime, lower runtime resource use,
faster builds, smaller binaries, lower provider cost, or better live-agent task
completion. CURRENT is empirically better engineered for documentation and
maintenance-gate usability; broader system-level superiority remains outside
what this pass changed and what this evaluation can prove.
