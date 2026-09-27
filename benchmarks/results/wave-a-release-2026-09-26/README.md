# Optimized runtime measurement campaign — 2026-09-26

This is a local optimized-build observation, not a universal instrumentation budget or release claim. Raw samples include every retained timing, discarded warmups, exact workload source and all 126 enabled reports. All 414 process executions matched stdout, stderr and exit status across modes.

## Method and provenance

`cd6d2ea93c72b1e2d09a391d853316204e095977` is the immutable pre-measurement baseline. Candidate runtime source is `5d72aab4b99e7f8c01e4c208d6c97061934c7447`; this task changes no runtime source. Both use identical Cargo.lock (SHA-256 in build-provenance.json), default features and release profile: opt-level 3, thin LTO, one codegen unit, stripped symbols. Rust 1.96.0 / LLVM 22.1.2, same Intel i7-9750H / macOS 26.6.2 host. Shared dependency compilation cache; separately frozen binaries. See host.txt, build-environment.json, build-provenance.json and binary-sha256.txt.

Two warmup triplets precede 21 measured triplets per workload. Mode order rotates `(round + offset) % 3`; same-round samples form pairs. Median is the central sorted sample; p90 is nearest rank. The first three inputs match the earlier dev campaign; the last three increase iterations. All runs use the same Rust harness, inputs and process capture. Times include startup, parser/compiler, execution, teardown, and (when enabled) exclusive output creation, export and file sync. CPU affinity/frequency and other desktop services were not controlled. Build/test jobs were stopped and the host settled for about 57 seconds before sampling. See conditions.txt.

## Process-inclusive distributions

All times below are milliseconds. No negative paired delta is presented as a speedup.

| Workload / iterations | Mode | Min | Median | p90 | Max |
| --- | --- | ---: | ---: | ---: | ---: |
| calls / 10000 | baseline | 117.551 | 130.063 | 144.642 | 151.411 |
| calls / 10000 | disabled | 113.927 | 124.061 | 143.589 | 166.085 |
| calls / 10000 | enabled | 136.876 | 147.327 | 155.688 | 170.250 |
| captures_generators / 1000 | baseline | 724.848 | 833.033 | 1246.124 | 1260.724 |
| captures_generators / 1000 | disabled | 722.350 | 855.092 | 1237.431 | 1248.030 |
| captures_generators / 1000 | enabled | 748.039 | 820.921 | 1272.841 | 1284.172 |
| tasks / 100 | baseline | 238.101 | 262.697 | 267.004 | 278.557 |
| tasks / 100 | disabled | 252.392 | 262.951 | 275.190 | 281.222 |
| tasks / 100 | enabled | 271.190 | 282.860 | 298.131 | 574.911 |
| calls_sustained / 200000 | baseline | 2331.709 | 2523.367 | 2724.312 | 2777.714 |
| calls_sustained / 200000 | disabled | 2320.188 | 2543.195 | 2717.106 | 2763.280 |
| calls_sustained / 200000 | enabled | 2337.895 | 2545.350 | 2733.556 | 2756.996 |
| captures_generators_sustained / 10000 | baseline | 9098.334 | 9715.655 | 10516.025 | 10919.479 |
| captures_generators_sustained / 10000 | disabled | 9007.964 | 9736.327 | 10585.556 | 11067.830 |
| captures_generators_sustained / 10000 | enabled | 8679.210 | 9744.393 | 10187.258 | 11621.090 |
| tasks_sustained / 1000 | baseline | 1934.871 | 1994.816 | 2039.979 | 2121.299 |
| tasks_sustained / 1000 | disabled | 1949.734 | 1991.962 | 2012.024 | 2032.123 |
| tasks_sustained / 1000 | enabled | 1965.948 | 2016.444 | 2043.454 | 2048.943 |

## Same-round paired median changes

Ratios are computed per pair before taking the median; they are **not** ratios of the marginal medians above. Spread/drift means these can differ in sign from ratios of marginal medians. Full delta distributions (min, median, p90, max in both ms and percent) are in release-summary.json.

| Workload | Disabled − baseline (ms) | Disabled / baseline (%) | Enabled − disabled (ms) | Enabled / disabled (%) |
| --- | ---: | ---: | ---: | ---: |
| calls | -5.342 | -4.22% | +21.038 | +16.95% |
| captures_generators | -7.374 | -0.89% | +24.063 | +2.86% |
| tasks | -0.013 | -0.00% | +22.888 | +8.95% |
| calls_sustained | +3.069 | +0.11% | +16.906 | +0.66% |
| captures_generators_sustained | +13.966 | +0.14% | +86.641 | +0.92% |
| tasks_sustained | -1.039 | -0.05% | +28.364 | +1.43% |

Disabled paired medians ranged from −4.22% to +0.14%; these samples do not establish a speedup or a universal disabled-overhead bound. Enabled short-call and short-async overhead is material here (+16.95% and +8.95%). The longer workloads show +0.66%, +0.92%, and +1.43% paired median enabled changes. The short capture workload has substantial spread; one short-async enabled sample is 575 ms. Do not remove outliers or generalize these results to service lifetimes, contended parallel tasks, other platforms, or JIT-heavy workloads.

## Focused component probes

The standalone optimized probe links the actual candidate collector for snapshot collection, and uses the actual serde/File APIs for export. The uncontended atomic probe reproduces the collector saturating relaxed CAS; it does not include its disabled OnceLock branch. The timer probe includes `Instant::now` plus elapsed. Each component has two discarded warmup samples and 21 retained samples. Loop samples average 1,000,000 atomic/empty operations, 100,000 clock pairs, or 1,000 snapshot/serialization operations; file samples time one operation, excluding cleanup. Components run sequentially, not as paired macro attribution. Raw values are ns/operation in component-costs.json.

| Component | Median ns/operation | p90 ns/operation |
| --- | ---: | ---: |
| actual_snapshot_including_process_usage | 12738.109 | 14197.260 |
| empty_black_box | 0.334 | 0.336 |
| exclusive_create | 64264.000 | 97997.000 |
| monotonic_now_elapsed | 84.129 | 86.980 |
| saturating_atomic_increment | 9.477 | 10.597 |
| serialize_report_to_vec | 1912.584 | 2049.520 |
| unbuffered_json_file | 1625446.000 | 1759158.000 |
| unbuffered_json_file_sync | 22280234.000 | 23322862.000 |

The measured fixed export boundary is the dominant component: unbuffered JSON file export plus `sync_all` took about **22.28 ms median**, versus **1.63 ms** without sync. Exclusive creation was about 64 µs, actual snapshot including process usage and report construction about 12.7 µs, and serialization to a Vec about 1.9 µs. The process-usage syscall is not separately isolated from snapshot construction. This supports `src/main.rs::finish_runtime_measurements` / synced export as the main observed fixed-cost boundary for short runs; it is not an exact additive decomposition of whole-program overhead.

Recommended maintenance experiment: serialize into a bounded buffer, then `write_all` and retain `sync_all`, exclusive creation and file permissions. That may reduce the roughly 1.6 ms unbuffered-write component, but cannot remove the measured sync cost. Amortize measurement over meaningful execution boundaries rather than exporting per function or tiny step. Keep measurement opt-in. Any optional deferred-durability mode would require a separate explicit contract review; this task does not add one or weaken durability. Do not remove counters merely to improve the chart.

## Reproduction

Build and freeze both binaries with the commands/profile in build-provenance.json. Then:

```sh
rustc -O scripts/runtime_measurement_release_bench.rs -o /tmp/kujo-release-bench
/tmp/kujo-release-bench BASELINE_BINARY CANDIDATE_BINARY > release-overhead.json
node scripts/summarize_runtime_measurement_bench.js release-overhead.json > release-summary.json
```

The component probe was compiled with `rustc --edition=2021 -O -C lto=thin -C codegen-units=1`, `--extern kujo=<candidate release rlib>`, `--extern serde_json=<matching release rlib>`, and `-L dependency=<candidate target/release/deps>`. Freeze that probe before rebuilding another source into the shared target directory. Its binary hash is recorded.

## Integration and validation evidence

validation.json records the canonical Kujo, Watchdog and RunLedger gates and their explicit optional/advisory limits. integration/ contains the real measured closure/generator/async workload report, native-normalized batch, persisted repository/API records, and separate-process receipt. watchdog-gate.log and integration-gate.log retain the consumer results. debug-stack-reproduction.txt isolates a pre-existing dev-profile worker-stack failure; optimized default-stack integration passed. No Wave B or C implementation changed.
