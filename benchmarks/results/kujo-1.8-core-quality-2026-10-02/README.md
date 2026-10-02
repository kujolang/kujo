# Kujo 1.8 core-quality measurements — 2026-10-02

These are process-inclusive comparisons of the exact starting source and the
core-quality candidate. They are characterization evidence, not universal
performance claims.

## Environment and reproduction

- Baseline source: `531cc223859977f1c203961e166782ea58108786`
- Candidate code source: `12b1a6438c8a996cf4594f4d412809c3ac317394`
- Baseline binary SHA-256:
  `3c6df66c72c4ffc1755d6827ee5e68466ea6a6464b00a2d7639db1007fef3587`
- Candidate binary SHA-256:
  `fb14bce6bbd71c5e0d9e8bb508e4a372e8087dca3823c66fb7127fff91abbda8`
- Build mode: Cargo default `release` profile, default features, optimized,
  thin LTO and one codegen unit as configured by the repository.
- Toolchain: `rustc 1.96.0 (ac68faa20 2026-05-25)`, LLVM 22.1.2;
  `cargo 1.96.0 (30a34c682 2026-05-25)`.
- Platform: macOS Darwin 25.6.0, x86_64; Intel Core i7-9750H 2.60 GHz;
  16 GiB RAM.
- Runtime method: 2 discarded warmups and 11 measured samples per mode,
  interleaving exact baseline, candidate without measurements and candidate with
  measurements. Each sample starts a new process and verifies identical
  stdout/stderr across modes.
- Static-analysis method: 2 discarded warmups and 21 measured samples per
  binary. These are process-inclusive startup, a 200-use imported-module check,
  and LSP diagnostics over the same source.

Reproduce after building immutable binaries:

```bash
rustc -O scripts/runtime_measurement_release_bench.rs -o /tmp/kujo-release-bench
/tmp/kujo-release-bench BASELINE CANDIDATE > runtime-raw.json
node scripts/summarize_runtime_measurement_bench.js runtime-raw.json > runtime-summary.json
rustc -O scripts/static_analysis_release_bench.rs -o /tmp/kujo-static-bench
/tmp/kujo-static-bench BASELINE CANDIDATE > static-analysis-raw.json
```

## Runtime results

Times are median wall milliseconds. The final column is the median same-round
candidate/baseline delta, which is preferred over comparing two independently
sorted medians on this shared host.

| Area / workload | Baseline ms | Candidate ms | Paired change |
| --- | ---: | ---: | ---: |
| Generator creation/suspend/resume/termination (`captures_generators_sustained`) | 2236.252 | 2244.145 | +0.4% |
| Retained closure creation/captures (`closures_retained`) | 121.599 | 120.428 | +0.3% |
| Nested generator aliases (`generator_nested_alias`) | 1215.895 | 1220.067 | +0.4% |
| Bounded task admission/completion (`tasks_sustained`) | 1573.989 | 1552.204 | -0.2% |
| Reusable Promise observers (`promise_observers`) | 1580.211 | 1601.665 | +0.6% |
| Scheduler/channel workload (`scheduler_channels`) | 1613.035 | 1591.823 | -0.3% |
| Rejected admission (`task_admission_rejection`) | 34.583 | 34.920 | -1.5% |
| Task cancellation (`task_cancellation`) | 38.202 | 38.238 | -2.1% |

No runtime optimization was retained: all long-running paired medians stayed
within 0.6%, so the data does not support a meaningful runtime-speed claim.
Measurement collection added 1.0–2.4% to the long-running task/generator
workloads. Its larger percentage on short processes reflects fixed report/setup
cost. Full min/median/p90/max and paired distributions are in
`runtime-summary.json`; all samples and reports are in `runtime-raw.json`.

## Allocation and lifecycle characterization

The counters are deterministic shallow runtime facts from one representative
measured sample; they are not allocator-total attribution.

| Workload | States / resumes / drops | Shallow state bytes | Closures / capture cells | Shallow capture bytes | Task facts |
| --- | ---: | ---: | ---: | ---: | --- |
| Captures + generators, 10k | 10,000 / 30,000 / 10,000 | 6,000,000 | 10,002 / 10,000 | 4,000,000 | — |
| Retained closures, 250 | 0 / 0 / 0 | 0 | 251 / 250 | 100,000 | — |
| Nested generator aliases, 5k | 10,000 / 30,000 / 10,000 | 6,000,000 | 2 / 0 | 0 | — |
| Sustained tasks, 1k | — | — | 1 / 0 | 0 | 1,000 admitted/started/completed; 1,001 scheduler rounds; 2,001 polls |
| Promise observers, 1k | — | — | 1 / 0 | 0 | 1,000 completions; 2,001 polls |
| Admission rejection | — | — | 1 / 0 | 0 | 16 admitted; 1 rejected; 16 completed |
| Cancellation | — | — | 1 / 0 | 0 | 40 admitted/started; 40 cancellations |

The runtime report explicitly marks total heap bytes, total value
allocations/drops and retained capture-graph bytes unsupported. No allocator-level
precision is inferred from shallow counters.

## Static-analysis results

| Workload | Baseline median ms | Candidate median ms | Change | Baseline → candidate p90 ms |
| --- | ---: | ---: | ---: | ---: |
| Startup (`--version`) | 30.227 | 31.025 | +2.6% | 31.104 → 32.275 |
| Imported project check | 33.981 | 34.094 | +0.3% | 35.388 → 35.152 |
| LSP diagnostics | 33.014 | 33.677 | +2.0% | 34.822 → 35.218 |

The richer project check showed no material latency regression. LSP diagnostics
does not consume the optional checker model today, so its small observed delta is
startup/host variance rather than inference work.

## Retained optimization

| Area | Uncached enriched analysis | Retained cache | Change |
| --- | ---: | ---: | ---: |
| Imported-module parse passes for function/value/struct export lookup | 3 | 1 | -66.7% |

This exact operation count is enforced by the type-checker regression. The cache
shares one immutable parsed AST without changing module resolution or runtime
behavior. No other attempted optimization showed material evidence, so no extra
runtime/compiler complexity was carried.
