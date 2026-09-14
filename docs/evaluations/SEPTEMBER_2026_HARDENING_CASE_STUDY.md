# Hardening Kujo's front door

## Why we did it

Kujo 1.4.0's front-door documentation still carried much of the project's
launch history. That material was useful evidence, but it competed with current
installation, package, security, and runtime guidance in the files developers
and agents read first. The same pass also tightened recurring parity, fuzz, and
supply-chain checks.

## What changed

Across six commits on September 13, 2026, Kujo:

- rewrote README and INSTALLATION around the current 1.4.0 experience;
- replaced a 2,022-line historical roadmap with a 137-line current roadmap;
- clarified the boundary between Kujo's local package workflow and Kennel;
- restored deterministic VM/interpreter inventory fixtures;
- routed fuzz smoke coverage through the canonical target script;
- added a weekly dependency-advisory gate; and
- removed a fixed sleep from a network test.

No production runtime source or dependency changed.

## How we measured it

We compared release sign-off commit `1a7b6ef` with post-pass commit `1395f49`
using detached worktrees and identical inputs. Kujo Eval checked runtime success,
deterministic failure evidence, retained documentation contracts, installer
discoverability, and explicit document-size budgets. Hyperfine ran 10–30 samples
per runtime workload after three warmups. Kujo's deterministic token estimator
measured the three core documents, and identical repository searches measured
agent-facing tool output.

## Before vs after

| Metric | Before | After | Change |
| --- | ---: | ---: | ---: |
| Core-doc bytes | 190,433 | 20,072 | -89.5% |
| Estimated context tokens | 47,608 | 5,019 | -89.5% |
| Core-doc words | 22,221 | 2,583 | -88.4% |
| Security-search output | 28,039 B | 982 B | -96.5% |
| Release-search output | 37,531 B | 4,249 B | -88.7% |
| Kujo Eval | 8/12 | 12/12 | +4 checks |
| Typical runtime median | 75.070 ms | 75.292 ms | neutral |
| Direct dependencies | 62 | 62 | unchanged |

Token counts are deterministic budgeting estimates, not provider billing data.

## Biggest improvements

The roadmap produced the largest movement: 41,085 estimated tokens became
1,449. The shorter documents still identify the current stable release and the
fact that Kujo is not a sandbox. CURRENT also makes the primary installer
discoverable from README, which BASELINE did not.

For agent tools, identical searches now return far less historical noise. A
security/capability search returned 96.5% fewer bytes, while a release/version
search returned 88.7% fewer. Search execution was not meaningfully faster—the
files were already small enough for process startup to dominate—but the result
passed into context is substantially smaller.

## What surprised us

The repository itself shrank by only 0.9%. The 89.5% gain appears when measuring
the documents most likely to enter active developer or agent context. This is a
useful reminder that distribution size and context efficiency are different
metrics.

Runtime samples moved in both directions despite using the same source-identical
binary. That exposed normal host variance and prevented us from mislabeling
small timing differences as performance improvements.

## What did not improve

This was not a runtime optimization. Execution speed, CPU, peak memory,
dependencies, and production code complexity were neutral. Build time and
binary-size change were not validly demonstrated. No live model run measured
tool-call count, reasoning cycles, provider tokens, dollars, or task completion.

## What remains

The most valuable next evaluation is a provider-recorded agent corpus that asks
real installation, package, security, and release questions against both context
sets. It should verify answer quality and citations while recording tool calls,
cycles, latency, and actual provider usage. A deterministic link checker would
also protect the shortened documents' reliance on canonical references.

## Reproducing the results

With Kujo Eval available as a sibling checkout:

```bash
bash scripts/run_hardening_evaluation.sh
```

The detailed methodology, limitations, raw Hyperfine samples, Eval artifacts,
and machine-readable results live under `docs/evaluations/` and
`benchmarks/results/hardening-2026-09/`.
