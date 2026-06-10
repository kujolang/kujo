# V1VM-PERF-003 — Cache Lookup Validation For Repeated Nested Imports

Date: 2026-05-22
Owner: codex agent

## Scope

Validated that repeated nested dotted imports use cache-backed lookup behavior and documented measurable cold-vs-warm timing evidence.

## Code changes

- Added warm-cache benchmark target:
  - `module_resolution/import_heavy_nested_dotted_cached_lookup_warm_loader`
- Added module-loader cache reuse regression:
  - `load_module_reuses_cached_nested_dotted_module_without_duplicate_cache_entries`

## Benchmark command

```bash
cargo bench --bench v1_perf_benchmarks -- import_heavy_nested_dotted_ --noplot --sample-size 10 --warm-up-time 0.5 --measurement-time 1
```

## Result snapshot

- Cold median: `35.467 ms`
- Warm cached median: `251.22 µs`
- Warm lookup is approximately `141x` faster.

## Guardrails

- Artifact: `docs/generated/VM_IMPORT_HEAVY_CACHE_LOOKUP.md`
- Contract: `tests/vm_import_heavy_cache_lookup_contract.rs`
