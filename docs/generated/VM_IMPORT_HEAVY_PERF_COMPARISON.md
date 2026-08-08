# VM Import-Heavy Nested Startup Perf Comparison

Date: 2026-05-22
Benchmark target: `module_resolution/import_heavy_nested_dotted_startup_cold_loader`

## Commands

Baseline capture (`V1VM-PERF-001`):

```bash
cargo bench --bench v1_perf_benchmarks -- import_heavy_nested_dotted_startup_cold_loader --noplot --sample-size 10 --warm-up-time 0.5 --measurement-time 1
```

Current capture (`V1VM-PERF-002`):

```bash
cargo bench --bench v1_perf_benchmarks -- import_heavy_nested_dotted_startup_cold_loader --noplot --sample-size 10 --warm-up-time 0.5 --measurement-time 1
```

## Before/After Summary

| Metric | Baseline (2026-05-21) | Current (2026-05-22) |
| --- | ---: | ---: |
| time low | 278.40 ms | 40.030 ms |
| time median | 350.61 ms | 40.763 ms |
| time high | 420.40 ms | 41.547 ms |

## Tolerance Policy

- Unacceptable regression threshold: current median must not exceed baseline median by more than `20.0%`.
- Median delta formula: `(current_median - baseline_median) / baseline_median * 100`.

## Result

- Computed median delta: `-88.37%`.
- Interpretation: `PASS` (no unacceptable regression after reliability fixes).
