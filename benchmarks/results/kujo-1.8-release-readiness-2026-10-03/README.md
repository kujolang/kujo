# Kujo 1.8 release-readiness measurements — 2026-10-03

These results compare the exact starting source with the 1.8 candidate. They are
bounded release evidence, not universal performance claims.

## Reproduction identity

- Starting source: `656c03c3c916e5fd25508438b9a6808e88263ba9`
- Final measured candidate source: `df7d140`
- Baseline binary SHA-256:
  `e0773d2536089041f71d25e7a8823d1abc28c6c9b23025bdcb15b3e8de1d2460`
- Final candidate binary SHA-256:
  `ebfced0b6730234860257772d9e0562ba9d6d16ef7405114f130f047395e1cb0`
- Generator-only candidate binary SHA-256:
  `2f9242613c497736f77c20fdb669a18b7c815e489e8a2e8cf5047ee75fd2e3e0`
- Build mode: Cargo `release`, default features, optimized, thin LTO, one
  codegen unit.
- Toolchain: Rust/Cargo 1.96.0, LLVM 22.1.2.
- Host: macOS 26.6.2 (25G83), x86_64; Intel i7-9750H; 16 GiB RAM.
- Runtime campaign: two discarded warmups and eleven measured interleaved
  baseline/candidate rounds per workload. Each process verifies equal output.
- Static campaign: two discarded warmups and twenty-one measured rounds;
  baseline/candidate order alternates each round.

## Retained optimization

Generator resume previously cloned the continuation's full bytecode instruction
and constant vectors even though the continuation already owned the active chunk.
The retained change passes an empty chunk into the one-shot resume path, where it
is intentionally ignored, eliminating that copy without changing continuation
ownership or execution semantics.

| Workload / campaign | Baseline | Candidate | Paired change | Variance note |
| --- | ---: | ---: | ---: | --- |
| Nested generator aliases, generator-only interleaved | 727.596 ms median | 667.238 ms median | -6.7% median | paired p90 -1.5%; max +25.0% outlier |
| Nested generator aliases, final interleaved | 882.286 ms median | 842.200 ms median | -6.8% median | heavily loaded host; paired p90 +26.7% |
| Longitudinal generator workload, 20 runs | 4.512 s mean | 3.609 s mean | -20.0% mean | standard deviation 0.865 s vs 0.917 s; supporting evidence only |

The repeated paired median and independent focused workload justify retaining the
small change. The data does **not** justify a blanket Kujo speedup claim.
`captures_generators_sustained` moved -2.4% in the generator-only campaign but
+5.2% in the final heavily contended campaign, so no broad generator-throughput
claim is made.

## Investigated runtime areas

| Area | Final paired median | Disposition |
| --- | ---: | --- |
| Sustained calls | -0.7% | Inconclusive; no change retained |
| Sustained generator lifecycle | +5.2% | Noisy/inconsistent; no additional change retained |
| Bounded task admission/completion | +0.1% | Within noise; deferred |
| Retained closures | +2.1% | Within host variance; deferred |
| Reusable Promise observers | -0.0% | Within noise; deferred |
| Scheduler/channels | +1.0% | Within noise; deferred |
| Admission rejection | +6.4% | Short-process noise; no change retained |
| Cancellation | +8.9% | Short-process noise; no change retained |

Existing deterministic shallow counters remain the allocation characterization:
generator state/resume/drop, closure/capture cells and task lifecycle facts. The
host supplied no trustworthy allocator-total attribution, so no heap-total claim
is made.

## Static analysis and editor latency

Process-inclusive static results use paired interleaving. Broad min/max and p90
ranges in the raw receipt show a busy shared host; all medians are treated as
characterization rather than improvements.

| Operation | Baseline median | Candidate median | Paired median | Disposition |
| --- | ---: | ---: | ---: | --- |
| Startup | 33.119 ms | 33.402 ms | +0.2% | Within noise |
| 200-use imported project check | 37.236 ms | 35.027 ms | -5.8% | No regression claim |
| LSP diagnostics process | 37.427 ms | 35.529 ms | +1.8% | Within noise |

Five debug-profile latency batches after the LSP invalidation fix recorded:

| Operation | Median batch mean | Range |
| --- | ---: | ---: |
| Full-file analysis | 19.234 ms | 16.807–19.804 ms |
| Repeated uncached completion | 25.351 ms | 23.289–26.651 ms |
| Repeated snapshot completion | 6.136 ms | 5.852–6.435 ms |
| Snapshot diagnostics | 0.173 ms | 0.169–0.178 ms |
| Snapshot hover | 3.841 ms | 3.804–4.192 ms |

Correctness takes priority over latency: open unsaved module source now refreshes
direct and transitive open dependents, and close/deletion returns analysis to disk
state. The associated regression tests are release-blocking.

## Raw evidence

- `generator-candidate-runtime-raw.json` and `generator-candidate-runtime-summary.json`
- `final-runtime-raw.json` and `final-runtime-summary.json`
- `generator-focused-hyperfine.json`
- `static-analysis-interleaved-raw.json`
- `lsp-latency-raw.txt`
- `v1-7-compatibility.txt`

Reproduce the process campaigns with:

```bash
rustc -O scripts/runtime_measurement_release_bench.rs -o /tmp/kujo-runtime-bench
/tmp/kujo-runtime-bench BASELINE CANDIDATE > runtime-raw.json
node scripts/summarize_runtime_measurement_bench.js runtime-raw.json > runtime-summary.json
rustc -O scripts/static_analysis_release_bench.rs -o /tmp/kujo-static-bench
/tmp/kujo-static-bench BASELINE CANDIDATE > static-analysis-raw.json
cargo test --test lsp_latency_guardrails -- --nocapture
```
