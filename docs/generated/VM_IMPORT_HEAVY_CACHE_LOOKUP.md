# VM Import-Heavy Nested Lookup Cache Validation

Date: 2026-05-22
Benchmark group: `module_resolution`

## Benchmark command

```bash
cargo bench --bench v1_perf_benchmarks -- import_heavy_nested_dotted_ --noplot --sample-size 10 --warm-up-time 0.5 --measurement-time 1
```

## Targets

- Cold path: `module_resolution/import_heavy_nested_dotted_startup_cold_loader`
- Warm path: `module_resolution/import_heavy_nested_dotted_cached_lookup_warm_loader`

## Timing Summary

| Metric | Cold loader | Warm cached lookup |
| --- | ---: | ---: |
| time low | 34.003 ms | 247.11 µs |
| time median | 35.467 ms | 251.22 µs |
| time high | 37.130 ms | 254.42 µs |

## Cache Effect Interpretation

- Warm cached median (`251.22 µs`) is approximately `141x` faster than cold startup median (`35.467 ms`).
- Interpretation: repeated nested import lookups are cache-backed and avoid cold-path startup overhead.

## Regression Guard

- Unit regression: `src/module.rs::load_module_reuses_cached_nested_dotted_module_without_duplicate_cache_entries`
- Contract guard: `tests/vm_import_heavy_cache_lookup_contract.rs`
